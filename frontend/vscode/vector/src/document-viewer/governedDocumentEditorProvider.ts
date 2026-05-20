import * as vscode from "vscode";
import type { CustomReadonlyEditorProvider } from "vscode";
import { loadDocumentTypes } from "../documentDiscovery.js";
import type { GovernedDocument } from "../documentDiscovery.js";
import { parseGovernedStem, isWikilinkMessage, isFmLinkMessage } from "./wikilinkNavigation.js";
import { buildPreviewHtml, escapeHtml } from "./previewHtml.js";
import { splitFrontmatter, renderFrontmatterPanel } from "./frontmatterRenderer.js";
import { renderGovernedMarkdownAnalysis } from "./markdownRenderer.js";
import {
    readGovernedDocumentContent,
    buildPreviewAssets,
    resolveGovernedPreviewSource,
} from "./previewAssets.js";
import { changeGovernedDocumentStatus } from "./documentStatus.js";
import {
    substituteVariables,
    findUnresolvedVariables,
} from "./document-actions/variableSubstitution.js";
import { loadAgentsConfig, resolveProfile } from "./document-actions/agentsConfig.js";
import {
    writeTempPrompt,
    spawnAgentTerminal,
    deleteTempFile,
} from "./document-actions/agentExecutor.js";
import {
    isFileSuggestionsRequest,
    FILE_SUGGESTIONS_RESULT_TYPE,
    isRenderFormBlockRequest,
    RENDER_FORM_BLOCK_RESULT_TYPE,
} from "./chat-input/chatInputMessaging.js";
import type {
    FileSuggestionsRequest,
    RenderFormBlockRequest,
} from "./chat-input/chatInputMessaging.js";
import type { ChatInputMention } from "./chat-input/chatInputTypes.js";
import { resolveFileSuggestions } from "./chat-input/chatInputSuggestionProvider.js";
import { renderFormBlock } from "./form-editor/formRenderer.js";

export const GOVERNED_DOCUMENT_VIEW_TYPE = "vector.documentPreview" as const;

/**
 * Custom editor provider for governed documents.
 *
 * Opens governed Markdown files inside a governed preview webview using the
 * VSCode custom editor API. Because VSCode routes opens through its native
 * editor pipeline, every open registers in the navigation history stack and
 * the Go Back / Go Forward buttons work correctly.
 */
export class GovernedDocumentEditorProvider implements CustomReadonlyEditorProvider {
    private readonly workspaceRoot: string;
    private readonly extensionUri: vscode.Uri;
    private _activePanel: vscode.WebviewPanel | undefined;
    private _activeUri: vscode.Uri | undefined;
    private _activeDoc: GovernedDocument | undefined;
    private readonly subscriptions: vscode.Disposable[] = [];

    constructor(workspaceRoot: string, extensionUri: vscode.Uri) {
        this.workspaceRoot = workspaceRoot;
        this.extensionUri = extensionUri;
    }

    openCustomDocument(uri: vscode.Uri): vscode.CustomDocument {
        return { uri, dispose: () => {} };
    }

