import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";
import { activate } from "../extension.js";

function makeTempWorkspace(): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-create-flows-test-"));
    fs.mkdirSync(path.join(dir, ".vector"), { recursive: true });
    return dir;
}

function writeFile(filePath: string, content = ""): void {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, content, "utf-8");
}

suite("Phase C — Open Create Forms Through the Existing Viewer", () => {
    let workspaceRoot: string;
    let context: vscode.ExtensionContext;
    const pkgRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

    setup(() => {
        vscode.__resetCommandHandlers();
        vscode.__resetUiState();
        workspaceRoot = makeTempWorkspace();
        // Set up vscode.workspace.workspaceFolders
        vscode.workspace.workspaceFolders = [
            {
                uri: vscode.Uri.file(workspaceRoot),
                name: "test",
                index: 0,
            },
        ];

        context = {
            subscriptions: [],
            extensionUri: vscode.Uri.file(pkgRoot),
        };
    });

    teardown(() => {
        fs.rmSync(workspaceRoot, { recursive: true, force: true });
        vscode.workspace.workspaceFolders = undefined;
    });

    test("vector.createDocument substitutes #{document-type} and opens temp file", async () => {
        const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001-create-rfc
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        const formPath = path.join(workspaceRoot, "doc", "form", "form-00001-create-rfc.md");
        writeFile(formPath, "---\ntitle: Create RFC\n---\n# New #{document-type}");

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler, "vector.createDocument command should be registered");

        await handler({ kind: "root", docType: "rfc" });

        const panels = vscode.__getCreatedPanels();
        assert.strictEqual(panels.length, 1, "Should have created 1 preview panel");
        const panel = panels[0];
        assert.ok(panel);

        assert.ok(
            panel.title.startsWith("vector_temp_"),
            `Title should be the temp file name, got ${panel.title}`,
        );

        assert.ok(
            panel.panel.webview.html.includes("New rfc"),
            "HTML should contain substituted 'rfc'",
        );
        assert.ok(
            !panel.panel.webview.html.includes("#{document-type}"),
            "HTML should not contain placeholder",
        );
    });

    test("vector.createDocumentType opens source form directly without substitution", async () => {
        const config = `document-types: {}
doc-type:
  create-document-type-form: form-00002-create-type
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        const formPath = path.join(workspaceRoot, "doc", "form", "form-00002-create-type.md");
        writeFile(formPath, "---\ntitle: Create Type\n---\n# Global Form #{document-type}");

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocumentType");
        assert.ok(handler, "vector.createDocumentType command should be registered");

        await handler();

        const panels = vscode.__getCreatedPanels();
        assert.strictEqual(panels.length, 1, "Should have created 1 preview panel");
        const panel = panels[0];
        assert.ok(panel);

        // For non-temp files, it parses the stem: form-00002 -> FORM 00002
        assert.strictEqual(panel.title, "FORM 00002");

        assert.ok(
            panel.panel.webview.html.includes("Global Form #{document-type}"),
            "HTML should contain placeholder unchanged",
        );
    });

    test("vector.createDocument shows error when no form is configured", async () => {
        const config = `document-types:
  task:
    layout: status
    code-width: 5
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler);
        await handler({ kind: "root", docType: "task" });

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("No create form configured")),
            "Should show configuration error",
        );
    });

    test("vector.createDocument shows error when source file is missing", async () => {
        const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00099-missing
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler);
        await handler({ kind: "root", docType: "rfc" });

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("not found")),
            "Should show resolution error",
        );
    });

    test("vector.createDocument shows error when create-form is ambiguous", async () => {
        const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001-create-rfc
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        // Create two files with the same name in different subdirectories
        writeFile(
            path.join(workspaceRoot, "doc", "form", "draft", "form-00001-create-rfc.md"),
            "---\ntitle: Draft\n---\n",
        );
        writeFile(
            path.join(workspaceRoot, "doc", "form", "published", "form-00001-create-rfc.md"),
            "---\ntitle: Published\n---\n",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler);
        await handler({ kind: "root", docType: "rfc" });

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("ambiguous")),
            "Should show ambiguous resolution error",
        );
    });

    test("vector.createDocumentType shows error when no form is configured", async () => {
        const config = `document-types: {}
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocumentType");
        assert.ok(handler);
        await handler();

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("No create document type form configured")),
            "Should show configuration error",
        );
    });

    test("vector.createDocumentType shows error when source file is missing", async () => {
        const config = `document-types: {}
doc-type:
  create-document-type-form: form-00099-missing
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocumentType");
        assert.ok(handler);
        await handler();

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("not found")),
            "Should show resolution error",
        );
    });

    test("vector.createDocumentType shows error when create-form is ambiguous", async () => {
        const config = `document-types: {}
doc-type:
  create-document-type-form: form-00002-create-type
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        // Create two files with the same name in different subdirectories
        writeFile(
            path.join(workspaceRoot, "doc", "form", "v1", "form-00002-create-type.md"),
            "---\ntitle: V1\n---\n",
        );
        writeFile(
            path.join(workspaceRoot, "doc", "form", "v2", "form-00002-create-type.md"),
            "---\ntitle: V2\n---\n",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocumentType");
        assert.ok(handler);
        await handler();

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("ambiguous")),
            "Should show ambiguous resolution error",
        );
    });

    test("vector.createDocument shows error when invoked without a tree node", async () => {
        const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001-create-rfc
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        const formPath = path.join(workspaceRoot, "doc", "form", "form-00001-create-rfc.md");
        writeFile(formPath, "---\ntitle: Create RFC\n---\n");

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler);
        await handler(undefined);

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("Select a document type folder")),
            "Should show selection error",
        );
    });

    test("vector.createDocument shows error when invoked on a non-root node", async () => {
        const config = `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001-create-rfc
`;
        fs.writeFileSync(
            path.join(workspaceRoot, ".vector", "document-types.yaml"),
            config,
            "utf-8",
        );

        activate(context as unknown as import("vscode").ExtensionContext);

        const handler = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(handler);
        await handler({ kind: "group", docType: "rfc", groupKind: "status", value: "draft" });

        const errors = vscode.__getErrorMessages();
        assert.ok(
            errors.some((e) => e.includes("Select a document type folder")),
            "Should show selection error for group node",
        );
    });
});
