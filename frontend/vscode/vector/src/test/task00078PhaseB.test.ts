import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";
import { activate } from "../extension.js";
import { renderAgentBlock } from "../document-viewer/document-actions/agentBlockRenderer.js";
import { documentActionMetadata } from "../document-viewer/document-actions/actionRuntimeContext.js";
import { GovernedDocumentEditorProvider } from "../document-viewer/governedDocumentEditorProvider.js";
import {
    cleanupAllTempFiles,
    writeTempDocument,
} from "../document-viewer/document-actions/agentExecutor.js";
import { PerTypeDocumentProvider } from "../governedDocumentProvider.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

function writeFile(filePath: string, content: string): void {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, content, "utf-8");
}

function makeWorkspace(): string {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "vector-task78-phaseb-"));
    writeFile(
        path.join(root, ".vector", "document-types.yaml"),
        "document-types:\n  task:\n    layout: directory\n  prompts:\n    layout: directory\n",
    );
    writeFile(
        path.join(root, ".vector", "agents.yaml"),
        [
            `instructions-dir: "\${system-temp}/${path.basename(root)}/instructions"`,
            "agents:",
            "  node:",
            "    type: cli",
            "    command: node '<instruction>'",
            "profiles:",
            "  configured: [node]",
        ].join("\n"),
    );
    writeFile(path.join(root, "doc", "task", "task-00001-source.md"), "# Source\n");
    writeFile(
        path.join(root, "doc", "prompts", "prompts-00001-action.md"),
        "Run #{label} #{custom} #{file-path} #{file-name} #{message} #{details} #{input} #{nested}",
    );
    writeFile(path.join(root, "doc", "prompts", "prompts-00002-create.md"), "Create #{doc-type}");
    return root;
}