    resolveCustomEditor(document: vscode.CustomDocument, webviewPanel: vscode.WebviewPanel): void {
        webviewPanel.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(this.extensionUri, "media"),
                vscode.Uri.joinPath(this.extensionUri, "node_modules"),
            ],
        };

        this._renderUri(document.uri, webviewPanel);

        webviewPanel.onDidChangeViewState(() => {
            if (webviewPanel.active) {
                this._activePanel = webviewPanel;
                this._activeUri = document.uri;
                this._activeDoc = buildStubDocument(document.uri);
            } else if (this._activePanel === webviewPanel) {
                this._activePanel = undefined;
                this._activeUri = undefined;
                this._activeDoc = undefined;
            }
        });

        webviewPanel.onDidDispose(() => {
            if (this._activePanel === webviewPanel) {
                this._activePanel = undefined;
                this._activeUri = undefined;
                this._activeDoc = undefined;
            }
        });

        if (webviewPanel.active) {
            this._activePanel = webviewPanel;
            this._activeUri = document.uri;
            this._activeDoc = buildStubDocument(document.uri);
        }

        webviewPanel.webview.onDidReceiveMessage((msg: unknown) => {
            if (isWikilinkMessage(msg) || isFmLinkMessage(msg)) {
                void vscode.commands.executeCommand("vector.openStem", msg.stem);
                return;
            }
            if (isOpenDocMessage(msg)) {
                const source = resolveGovernedPreviewSource(this.workspaceRoot, msg.doc);
                if (!source) {
                    void vscode.window.showErrorMessage(
                        `Vector: cannot resolve document: ${msg.doc}`,
                    );
                    return;
                }
                this._renderDocument(source.doc, source.content, webviewPanel, msg.input);
                return;
            }
            if (isFileSuggestionsRequest(msg)) {
                void this._handleFileSuggestionsRequest(webviewPanel, msg);
                return;
            }
            if (isRenderFormBlockRequest(msg)) {
                this._handleRenderFormBlockRequest(webviewPanel, msg);
                return;
            }
            if (isRunAgentMessage(msg)) {
                void this._handleRunAgent(msg);
                return;
            }
            if (isOpenEditorMessage(msg)) {
                void this._openActiveInEditor();
                return;
            }
            if (isChangeStatusMessage(msg)) {
                void this._changeCurrentDocumentStatus(msg.status, webviewPanel);
            }
        });
    }

    refresh(): void {
        if (this._activePanel && this._activeUri) {
            this._renderUri(this._activeUri, this._activePanel);
        }
    }

    postToggleToc(): void {
        this._activePanel?.webview.postMessage({ type: "vector.toggleToc" });
    }

    openCurrentInEditor(): void {
        void this._openActiveInEditor();
    }

    private async _openActiveInEditor(): Promise<void> {
        if (!this._activeUri) {
            return;
        }
        const document = await vscode.workspace.openTextDocument(this._activeUri);
        await vscode.window.showTextDocument(document, { preview: false });
    }

    private _renderUri(uri: vscode.Uri, panel: vscode.WebviewPanel): void {
        const content = readGovernedDocumentContent(uri.fsPath);
        if (content === null) {
            panel.webview.html = buildSimpleErrorHtml("Cannot read governed document.");
            return;
        }

        const assets = buildPreviewAssets(this.extensionUri, panel.webview);
        if (assets === null) {
            panel.webview.html = buildSimpleErrorHtml("Preview resources are unavailable.");
            return;
        }

        this._renderDocument(buildStubDocument(uri), content, panel, undefined, assets);
    }

    private _renderDocument(
        doc: GovernedDocument,
        content: string,
        panel: vscode.WebviewPanel,
        variables?: Record<string, string>,
        assets = buildPreviewAssets(this.extensionUri, panel.webview),
    ): void {
        if (assets === null) {
            panel.webview.html = buildSimpleErrorHtml("Preview resources are unavailable.");
            return;
        }

        const { fields, body } = splitFrontmatter(content);
        const statusEditor = resolveStatusEditor(this.workspaceRoot, doc.type, fields);
        const substitutedBody =
            variables && Object.keys(variables).length > 0
                ? substituteVariables(body, variables)
                : body;
        const frontmatterHtml = renderFrontmatterPanel(fields, statusEditor);
        const stemValue =
            doc.type && doc.code && doc.slug ? `${doc.type}-${doc.code}-${doc.slug}` : undefined;
        const markdown = renderGovernedMarkdownAnalysis(
            substitutedBody,
            stemValue !== undefined ? { documentStem: stemValue } : undefined,
        );

        this._activePanel = panel;
        this._activeUri = vscode.Uri.file(doc.filePath);
        this._activeDoc = doc;
        panel.title = doc.title;
        panel.webview.html = buildPreviewHtml(
            panel.webview,
            doc.title,
            markdown.html,
            assets,
            frontmatterHtml,
            { headings: markdown.headings },
        );
    }

    private async _handleFileSuggestionsRequest(
        panel: vscode.WebviewPanel,
        msg: FileSuggestionsRequest,
    ): Promise<void> {
        const suggestions = await resolveFileSuggestions(this.workspaceRoot, msg.query);
        panel.webview.postMessage({
            type: FILE_SUGGESTIONS_RESULT_TYPE,
            requestId: msg.requestId,
            suggestions,
        });
    }

    private _handleRenderFormBlockRequest(
        panel: vscode.WebviewPanel,
        msg: RenderFormBlockRequest,
    ): void {
        const html = renderFormBlock(msg.content);
        panel.webview.postMessage({
            type: RENDER_FORM_BLOCK_RESULT_TYPE,
            requestId: msg.requestId,
            html,
        });
    }

    private async _handleRunAgent(msg: RunAgentMessage): Promise<void> {
        const agentsLoad = loadAgentsConfig(this.workspaceRoot);
        if (!agentsLoad.ok) {
            if (!agentsLoad.missing) {
                void vscode.window.showErrorMessage(`Vector: ${agentsLoad.error}`);
            } else {
                void vscode.window.showErrorMessage(
                    "Vector: .vector/agents.yaml not found — add it to use agent triggers.",
                );
            }
            return;
        }

        const allAgents = resolveProfile(agentsLoad.config, msg.profile);
        if (allAgents.length === 0) {
            void vscode.window.showErrorMessage(
                `Vector: profile '${msg.profile}' not found in .vector/agents.yaml`,
            );
            return;
        }

        const available = allAgents.filter((agent) => agent.available);
        const unavailableNames = allAgents
            .filter((agent) => !agent.available)
            .map((agent) => agent.name)
            .join(", ");

        if (available.length === 0) {
            void vscode.window.showErrorMessage(
                `Vector: no agents in profile '${msg.profile}' are installed` +
                    (unavailableNames ? ` (not in PATH: ${unavailableNames})` : ""),
            );
            return;
        }

        let chosenAgent = available[0];
        if (available.length > 1) {
            const items = [
                ...available.map((agent) => ({
                    label: agent.name,
                    description: agent.command,
                    agent,
                })),
                ...allAgents
                    .filter((agent) => !agent.available)
                    .map((agent) => ({
                        label: agent.name,
                        description: `${agent.command} (not installed)`,
                        agent,
                    })),
            ];
            const picked = await vscode.window.showQuickPick(items, {
                placeHolder: `Select agent for "${msg.label}"`,
            });
            if (!picked || !picked.agent.available) {
                return;
            }
            chosenAgent = picked.agent;
        }

        if (!chosenAgent) {
            return;
        }

        const promptSource = resolveGovernedPreviewSource(this.workspaceRoot, msg.prompt);
        if (!promptSource) {
            void vscode.window.showErrorMessage(
                `Vector: cannot resolve prompt document '${msg.prompt}'`,
            );
            return;
        }

        const mergedVars: Record<string, string> = { ...msg.staticInput, ...msg.formValues };
        const unresolved = findUnresolvedVariables(promptSource.content, mergedVars);
        if (unresolved.length > 0) {
            void vscode.window.showWarningMessage(
                `Vector: prompt has unresolved variables: ${unresolved.join(", ")}`,
            );
        }

        const resolvedPrompt = substituteVariables(promptSource.content, mergedVars);
        const tempFilePath = writeTempPrompt(resolvedPrompt);
        try {
            spawnAgentTerminal(
                chosenAgent.command,
                chosenAgent.name,
                msg.label,
                tempFilePath,
                this.subscriptions,
            );
        } catch (error) {
            deleteTempFile(tempFilePath);
            const message = error instanceof Error ? error.message : String(error);
            void vscode.window.showErrorMessage(message);
        }
    }

    private async _changeCurrentDocumentStatus(
        nextStatus: string,
        panel: vscode.WebviewPanel,
    ): Promise<void> {
        if (!this._activeDoc) {
            return;
        }

        const config = loadDocumentTypes(this.workspaceRoot);
        const typeConfig = config?.["document-types"][this._activeDoc.type];
        const allowedStatuses = typeConfig?.statuses ?? [];
        if (!typeConfig || typeConfig.layout !== "status") {
            return;
        }

        try {
            const result = changeGovernedDocumentStatus(
                this.workspaceRoot,
                this._activeDoc,
                nextStatus,
                allowedStatuses,
            );
            if (!result.changed) {
                return;
            }

            const nextDoc: GovernedDocument = {
                ...this._activeDoc,
                filePath: result.filePath,
                status: nextStatus,
            };
            this._activeDoc = nextDoc;
            this._activeUri = vscode.Uri.file(result.filePath);
            await vscode.commands.executeCommand("vector.refreshGovernedDocuments");
            this._renderDocument(nextDoc, result.content, panel);
        } catch (error) {
            const message = error instanceof Error ? error.message : String(error);
            void vscode.window.showErrorMessage(message);
        }
    }
}

