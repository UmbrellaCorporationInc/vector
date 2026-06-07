import * as fs from "fs";
import * as vscode from "vscode";
import {
    findGovernedWorkspaceRoot,
    loadDocumentTypes,
    resolveDocumentByCode,
    resolveCreateFormSource,
    PerTypeDocumentProvider,
    type ActiveFilter,
    type GovernedTreeNode,
} from "./governedDocumentProvider.js";
import {
    GovernedDocumentEditorProvider,
    GOVERNED_DOCUMENT_VIEW_TYPE,
    resolveGovernedPreviewSource,
    resolveGovernedPreviewSourceByIdentifier,
} from "./document-viewer/index.js";
import {
    cleanupAllTempFiles,
    writeTempDocument,
} from "./document-viewer/document-actions/agentExecutor.js";
import { substituteVariables } from "./document-viewer/document-actions/variableSubstitution.js";
import { DashboardViewerController } from "./dashboard-viewer/index.js";

export function activate(context: vscode.ExtensionContext): void {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        void vscode.commands.executeCommand("setContext", "vector.hasConfig", false);
        return;
    }

    const workspaceRoot = findGovernedWorkspaceRoot(
        workspaceFolders.map((folder) => folder.uri.fsPath),
    );
    if (!workspaceRoot) {
        void vscode.commands.executeCommand("setContext", "vector.hasConfig", false);
        return;
    }

    const hasConfig = loadDocumentTypes(workspaceRoot) !== null;
    void vscode.commands.executeCommand("setContext", "vector.hasConfig", hasConfig);

    // Phase D (RFC 00017): dashboard viewer controller.
    const dashboardController = new DashboardViewerController(workspaceRoot, context.extensionUri);

    // Phase C: per-type provider with filter state, scoped actions.
    const provider = new PerTypeDocumentProvider(workspaceRoot);

    const treeView = vscode.window.createTreeView("vector.governedDocuments", {
        treeDataProvider: provider,
        showCollapseAll: true,
    });
    provider.setTreeView(treeView);

    // --- Custom editor provider (RFC 00024 Phase A) ---
    const editorProvider = new GovernedDocumentEditorProvider(workspaceRoot, context.extensionUri);
    const customEditorRegistration = vscode.window.registerCustomEditorProvider(
        GOVERNED_DOCUMENT_VIEW_TYPE,
        editorProvider,
        {
            webviewOptions: { retainContextWhenHidden: false },
            supportsMultipleEditorsPerDocument: false,
        },
    );

    // --- Open Governed Preview (RFC 00024 Phase A) ---
    const openPreviewCmd = vscode.commands.registerCommand(
        "vector.openGovernedPreview",
        async (uri: vscode.Uri) => {
            await vscode.commands.executeCommand(
                "vscode.openWith",
                uri,
                GOVERNED_DOCUMENT_VIEW_TYPE,
            );
        },
    );

    const openStemCmd = vscode.commands.registerCommand("vector.openStem", async (stem: string) => {
        const source = resolveGovernedPreviewSourceByIdentifier(workspaceRoot, stem);
        if (!source) {
            void vscode.window.showErrorMessage(
                `Vector: cannot resolve governed document: ${stem}`,
            );
            return;
        }
        await vscode.commands.executeCommand(
            "vscode.openWith",
            vscode.Uri.file(source.doc.filePath),
            GOVERNED_DOCUMENT_VIEW_TYPE,
        );
    });

    // --- Activate (legacy) ---
    const activateCmd = vscode.commands.registerCommand("vector.activate", () => {
        vscode.window.showInformationMessage("Vector extension activated.");
    });

    // --- Refresh ---
    const refreshCmd = vscode.commands.registerCommand("vector.refreshGovernedDocuments", () => {
        const updated = loadDocumentTypes(workspaceRoot);
        void vscode.commands.executeCommand("setContext", "vector.hasConfig", updated !== null);
        provider.refresh();
    });

    // --- Search ---
    const searchCmd = vscode.commands.registerCommand("vector.searchInType", async () => {
        const config = loadDocumentTypes(workspaceRoot);
        if (!config) {
            return;
        }
        const docTypes = Object.keys(config["document-types"]);

        const selectedType = await vscode.window.showQuickPick(
            docTypes.map((t) => t.toUpperCase()),
            { placeHolder: "Select document type to search in" },
        );
        if (!selectedType) {
            return;
        }
        const docType = selectedType.toLowerCase();
        const tc = config["document-types"][docType];
        if (!tc) {
            return;
        }

        const raw = await vscode.window.showInputBox({
            prompt: `Enter ${selectedType} document code`,
            placeHolder: `e.g. ${"1".padStart(tc["code-width"], "0")}`,
            validateInput: (v) => (/^\d+$/.test(v.trim()) ? null : "Enter a numeric code"),
        });
        if (!raw) {
            return;
        }
        const code = raw.trim().padStart(tc["code-width"], "0");
        const doc = resolveDocumentByCode(workspaceRoot, docType, code);
        if (!doc) {
            void vscode.window.showErrorMessage(
                `No ${selectedType} document found with code ${code}.`,
            );
            return;
        }
        // Phase A keeps the tree lazy at the root level, so reveal the
        // owning tree path before opening the preview.
        provider.focusType(docType);
        const node = provider.getRevealTargetForDocument(doc);
        await treeView.reveal(node, { select: true, focus: true, expand: true });
        await vscode.commands.executeCommand(
            "vscode.openWith",
            vscode.Uri.file(doc.filePath),
            GOVERNED_DOCUMENT_VIEW_TYPE,
        );
    });

    // --- List (filter by status or category) ---
    const listCmd = vscode.commands.registerCommand("vector.listByFilter", async () => {
        const config = loadDocumentTypes(workspaceRoot);
        if (!config) {
            return;
        }
        const docTypes = Object.keys(config["document-types"]);

        const selectedType = await vscode.window.showQuickPick(
            docTypes.map((t) => t.toUpperCase()),
            { placeHolder: "Select document type to filter" },
        );
        if (!selectedType) {
            return;
        }
        const docType = selectedType.toLowerCase();
        const tc = config["document-types"][docType];
        if (!tc) {
            return;
        }

        const filterValues: string[] = ["All"];
        if (tc.layout === "status" && tc.statuses) {
            filterValues.push(...tc.statuses);
        } else if (tc.layout === "category") {
            filterValues.push(...provider.getCategoryOptions(docType));
        }

        const picked = await vscode.window.showQuickPick(filterValues, {
            placeHolder:
                tc.layout === "directory"
                    ? `No filters available for ${selectedType}`
                    : `Filter ${selectedType} documents`,
        });
        if (!picked || (tc.layout === "directory" && picked !== "All")) {
            return;
        }

        let filter: ActiveFilter;
        if (picked === "All") {
            filter = { kind: "all" };
        } else if (tc.layout === "status") {
            filter = { kind: "status", value: picked };
        } else {
            filter = { kind: "category", value: picked };
        }
        provider.applyFilter(docType, filter);
        const revealNode = provider.getRevealTargetForFilter(docType);
        await treeView.reveal(revealNode, { select: true, focus: false, expand: true });
    });

    // --- Clear All Filters ---
    const clearFiltersCmd = vscode.commands.registerCommand("vector.clearAllFilters", () => {
        provider.clearAllFilters();
    });

    // --- Phase G: Native editor/title preview actions ---
    const previewRefreshCmd = vscode.commands.registerCommand("vector.previewRefresh", () => {
        editorProvider.refresh();
    });

    const previewToggleTocCmd = vscode.commands.registerCommand("vector.previewToggleToc", () => {
        editorProvider.postToggleToc();
    });

    const previewOpenEditorCmd = vscode.commands.registerCommand("vector.previewOpenEditor", () => {
        editorProvider.openCurrentInEditor();
    });

    // --- Create Document (RFC 00020 Phase B) ---
    const createDocumentCmd = vscode.commands.registerCommand(
        "vector.createDocument",
        async (node: GovernedTreeNode | undefined) => {
            if (!node || node.kind !== "root") {
                void vscode.window.showErrorMessage(
                    "Select a document type folder to create a document.",
                );
                return;
            }
            const config = loadDocumentTypes(workspaceRoot);
            const typeConfig = config?.["document-types"][node.docType];
            const createFormId = typeConfig?.["create-document-form"];
            if (!createFormId) {
                void vscode.window.showErrorMessage(
                    `No create form configured for document type "${node.docType}".`,
                );
                return;
            }
            const result = resolveCreateFormSource(workspaceRoot, createFormId);
            if (!result.ok) {
                void vscode.window.showErrorMessage(
                    `Cannot open create form for "${node.docType}": ${result.reason}`,
                );
                return;
            }

            try {
                const sourceContent = fs.readFileSync(result.filePath, "utf-8");
                const substituted = substituteVariables(sourceContent, {
                    "document-type": node.docType,
                });
                const tempPath = writeTempDocument(substituted);
                await vscode.commands.executeCommand(
                    "vscode.openWith",
                    vscode.Uri.file(tempPath),
                    GOVERNED_DOCUMENT_VIEW_TYPE,
                );
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                void vscode.window.showErrorMessage(
                    `Failed to instantiate create form: ${message}`,
                );
            }
        },
    );

    // --- Create Document Type (RFC 00020 Phase B) ---
    const createDocumentTypeCmd = vscode.commands.registerCommand(
        "vector.createDocumentType",
        async () => {
            const config = loadDocumentTypes(workspaceRoot);
            const createTypeFormId = config?.["doc-type"]?.["create-document-type-form"];
            if (!createTypeFormId) {
                void vscode.window.showErrorMessage("No create document type form configured.");
                return;
            }
            const result = resolveCreateFormSource(workspaceRoot, createTypeFormId);
            if (!result.ok) {
                void vscode.window.showErrorMessage(
                    `Cannot open create document type form: ${result.reason}`,
                );
                return;
            }
            await vscode.commands.executeCommand(
                "vscode.openWith",
                vscode.Uri.file(result.filePath),
                GOVERNED_DOCUMENT_VIEW_TYPE,
            );
        },
    );
    // --- Package Sync (RFC 00030 Phase B) ---
    const packageSyncCmd = vscode.commands.registerCommand("vector.packageSync", () => {
        const terminal = vscode.window.createTerminal("vector sync");
        terminal.sendText("vector-database package sync");
        terminal.show();
    });

    // --- Validate Fix (RFC 00024 Phase C) ---
    const validateFixCmd = vscode.commands.registerCommand("vector.validateFix", async () => {
        const config = loadDocumentTypes(workspaceRoot);
        if (!config) {
            void vscode.window.showErrorMessage(
                "Vector: cannot load .vector/document-types.yaml — validate-fix unavailable.",
            );
            return;
        }
        const promptStem = config["doc-type"]?.["prompt-validate-fix"];
        if (!promptStem) {
            void vscode.window.showErrorMessage(
                "Vector: doc-type.prompt-validate-fix is not configured in .vector/document-types.yaml — add it to enable the validate-fix action.",
            );
            return;
        }
        const source = resolveGovernedPreviewSource(workspaceRoot, promptStem);
        if (!source) {
            void vscode.window.showErrorMessage(
                `Vector: cannot resolve validate-fix prompt document: ${promptStem}`,
            );
            return;
        }
        await vscode.commands.executeCommand(
            "vscode.openWith",
            vscode.Uri.file(source.doc.filePath),
            GOVERNED_DOCUMENT_VIEW_TYPE,
        );
    });

    // --- Open Dashboard (RFC 00017 Phase B) ---
    const openDashboardCmd = vscode.commands.registerCommand(
        "vector.openDashboard",
        (uri: vscode.Uri) => {
            dashboardController.openDashboard(uri);
        },
    );

    const dashboardRefreshCmd = vscode.commands.registerCommand("vector.dashboardRefresh", () => {
        dashboardController.refresh();
    });

    context.subscriptions.push(
        customEditorRegistration,
        dashboardController,
        treeView,
        openPreviewCmd,
        openStemCmd,
        activateCmd,
        refreshCmd,
        searchCmd,
        listCmd,
        clearFiltersCmd,
        previewRefreshCmd,
        previewToggleTocCmd,
        previewOpenEditorCmd,
        openDashboardCmd,
        dashboardRefreshCmd,
        createDocumentCmd,
        createDocumentTypeCmd,
        packageSyncCmd,
        validateFixCmd,
    );
}

export function deactivate(): void {
    cleanupAllTempFiles();
}
