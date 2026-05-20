import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

suite("Task 00039 Phase A - CodeMirror foundation", () => {
    test("preview html assets now include the dedicated runtime module and import map", () => {
        const previewHtmlSource = fs.readFileSync(
            path.join(extensionRoot, "src", "document-viewer", "previewHtml.ts"),
            "utf-8",
        );
        assert.ok(previewHtmlSource.includes("chatInputRuntimeUri"));
        assert.ok(previewHtmlSource.includes("codeMirrorImportMap"));
        assert.ok(previewHtmlSource.includes('<script type="importmap" nonce="${nonce}">'));
        assert.ok(
            previewHtmlSource.includes('<meta name="vector-csp-nonce" content="${nonce}" />'),
        );
        assert.ok(
            previewHtmlSource.includes(
                '<script type="module" nonce="${nonce}" src="${assets.chatInputRuntimeUri}"></script>',
            ),
        );
    });

    test("the preview components expose node_modules assets to the webview and resolve CodeMirror entrypoints", () => {
        const assetsSource = fs.readFileSync(
            path.join(extensionRoot, "src", "document-viewer", "previewAssets.ts"),
            "utf-8",
        );
        const providerSource = fs.readFileSync(
            path.join(extensionRoot, "src", "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            providerSource.includes('vscode.Uri.joinPath(this.extensionUri, "node_modules")'),
        );
        assert.ok(assetsSource.includes('"@codemirror/state"'));
        assert.ok(assetsSource.includes('"@codemirror/view"'));
        assert.ok(assetsSource.includes('"@codemirror/autocomplete"'));
        assert.ok(assetsSource.includes('"@codemirror/commands"'));
        assert.ok(assetsSource.includes('"@codemirror/language"'));
        assert.ok(assetsSource.includes('"@marijn/find-cluster-break"'));
    });

    test("the dedicated runtime owns editor state and selection through EditorView", () => {
        const runtimeSource = fs.readFileSync(
            path.join(extensionRoot, "media", "chat-input-runtime.js"),
            "utf-8",
        );
        assert.ok(runtimeSource.includes("EditorSelection"));
        assert.ok(runtimeSource.includes("EditorState.create({"));
        assert.ok(runtimeSource.includes('meta[name="vector-csp-nonce"]'));
        assert.ok(runtimeSource.includes("EditorView.cspNonce.of(cspNonce)"));
        assert.ok(runtimeSource.includes("new EditorView({ state: state, parent: mountEl })"));
        assert.ok(runtimeSource.includes("instance.view.state.selection.main.head"));
        assert.ok(runtimeSource.includes("instance.view.state.doc.toString()"));
    });
});
