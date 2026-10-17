import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { fileURLToPath } from "url";
import * as vscode from "./vscode-stub.js";
import { activate } from "../extension.js";
import { GovernedDocumentEditorProvider } from "../document-viewer/governedDocumentEditorProvider.js";
import { cleanupAllTempFiles } from "../document-viewer/document-actions/agentExecutor.js";
import {
    loadActionsConfig,
    resolveConfiguredAction,
} from "../document-viewer/document-actions/actionsConfig.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

function writeFile(filePath: string, content: string): void {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, content, "utf-8");
}

suite("Task 00079 Phase A — optional action profiles", () => {
    let root: string;

    setup(() => {
        root = fs.mkdtempSync(path.join(os.tmpdir(), "vector-task79-phasea-"));
        writeFile(
            path.join(root, ".vector", "agents.yaml"),
            [
                `instructions-dir: "\${system-temp}/${path.basename(root)}/instructions"`,
                "agents:",
                "  node:",
                "    type: cli",
                "    command: node '<instruction>'",
                "profiles:",
                "  runtime: [node]",
                "  configured: [node]",
            ].join("\n"),
        );
        writeFile(
            path.join(root, ".vector", "document-types.yaml"),
            "document-types:\n  task:\n    layout: directory\n  prompts:\n    layout: directory\n",
        );
        writeFile(path.join(root, "doc", "prompts", "prompts-00001-action.md"), "Run #{doc-type}");
        writeFile(path.join(root, "doc", "task", "task-00001-source.md"), "# Source\n");
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

    function configure(actionId: string, profile?: string): void {
        writeFile(
            path.join(root, ".vector", "actions.yaml"),
            `actions:\n  ${actionId}:\n${profile ? `    profile: ${profile}\n` : ""}    prompt: prompts-00001-action\n`,
        );
    }

    for (const actionId of ["create-doc", "vector-action"]) {
        test(`${actionId} uses configured profile before runtime profile`, () => {
            configure(actionId, "configured");
            const result = resolveConfiguredAction({
                workspaceRoot: root,
                actionId,
                runtimeProfile: "runtime",
                extensionGeneratedValues: { "doc-type": "task" },
                isAvailable: () => true,
            });
            assert.strictEqual(result.type, "resolved");
            assert.strictEqual(result.effectiveProfile, "configured");
            assert.strictEqual(result.renderedPrompt, "Run task");
        });

        test(`${actionId} inherits runtime profile when omitted`, () => {
            configure(actionId);
            const loaded = loadActionsConfig(root);
            assert.strictEqual(loaded.ok, true);
            assert.strictEqual(loaded.config.actions[actionId]?.profile, undefined);
            const result = resolveConfiguredAction({
                workspaceRoot: root,
                actionId,
                runtimeProfile: "runtime",
                extensionGeneratedValues: { "doc-type": "task" },
                isAvailable: () => true,
            });
            assert.strictEqual(result.type, "resolved");
            assert.strictEqual(result.effectiveProfile, "runtime");
            assert.strictEqual(result.renderedPrompt, "Run task");
        });

        test(`${actionId} reports missing and unknown effective profiles`, () => {
            configure(actionId);
            const missing = resolveConfiguredAction({ workspaceRoot: root, actionId });
            assert.strictEqual(missing.type, "error");
            assert.match(missing.error, /profile.*\.vector\/actions\.yaml.*runtime action/);
            const unknown = resolveConfiguredAction({
                workspaceRoot: root,
                actionId,
                runtimeProfile: "unknown",
                isAvailable: () => true,
            });
            assert.strictEqual(unknown.type, "error");
            assert.match(unknown.error, /profile 'unknown'/);
            assert.match(unknown.error, /\.vector\/agents\.yaml/);
        });

        test(`${actionId} reports unavailable agents in the effective profile`, () => {
            configure(actionId);
            const result = resolveConfiguredAction({
                workspaceRoot: root,
                actionId,
                runtimeProfile: "runtime",
                extensionGeneratedValues: { "doc-type": "task" },
                isAvailable: () => false,
            });
            assert.strictEqual(result.type, "resolved");
            assert.strictEqual(result.agents[0]?.available, false);
            assert.strictEqual(result.effectiveProfile, "runtime");
        });

        test(`${actionId} preserves absent override fallback and invalid entry error`, () => {
            assert.strictEqual(
                resolveConfiguredAction({ workspaceRoot: root, actionId }).type,
                "fallback",
            );
            writeFile(path.join(root, ".vector", "actions.yaml"), "actions: {}\n");
            assert.strictEqual(
                resolveConfiguredAction({ workspaceRoot: root, actionId }).type,
                "fallback",
            );
            writeFile(
                path.join(root, ".vector", "actions.yaml"),
                `actions:\n  ${actionId}:\n    profile: ""\n    prompt: prompts-00001-action\n`,
            );
            const invalid = resolveConfiguredAction({ workspaceRoot: root, actionId });
            assert.strictEqual(invalid.type, "error");
        });
    }

    test("present null, empty, and non-string profiles remain invalid", () => {
        for (const value of ["null", '""', "42", "[]"]) {
            writeFile(
                path.join(root, ".vector", "actions.yaml"),
                `actions:\n  create-doc:\n    profile: ${value}\n    prompt: prompts-00001-action\n`,
            );
            const loaded = loadActionsConfig(root);
            assert.strictEqual(loaded.ok, true);
            assert.ok(loaded.config.invalidActionIds.has("create-doc"));
            assert.ok(loaded.config.diagnostics.some((d) => d.invalidField === "profile"));
        }
    });

    test("clicked Markdown profile drives vector-action when configuration omits profile", async () => {
        configure("vector-action");
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
            actionContext: { profile: "runtime" },
            profile: "runtime",
            prompt: "ignored",
            label: "Run",
            staticInput: { "doc-type": "task" },
            formValues: {},
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        assert.strictEqual(
            vscode.__getCreatedTerminals().length,
            1,
            vscode.__getErrorMessages().join(" | "),
        );
    });

    test("document action reports an absent runtime profile when YAML also omits it", async () => {
        configure("vector-action");
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
            actionContext: {},
            prompt: "ignored",
            label: "Run",
            staticInput: { "doc-type": "task" },
            formValues: {},
        });
        await new Promise((resolve) => setTimeout(resolve, 20));
        assert.strictEqual(vscode.__getCreatedTerminals().length, 0);
        assert.ok(vscode.__getErrorMessages().some((message) => message.includes("profile")));
    });

    test("tree create-doc command fails when configured profile and runtime profile are absent", async () => {
        configure("create-doc");
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
        assert.ok(vscode.__getErrorMessages().some((message) => message.includes("profile")));
    });
});
