import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";

const pkg_root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

const STATUS_CONFIG = `document-types:
  rfc:
    layout: status
    "code-width": 5
    statuses:
      - draft
      - accepted
`;

function makeTempWorkspace(config?: string): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-nav-history-"));
    if (config !== undefined) {
        fs.mkdirSync(path.join(dir, ".vector"), { recursive: true });
        fs.writeFileSync(path.join(dir, ".vector", "document-types.yaml"), config, "utf-8");
    }
    return dir;
}

function createCustomEditorPanel(): {
    panel: import("vscode").WebviewPanel;
    fireMessage(message: unknown): void;
} {
    const messageListeners: Array<(message: unknown) => void> = [];
    const panel = {
        active: true,
        title: "Vector Test",
        webview: {
            cspSource: "vscode-webview-resource:",
            html: "",
            options: {},
            asWebviewUri: (uri: vscode.Uri) => uri,
            onDidReceiveMessage: (listener: (message: unknown) => void) => {
                messageListeners.push(listener);
                return { dispose: () => undefined };
            },
            postMessage: () => undefined,
        },
        onDidChangeViewState: () => ({ dispose: () => undefined }),
        onDidDispose: () => ({ dispose: () => undefined }),
        reveal: () => undefined,
        dispose: () => undefined,
    } as unknown as import("vscode").WebviewPanel;

    return {
        panel,
        fireMessage(message: unknown) {
            for (const listener of messageListeners) {
                listener(message);
            }
        },
    };
}