function resolveStatusEditor(
    workspaceRoot: string,
    docType: string,
    fields: Record<string, unknown>,
): { current: string; options: readonly string[] } | undefined {
    const config = loadDocumentTypes(workspaceRoot);
    const typeConfig = config?.["document-types"][docType];
    const currentStatus = typeof fields.status === "string" ? fields.status : undefined;
    if (!typeConfig || typeConfig.layout !== "status" || !currentStatus) {
        return undefined;
    }
    return { current: currentStatus, options: typeConfig.statuses ?? [] };
}

interface OpenEditorMessage {
    type: "vector.openEditor";
}

interface RunAgentMessage {
    type: "vector.runAgent";
    profile: string;
    prompt: string;
    label: string;
    staticInput: Record<string, string>;
    formValues: Record<string, string>;
    chatInputMentions?: Record<string, ChatInputMention[]>;
}

interface OpenDocMessage {
    type: "vector.openDoc";
    doc: string;
    input: Record<string, string>;
}

interface ChangeStatusMessage {
    type: "vector.changeStatus";
    status: string;
}

function isRunAgentMessage(message: unknown): message is RunAgentMessage {
    return (
        isRecord(message) &&
        message.type === "vector.runAgent" &&
        typeof message.profile === "string" &&
        typeof message.prompt === "string" &&
        typeof message.label === "string" &&
        isRecord(message.staticInput) &&
        isRecord(message.formValues)
    );
}