suite("Task 00078 Phase B — action entry points", () => {
    let root: string;

    setup(() => {
        root = makeWorkspace();
        vscode.__resetUiState();
        vscode.__resetTerminalState();
        vscode.__resetCommandHandlers();
    });

    teardown(() => {
        cleanupAllTempFiles();
        fs.rmSync(root, { recursive: true, force: true });
        vscode.workspace.workspaceFolders = undefined;
        vscode.__resetTerminalState();
    });

    test("block and inline renderers preserve parsed action YAML for runtime context", () => {
        const yaml =
            "label: Run\nprofile: code\nprompt: prompts-00001-action\ncustom: supplied\ninput:\n  message: hello\n";
        for (const variant of ["action", "inline-action"] as const) {
            const html = renderAgentBlock(yaml, variant);
            assert.ok(html.includes('data-agent-action-id="vector-action"'));
            assert.ok(html.includes("data-agent-context="));
            assert.ok(html.includes("custom"));
            assert.ok(html.includes("supplied"));
        }
        assert.ok(!renderAgentBlock(yaml, "button").includes("data-agent-action-id"));
    });

    test("document metadata is relative to doc and rejects paths outside it", () => {
        const docPath = path.join(root, "doc", "task", "task-00001-source.md");
        assert.deepStrictEqual(documentActionMetadata(root, docPath), {
            "file-path": "task/task-00001-source.md",
            "file-name": "task-00001-source.md",
        });
        const packagePath = path.join(
            root,
            ".vector-database",
            "packages",
            "shared",
            "doc",
            "task",
            "task-00002-shared.md",
        );
        assert.deepStrictEqual(documentActionMetadata(root, packagePath), {
            "file-path": "task/task-00002-shared.md",
            "file-name": "task-00002-shared.md",
        });
        assert.strictEqual(documentActionMetadata(root, path.join(root, "outside.md")), null);
    });

    test("configured document action uses YAML, form values, and generated metadata", async () => {
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  vector-action:\n    profile: configured\n    prompt: prompts-00001-action\n    input:\n      custom: default\n      file-path: spoofed\n",
        );
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const docUri = vscode.Uri.file(path.join(root, "doc", "task", "task-00001-source.md"));
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(docUri as unknown as import("vscode").Uri),
            panel as unknown as import("vscode").WebviewPanel,
        );
        vscode.__fireWebviewMessage(panel, {
            type: "vector.runAgent",
            actionId: "vector-action",
            actionContext: {
                label: "Run",
                profile: "ignored",
                prompt: "ignored",
                custom: "supplied",
                details: { nested: ["one", 2] },
                input: { message: "block", nested: { enabled: true } },
                "file-path": "spoofed-again",
            },
            profile: "ignored",
            prompt: "ignored",
            label: "Run",
            staticInput: { message: "block", nested: "" },
            formValues: { message: "form" },
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        const terminals = vscode.__getCreatedTerminals();
        assert.strictEqual(terminals.length, 1, vscode.__getErrorMessages().join(" | "));
        assert.strictEqual(terminals[0]?.cwd, root);
        const files = fs.readdirSync(path.join(root, "instructions"));
        assert.strictEqual(files.length, 1);
        assert.strictEqual(
            fs.readFileSync(path.join(root, "instructions", files[0] ?? ""), "utf-8"),
            'Run Run supplied task/task-00001-source.md task-00001-source.md form {"nested":["one",2]} {"message":"block","nested":{"enabled":true}} {"enabled":true}',
        );
    });

    test("document action retains built-in launch when override is absent", async () => {
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const docUri = vscode.Uri.file(path.join(root, "doc", "task", "task-00001-source.md"));
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(docUri as unknown as import("vscode").Uri),
            panel as unknown as import("vscode").WebviewPanel,
        );
        vscode.__fireWebviewMessage(panel, {
            type: "vector.runAgent",
            actionId: "vector-action",
            actionContext: { label: "Run", profile: "configured" },
            profile: "configured",
            prompt: "prompts-00002-create",
            label: "Run",
            staticInput: { "doc-type": "task" },
            formValues: {},
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        assert.strictEqual(vscode.__getCreatedTerminals().length, 1);
    });

    test("invalid configured document action fails without launching built-in prompt", async () => {
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  vector-action:\n    profile: missing\n",
        );
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const docUri = vscode.Uri.file(path.join(root, "doc", "task", "task-00001-source.md"));
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(docUri as unknown as import("vscode").Uri),
            panel as unknown as import("vscode").WebviewPanel,
        );
        vscode.__fireWebviewMessage(panel, {
            type: "vector.runAgent",
            actionId: "vector-action",
            actionContext: {},
            profile: "configured",
            prompt: "prompts-00002-create",
            label: "Run",
            staticInput: { "doc-type": "task" },
            formValues: {},
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        assert.strictEqual(vscode.__getCreatedTerminals().length, 0);
        assert.ok(vscode.__getErrorMessages().some((message) => message.includes("vector-action")));
    });

    test("configured action in a temporary create form uses its governed source path", async () => {
        const formPath = path.join(root, "doc", "form", "form-00001-create.md");
        writeFile(formPath, "# Create form\n");
        writeFile(
            path.join(root, "doc", "prompts", "prompts-00003-form.md"),
            "Source #{file-path} #{file-name}",
        );
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  vector-action:\n    profile: configured\n    prompt: prompts-00003-form\n",
        );
        const tempPath = writeTempDocument("# Create form\n", formPath);
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const tempUri = vscode.Uri.file(tempPath);
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(tempUri as unknown as import("vscode").Uri),
            panel as unknown as import("vscode").WebviewPanel,
        );
        vscode.__fireWebviewMessage(panel, {
            type: "vector.runAgent",
            actionId: "vector-action",
            actionContext: { label: "Create" },
            profile: "configured",
            prompt: "prompts-00003-form",
            label: "Create",
            staticInput: {},
            formValues: {},
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        assert.strictEqual(vscode.__getCreatedTerminals().length, 1);
        const files = fs.readdirSync(path.join(root, "instructions"));
        assert.strictEqual(
            fs.readFileSync(path.join(root, "instructions", files[0] ?? ""), "utf-8"),
            "Source form/form-00001-create.md form-00001-create.md",
        );
    });

    test("sidebar create-doc override launches with selected doc-type", async () => {
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  create-doc:\n    profile: configured\n    prompt: prompts-00002-create\n",
        );
        const tree = new PerTypeDocumentProvider(root);
        assert.strictEqual(
            tree.getTreeItem({ kind: "root", docType: "task" }).contextValue,
            "docTypeFolder",
        );
        vscode.workspace.workspaceFolders = [
            {
                uri: vscode.Uri.file(root),
                name: "test",
                index: 0,
            },
        ];
        const context: vscode.ExtensionContext = {
            subscriptions: [],
            extensionUri: vscode.Uri.file(extensionRoot),
        };
        activate(context as unknown as import("vscode").ExtensionContext);
        const command = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(command);
        await command({ kind: "root", docType: "task" });
        assert.strictEqual(
            vscode.__getCreatedTerminals().length,
            1,
            vscode.__getErrorMessages().join(" | "),
        );
        const files = fs.readdirSync(path.join(root, "instructions"));
        assert.strictEqual(
            fs.readFileSync(path.join(root, "instructions", files[0] ?? ""), "utf-8"),
            "Create task",
        );
    });

    test("sidebar uses its create form when create-doc is not configured", async () => {
        writeFile(
            path.join(root, ".vector", "document-types.yaml"),
            "document-types:\n  task:\n    layout: directory\n    create-document-form: form-00001-create\n  prompts:\n    layout: directory\n",
        );
        writeFile(
            path.join(root, "doc", "form", "form-00001-create.md"),
            "Type #{document-type}\n",
        );
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  unrelated:\n    profile: configured\n    prompt: prompts-00002-create\n",
        );
        vscode.workspace.workspaceFolders = [
            { uri: vscode.Uri.file(root), name: "test", index: 0 },
        ];
        activate({
            subscriptions: [],
            extensionUri: vscode.Uri.file(extensionRoot),
        } as unknown as import("vscode").ExtensionContext);

        const command = vscode.__getCommandHandler("vector.createDocument");
        assert.ok(command);
        await command({ kind: "root", docType: "task" });

        assert.strictEqual(vscode.__getCreatedTerminals().length, 0);
        assert.ok(
            vscode.__getExecutedCommands().some((entry) => entry.command === "vscode.openWith"),
            vscode.__getErrorMessages().join(" | "),
        );
    });

    test("inline action falls back to its own prompt when vector-action is absent", async () => {
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  create-doc:\n    profile: configured\n    prompt: prompts-00002-create\n",
        );
        writeFile(
            path.join(root, "doc", "prompts", "prompts-00003-inline.md"),
            "Inline #{prompt-message}",
        );
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(
                vscode.Uri.file(
                    path.join(root, "doc", "task", "task-00001-source.md"),
                ) as unknown as import("vscode").Uri,
            ),
            panel as unknown as import("vscode").WebviewPanel,
        );
        vscode.__fireWebviewMessage(panel, {
            type: "vector.runAgent",
            actionId: "vector-action",
            actionContext: { label: "Inline" },
            profile: "configured",
            prompt: "prompts-00003-inline",
            label: "Inline",
            staticInput: {},
            formValues: { "prompt-message": "user response" },
        });
        await new Promise((resolve) => setTimeout(resolve, 20));

        assert.strictEqual(vscode.__getCreatedTerminals().length, 1);
        const files = fs.readdirSync(path.join(root, "instructions"));
        assert.strictEqual(
            fs.readFileSync(path.join(root, "instructions", files[0] ?? ""), "utf-8"),
            "Inline user response",
        );
    });

    test("configured profile presents its agents and launches the selected one", async () => {
        writeFile(
            path.join(root, ".vector", "agents.yaml"),
            [
                `instructions-dir: "\${system-temp}/${path.basename(root)}/instructions"`,
                "agents:",
                "  first:",
                "    type: cli",
                "    command: node '<instruction>'",
                "  second:",
                "    type: cli",
                "    command: node '<instruction>'",
                "profiles:",
                "  configured: [first, second]",
            ].join("\n"),
        );
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            "actions:\n  vector-action:\n    profile: configured\n    prompt: prompts-00002-create\n",
        );
        const provider = new GovernedDocumentEditorProvider(
            root,
            vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
            () => true,
        );
        const panel = vscode.window.createWebviewPanel("vector.documentPreview", "Test");
        provider.resolveCustomEditor(
            provider.openCustomDocument(
                vscode.Uri.file(
                    path.join(root, "doc", "task", "task-00001-source.md"),
                ) as unknown as import("vscode").Uri,
            ),
            panel as unknown as import("vscode").WebviewPanel,
        );
        const pickerWindow = vscode.window as unknown as {
            showQuickPick: (
                items: Array<{ label: string }>,
            ) => Promise<{ label: string } | undefined>;
        };
        const originalShowQuickPick = pickerWindow.showQuickPick;
        let presented: string[] = [];
        pickerWindow.showQuickPick = (items: Array<{ label: string }>) => {
            presented = items.map((item) => item.label);
            return Promise.resolve(items[1]);
        };
        try {
            vscode.__fireWebviewMessage(panel, {
                type: "vector.runAgent",
                actionId: "vector-action",
                actionContext: { "doc-type": "task" },
                profile: "ignored",
                prompt: "ignored",
                label: "Select",
                staticInput: {},
                formValues: {},
            });
            await new Promise((resolve) => setTimeout(resolve, 20));
        } finally {
            pickerWindow.showQuickPick = originalShowQuickPick;
        }

        assert.deepStrictEqual(presented, ["first", "second"]);
        assert.strictEqual(vscode.__getCreatedTerminals().length, 1);
        assert.ok(vscode.__getCreatedTerminals()[0]?.name.includes("second"));
    });
});