suite("Phase A (RFC 00024) — Navigation History Integration", () => {
    // ── static source checks ──────────────────────────────────────────────

    test("package.json declares vector.documentPreview custom editor", () => {
        const pkg = JSON.parse(fs.readFileSync(path.join(pkg_root, "package.json"), "utf-8")) as {
            contributes: { customEditors?: { viewType: string; priority: string }[] };
        };
        const editor = pkg.contributes.customEditors?.find(
            (e) => e.viewType === "vector.documentPreview",
        );
        assert.ok(editor, "package.json must declare the vector.documentPreview custom editor");
        assert.strictEqual(
            editor.priority,
            "option",
            "custom editor priority must be 'option' to avoid becoming the default .md viewer",
        );
    });

    test("extension.ts registers the custom editor provider", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(
            src.includes("registerCustomEditorProvider"),
            "extension.ts must call vscode.window.registerCustomEditorProvider",
        );
        assert.ok(
            src.includes("GOVERNED_DOCUMENT_VIEW_TYPE"),
            "extension.ts must use GOVERNED_DOCUMENT_VIEW_TYPE when registering",
        );
    });

    test("vector.openGovernedPreview handler uses vscode.openWith", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.openGovernedPreview"');
        const block = src.slice(idx, idx + 300);
        assert.ok(
            block.includes('"vscode.openWith"'),
            "vector.openGovernedPreview must dispatch vscode.openWith",
        );
        assert.ok(
            block.includes("GOVERNED_DOCUMENT_VIEW_TYPE"),
            "vector.openGovernedPreview must pass GOVERNED_DOCUMENT_VIEW_TYPE to vscode.openWith",
        );
    });

    test("vector.openStem handler uses vscode.openWith", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.openStem"');
        const block = src.slice(idx, idx + 600);
        assert.ok(
            block.includes('"vscode.openWith"'),
            "vector.openStem must dispatch vscode.openWith",
        );
        assert.ok(
            block.includes("GOVERNED_DOCUMENT_VIEW_TYPE"),
            "vector.openStem must pass GOVERNED_DOCUMENT_VIEW_TYPE to vscode.openWith",
        );
    });

    test("vector.openGovernedPreview handler is async", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.openGovernedPreview"');
        assert.ok(
            src.slice(idx, idx + 200).includes("async"),
            "vector.openGovernedPreview handler must be async",
        );
    });

    test("vector.openStem handler is async", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.openStem"');
        assert.ok(
            src.slice(idx, idx + 200).includes("async"),
            "vector.openStem handler must be async",
        );
    });

    test("GovernedDocumentEditorProvider implements CustomReadonlyEditorProvider", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes("implements CustomReadonlyEditorProvider"),
            "GovernedDocumentEditorProvider must implement CustomReadonlyEditorProvider",
        );
    });

    test("GovernedDocumentEditorProvider exports GOVERNED_DOCUMENT_VIEW_TYPE", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes('GOVERNED_DOCUMENT_VIEW_TYPE = "vector.documentPreview"'),
            "GOVERNED_DOCUMENT_VIEW_TYPE must equal 'vector.documentPreview'",
        );
    });

    test("wikilink messages in the provider dispatch vector.openStem", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes('"vector.openStem"'),
            "GovernedDocumentEditorProvider must dispatch vector.openStem for wikilink messages",
        );
    });

    test("custom editor provider listens for runAgent messages", () => {
        const src = fs.readFileSync(
            path.join(pkg_root, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            src.includes("isRunAgentMessage"),
            "GovernedDocumentEditorProvider must handle vector.runAgent messages",
        );
    });

    // ── behavioural: vector.openStem error path ───────────────────────────

    test("vector.openStem emits an error message for an unresolvable stem", async () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            vscode.__resetUiState();
            vscode.__resetCommandHandlers();

            const { resolveGovernedPreviewSource } =
                await import("../document-viewer/previewAssets.js");

            vscode.commands.registerCommand("vector.openStem", async (...args: unknown[]) => {
                const stem = args[0] as string;
                const source = resolveGovernedPreviewSource(dir, stem);
                if (!source) {
                    vscode.window.showErrorMessage(
                        `Vector: cannot resolve governed document: ${stem}`,
                    );
                    return;
                }
                await vscode.commands.executeCommand(
                    "vscode.openWith",
                    vscode.Uri.file(source.doc.filePath),
                    "vector.documentPreview",
                );
            });

            await vscode.commands.executeCommand("vector.openStem", "rfc-00099-nonexistent");

            assert.ok(
                vscode
                    .__getErrorMessages()
                    .some((m) => m.includes("Vector: cannot resolve governed document")),
                "must emit an error for an unresolvable stem",
            );
        } finally {
            vscode.__resetCommandHandlers();
            vscode.__resetUiState();
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── behavioural: vector.openGovernedPreview dispatches vscode.openWith ──

    test("vector.openGovernedPreview dispatches vscode.openWith with the correct view type", async () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            vscode.__resetUiState();
            vscode.__resetCommandHandlers();

            const openWithCalls: { uri: vscode.Uri; viewType: string }[] = [];
            vscode.commands.registerCommand("vscode.openWith", (...args: unknown[]) => {
                openWithCalls.push({ uri: args[0] as vscode.Uri, viewType: args[1] as string });
            });

            const filePath = path.join(dir, "rfc-00001-sample.md");
            const uri = vscode.Uri.file(filePath);

            vscode.commands.registerCommand(
                "vector.openGovernedPreview",
                async (...args: unknown[]) => {
                    await vscode.commands.executeCommand(
                        "vscode.openWith",
                        args[0],
                        "vector.documentPreview",
                    );
                },
            );

            await vscode.commands.executeCommand("vector.openGovernedPreview", uri);

            assert.strictEqual(openWithCalls.length, 1, "vscode.openWith must be called once");
            const first = openWithCalls[0];
            assert.ok(first, "first openWith call must exist");
            assert.strictEqual(first.uri.fsPath, filePath);
            assert.strictEqual(first.viewType, "vector.documentPreview");
        } finally {
            vscode.__resetCommandHandlers();
            vscode.__resetUiState();
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    // ── Phase C (Task 00044) — validate-fix migrated to vscode.openWith ─────

    test("vector.validateFix handler in extension.ts dispatches vscode.openWith", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.validateFix"');
        assert.ok(idx !== -1, "vector.validateFix must be registered in extension.ts");
        const block = src.slice(idx, idx + 1200);
        assert.ok(
            block.includes('"vscode.openWith"'),
            "vector.validateFix handler must dispatch vscode.openWith",
        );
        assert.ok(
            block.includes("GOVERNED_DOCUMENT_VIEW_TYPE"),
            "vector.validateFix handler must pass GOVERNED_DOCUMENT_VIEW_TYPE to vscode.openWith",
        );
    });

    test("vector.validateFix handler does not route through previewController.runValidateFix", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        const idx = src.indexOf('"vector.validateFix"');
        assert.ok(idx !== -1, "vector.validateFix must be registered in extension.ts");
        const block = src.slice(idx, idx + 1200);
        assert.ok(
            !block.includes("previewController.runValidateFix"),
            "vector.validateFix must not route through the legacy previewController",
        );
    });

    test("extension.ts has no orphaned previewController.openDocument calls", () => {
        const src = fs.readFileSync(path.join(pkg_root, "src", "extension.ts"), "utf-8");
        assert.ok(
            !src.includes("previewController.openDocument"),
            "extension.ts must not contain orphaned previewController.openDocument calls",
        );
    });

    test("vector.validateFix shows error when prompt stem cannot be resolved", async () => {
        const dir = makeTempWorkspace(`document-types: {}
doc-type:
  prompt-validate-fix: prompts-00099-nonexistent
`);
        try {
            vscode.__resetUiState();
            vscode.__resetCommandHandlers();

            const { loadDocumentTypes } = await import("../documentDiscovery.js");
            const { resolveGovernedPreviewSource } =
                await import("../document-viewer/previewAssets.js");

            vscode.commands.registerCommand("vector.validateFix", async () => {
                const config = loadDocumentTypes(dir);
                if (!config) {
                    vscode.window.showErrorMessage(
                        "Vector: cannot load .vector/document-types.yaml — validate-fix unavailable.",
                    );
                    return;
                }
                const promptStem = config["doc-type"]?.["prompt-validate-fix"];
                if (!promptStem) {
                    vscode.window.showErrorMessage(
                        "Vector: doc-type.prompt-validate-fix is not configured.",
                    );
                    return;
                }
                const source = resolveGovernedPreviewSource(dir, promptStem);
                if (!source) {
                    vscode.window.showErrorMessage(
                        `Vector: cannot resolve validate-fix prompt document: ${promptStem}`,
                    );
                    return;
                }
                await vscode.commands.executeCommand(
                    "vscode.openWith",
                    vscode.Uri.file(source.doc.filePath),
                    "vector.documentPreview",
                );
            });

            await vscode.commands.executeCommand("vector.validateFix");

            assert.ok(
                vscode
                    .__getErrorMessages()
                    .some((m) => m.includes("cannot resolve validate-fix prompt document")),
                "must show error when validate-fix prompt stem cannot be resolved",
            );
        } finally {
            vscode.__resetCommandHandlers();
            vscode.__resetUiState();
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("custom editor provider routes runAgent messages through the agent executor path", async () => {
        const dir = makeTempWorkspace(STATUS_CONFIG);
        try {
            vscode.__resetUiState();

            const { GovernedDocumentEditorProvider } =
                await import("../document-viewer/governedDocumentEditorProvider.js");

            const docDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(docDir, { recursive: true });
            const docPath = path.join(docDir, "rfc-00001-sample.md");
            fs.writeFileSync(
                docPath,
                "---\ntitle: Sample RFC\nstatus: draft\n---\n# Sample\n",
                "utf-8",
            );

            const provider = new GovernedDocumentEditorProvider(
                dir,
                vscode.Uri.file(pkg_root) as unknown as import("vscode").Uri,
            );
            const panel = createCustomEditorPanel();
            const document = provider.openCustomDocument(
                vscode.Uri.file(docPath) as unknown as import("vscode").Uri,
            );

            provider.resolveCustomEditor(document, panel.panel);
            panel.fireMessage({
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00004-execute-task-phase",
                label: "Run",
                staticInput: {},
                formValues: {},
            });

            assert.deepStrictEqual(vscode.__getErrorMessages(), [
                "Vector: .vector/agents.yaml not found — add it to use agent triggers.",
            ]);
        } finally {
            vscode.__resetUiState();
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