function isOpenDocMessage(message: unknown): message is OpenDocMessage {
    return (
        isRecord(message) &&
        message.type === "vector.openDoc" &&
        typeof message.doc === "string" &&
        isRecord(message.input)
    );
}

function isOpenEditorMessage(msg: unknown): msg is OpenEditorMessage {
    return (
        typeof msg === "object" &&
        msg !== null &&
        (msg as Record<string, unknown>)["type"] === "vector.openEditor"
    );
}

function isChangeStatusMessage(message: unknown): message is ChangeStatusMessage {
    return (
        isRecord(message) &&
        message.type === "vector.changeStatus" &&
        typeof message.status === "string"
    );
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
}

function buildStubDocument(uri: vscode.Uri): GovernedDocument {
    const fileName = uri.fsPath.replace(/\\/g, "/").split("/").pop() ?? "";
    const stem = fileName.replace(/\.md$/, "");
    const parsed = parseGovernedStem(stem);
    return {
        type: parsed?.type ?? "doc",
        code: parsed?.code ?? "",
        slug: parsed?.slug ?? stem,
        title: parsed ? `${parsed.type.toUpperCase()} ${parsed.code}` : stem,
        filePath: uri.fsPath,
    };
}

function buildSimpleErrorHtml(message: string): string {
    return `<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"/></head><body><p>${escapeHtml(message)}</p></body></html>`;
}
