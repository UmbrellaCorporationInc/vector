/**
 * Task 00076 Phase B — Verification
 *
 * Integration-level tests that exercise the full launch-prompt pipeline by
 * combining `loadAgentsConfig` + `resolveAgentPrompt` the same way
 * `_handleRunAgent` does at runtime.  Each test below directly corresponds to
 * a Phase B checklist item.
 *
 * Tests for:
 *   - Built-in default prompt is used when `prompt` is absent from agents.yaml
 *   - Configured root-level `prompt` overrides the built-in default
 *   - Every `<instruction-id>` occurrence in a configured prompt is substituted
 *   - A configured prompt without the placeholder is preserved verbatim
 *   - An invalid non-string `prompt` value produces a configuration error
 *   - Full terminal-surface: terminal receives the configured prompt (not the default)
 *   - Full terminal-surface: terminal receives the default prompt when none is configured
 */

import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "./vscode-stub.js";
import {
    DEFAULT_AGENT_PROMPT,
    resolveAgentPrompt,
} from "../document-viewer/document-actions/agentExecutor.js";
import { loadAgentsConfig } from "../document-viewer/document-actions/agentsConfig.js";
import { GovernedDocumentEditorProvider } from "../document-viewer/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Creates an isolated temporary workspace directory and returns cleanup helpers. */
function makeTempWorkspace(label: string): { dir: string; cleanup: () => void } {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `vector-task76b-${label}-`));
    return {
        dir,
        cleanup: () => {
            fs.rmSync(dir, { recursive: true, force: true });
        },
    };
}

/**
 * Writes a minimal `.vector/agents.yaml` in `workspaceDir`.
 *
 * @param workspaceDir  - Root of the isolated workspace.
 * @param extraRootLines - Optional additional root-level YAML lines (e.g. `prompt: "…"`).
 */
function writeAgentsYaml(workspaceDir: string, extraRootLines: string[] = []): void {
    const vectorDir = path.join(workspaceDir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    const lines = [
        `instructions-dir: "\${system-temp}/vector/instructions"`,
        `agents:`,
        `  node-agent:`,
        `    type: cli`,
        `    command: "node <instruction>"`,
        `profiles:`,
        `  code:`,
        `    - node-agent`,
        ...extraRootLines,
    ];
    fs.writeFileSync(path.join(vectorDir, "agents.yaml"), lines.join("\n"), "utf-8");
}

// ---------------------------------------------------------------------------
// 1. Built-in default when `prompt` is absent
// ---------------------------------------------------------------------------

suite("Task 00076 Phase B — default prompt when prompt is absent", () => {
    test("resolveAgentPrompt uses DEFAULT_AGENT_PROMPT when config.prompt is undefined", () => {
        const { dir, cleanup } = makeTempWorkspace("absent-prompt");
        try {
            writeAgentsYaml(dir);
            const load = loadAgentsConfig(dir);
            assert.ok(
                load.ok,
                `config must load: ${load.ok ? "" : load.missing ? "missing" : load.error}`,
            );

            const uuid = "aaaaaaaa-1111-2222-3333-444444444444";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            const expected = DEFAULT_AGENT_PROMPT.replaceAll("<instruction-id>", uuid);
            assert.strictEqual(directive, expected, "must resolve to the built-in default prompt");
        } finally {
            cleanup();
        }
    });

    test("resolved default prompt contains the UUID and not the raw placeholder", () => {
        const { dir, cleanup } = makeTempWorkspace("absent-uuid-check");
        try {
            writeAgentsYaml(dir);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "bbbbbbbb-2222-3333-4444-555555555555";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.ok(directive.includes(uuid), "resolved prompt must embed the UUID");
            assert.ok(!directive.includes("<instruction-id>"), "raw placeholder must not remain");
        } finally {
            cleanup();
        }
    });

    test("resolved default prompt references 'Vector MCP' and not 'server'", () => {
        const { dir, cleanup } = makeTempWorkspace("absent-wording");
        try {
            writeAgentsYaml(dir);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "cccccccc-3333-4444-5555-666666666666";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.ok(directive.includes("Vector MCP"), "must reference 'Vector MCP'");
            assert.ok(!directive.includes("server"), "must not contain the word 'server'");
        } finally {
            cleanup();
        }
    });
});

// ---------------------------------------------------------------------------
// 2. Configured prompt overrides the built-in default
// ---------------------------------------------------------------------------

suite("Task 00076 Phase B — configured prompt overrides default", () => {
    test("resolveAgentPrompt uses the configured prompt, not DEFAULT_AGENT_PROMPT", () => {
        const { dir, cleanup } = makeTempWorkspace("configured-override");
        try {
            // Prompt uses no inner double-quotes so the YAML double-quoted scalar is valid.
            const configured = "Custom agent launcher: id=<instruction-id>";
            writeAgentsYaml(dir, [`prompt: "${configured}"`]);
            const load = loadAgentsConfig(dir);
            assert.ok(
                load.ok,
                `config must load: ${load.ok ? "" : load.missing ? "missing" : load.error}`,
            );
            assert.strictEqual(load.config.prompt, configured, "config must expose the prompt");

            const uuid = "dddddddd-4444-5555-6666-777777777777";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.ok(
                directive.includes("Custom agent launcher"),
                "must use configured prompt text",
            );
            assert.ok(!directive.includes("Vector MCP"), "must not fall back to default wording");
        } finally {
            cleanup();
        }
    });

    test("configured prompt with the placeholder resolves to the exact expected string", () => {
        const { dir, cleanup } = makeTempWorkspace("configured-exact");
        try {
            // Prompt uses no inner double-quotes so the YAML double-quoted scalar is valid.
            const template = "Run task with <instruction-id> now.";
            writeAgentsYaml(dir, [`prompt: "${template}"`]);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "eeeeeeee-5555-6666-7777-888888888888";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.strictEqual(
                directive,
                `Run task with ${uuid} now.`,
                "must produce the exact substituted string",
            );
        } finally {
            cleanup();
        }
    });
});

// ---------------------------------------------------------------------------
// 3. All <instruction-id> occurrences are substituted
// ---------------------------------------------------------------------------

suite("Task 00076 Phase B — all <instruction-id> occurrences are substituted", () => {
    test("every <instruction-id> token in a configured prompt is replaced with the UUID", () => {
        const { dir, cleanup } = makeTempWorkspace("multi-placeholder");
        try {
            // YAML multi-line scalar — use double-quoted string to keep it one line.
            const template = "id=<instruction-id> and backup=<instruction-id>";
            writeAgentsYaml(dir, [`prompt: "${template}"`]);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "ffffffff-6666-7777-8888-999999999999";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.strictEqual(directive, `id=${uuid} and backup=${uuid}`);
            assert.ok(!directive.includes("<instruction-id>"), "no raw placeholder must remain");
        } finally {
            cleanup();
        }
    });

    test("DEFAULT_AGENT_PROMPT itself has exactly one <instruction-id> occurrence after substitution", () => {
        const uuid = "11111111-aaaa-bbbb-cccc-dddddddddddd";
        const directive = resolveAgentPrompt(undefined, uuid);
        // After substitution the placeholder must be gone and the UUID present once.
        const occurrences = directive.split(uuid).length - 1;
        assert.strictEqual(occurrences, 1, "UUID must appear exactly once in the resolved default");
        assert.ok(!directive.includes("<instruction-id>"), "placeholder must be fully replaced");
    });
});

// ---------------------------------------------------------------------------
// 4. Configured prompt without the placeholder is preserved verbatim
// ---------------------------------------------------------------------------

suite("Task 00076 Phase B — configured prompt without placeholder", () => {
    test("a configured prompt without <instruction-id> is returned verbatim", () => {
        const { dir, cleanup } = makeTempWorkspace("no-placeholder");
        try {
            const template = "Execute the current task without any substitution token.";
            writeAgentsYaml(dir, [`prompt: "${template}"`]);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "22222222-bbbb-cccc-dddd-eeeeeeeeeeee";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.strictEqual(directive, template, "verbatim text must be returned unchanged");
            assert.ok(
                !directive.includes(uuid),
                "UUID must not appear when prompt has no placeholder",
            );
        } finally {
            cleanup();
        }
    });

    test("surrounding text around the placeholder is preserved in a configured prompt", () => {
        const { dir, cleanup } = makeTempWorkspace("surrounding-text");
        try {
            const template = "PREFIX <instruction-id> SUFFIX";
            writeAgentsYaml(dir, [`prompt: "${template}"`]);
            const load = loadAgentsConfig(dir);
            assert.ok(load.ok);

            const uuid = "33333333-cccc-dddd-eeee-ffffffffffff";
            const directive = resolveAgentPrompt(load.config.prompt, uuid);
            assert.strictEqual(directive, `PREFIX ${uuid} SUFFIX`);
        } finally {
            cleanup();
        }
    });
});

// ---------------------------------------------------------------------------
// 5. Invalid non-string `prompt` produces a configuration error
// ---------------------------------------------------------------------------

suite("Task 00076 Phase B — invalid non-string prompt produces a config error", () => {
    test("a numeric prompt value is rejected with an error referencing 'prompt' and 'string'", () => {
        const { dir, cleanup } = makeTempWorkspace("invalid-number");
        try {
            writeAgentsYaml(dir, [`prompt: 123`]);
            const load = loadAgentsConfig(dir);
            assert.ok(!load.ok, "config must not load when prompt is a number");
            assert.ok(!load.missing, "result must not be treated as a missing file");
            assert.ok(load.error.includes("prompt"), `error must mention 'prompt': ${load.error}`);
            assert.ok(load.error.includes("string"), `error must mention 'string': ${load.error}`);
        } finally {
            cleanup();
        }
    });

    test("a boolean prompt value is rejected with a configuration error", () => {
        const { dir, cleanup } = makeTempWorkspace("invalid-bool");
        try {
            writeAgentsYaml(dir, [`prompt: false`]);
            const load = loadAgentsConfig(dir);
            assert.ok(!load.ok, "config must not load when prompt is a boolean");
            assert.ok(!load.missing);
            assert.ok(load.error.includes("prompt"), `error must mention 'prompt': ${load.error}`);
        } finally {
            cleanup();
        }
    });

    test("an object prompt value is rejected with a configuration error", () => {
        const { dir, cleanup } = makeTempWorkspace("invalid-object");
        try {
            // YAML object value for prompt
            writeAgentsYaml(dir, [`prompt:`, `  key: value`]);
            const load = loadAgentsConfig(dir);
            assert.ok(!load.ok, "config must not load when prompt is an object");
            assert.ok(!load.missing);
            assert.ok(load.error.includes("prompt"), `error must mention 'prompt': ${load.error}`);
        } finally {
            cleanup();
        }
    });
});

// ---------------------------------------------------------------------------
// 6 & 7. Full terminal-surface integration tests
// ---------------------------------------------------------------------------

const extensionRoot = new URL("../../", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");

/**
 * Writes a complete agent workspace with optional root-level prompt and a
 * minimal prompt document for the `resolveGovernedPreviewSource` pipeline.
 */
function makeAgentWorkspace(extraRootLines: string[] = []): { dir: string; docPath: string } {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-task76b-surface-"));
    const vectorDir = path.join(dir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });

    // Use `node` as the agent command — guaranteed in PATH in any Node test runner.
    const agentLines = [
        `instructions-dir: "\${system-temp}/vector/instructions"`,
        `agents:`,
        `  node-agent:`,
        `    type: cli`,
        `    command: "node <instruction>"`,
        `profiles:`,
        `  code:`,
        `    - node-agent`,
        ...extraRootLines,
    ];
    fs.writeFileSync(path.join(vectorDir, "agents.yaml"), agentLines.join("\n"), "utf-8");

    fs.writeFileSync(
        path.join(vectorDir, "document-types.yaml"),
        ["document-types:", "  prompts:", "    layout: directory", '    "code-width": 5'].join(
            "\n",
        ),
        "utf-8",
    );

    const promptDir = path.join(dir, "doc", "prompts");
    fs.mkdirSync(promptDir, { recursive: true });
    const docPath = path.join(promptDir, "prompts-00001-run-task.md");
    fs.writeFileSync(docPath, "---\ntitle: Run Task\n---\n\nExecute the task.\n", "utf-8");

    return { dir, docPath };
}

suite("Task 00076 Phase B — full terminal surface: configured prompt", () => {
    setup(() => {
        vscode.__resetTerminalState();
        vscode.__resetUiState();
    });

    teardown(() => {
        vscode.__resetTerminalState();
        vscode.__resetUiState();
    });

    test("terminal receives a command containing the configured prompt text, not the default", async () => {
        const customText = "CUSTOM_DIRECTIVE_MARKER";
        const { dir, docPath } = makeAgentWorkspace([
            `prompt: "${customText} with id <instruction-id> done."`,
        ]);
        try {
            const provider = new GovernedDocumentEditorProvider(
                dir,
                vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
                () => true,
            );
            const stubPanel = vscode.window.createWebviewPanel("vector.documentPreview", docPath);
            const document = provider.openCustomDocument(
                vscode.Uri.file(docPath) as unknown as import("vscode").Uri,
            );
            provider.resolveCustomEditor(
                document,
                stubPanel as unknown as import("vscode").WebviewPanel,
            );

            vscode.__fireWebviewMessage(stubPanel, {
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00001-run-task",
                label: "Phase B Surface Test",
                staticInput: {},
                formValues: {},
            });

            await new Promise((resolve) => setTimeout(resolve, 50));

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1, "must create exactly one terminal");
            const sentText = terminals[0]?.sentText[0] ?? "";
            assert.ok(
                sentText.includes(customText),
                `command must contain the configured prompt text "${customText}", got: ${sentText}`,
            );
            assert.ok(
                !sentText.includes("Vector MCP"),
                "command must not fall back to the default prompt wording",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("terminal receives a command containing 'Vector MCP' when no prompt is configured", async () => {
        const { dir, docPath } = makeAgentWorkspace(); // no extra root lines
        try {
            const provider = new GovernedDocumentEditorProvider(
                dir,
                vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
                () => true,
            );
            const stubPanel = vscode.window.createWebviewPanel("vector.documentPreview", docPath);
            const document = provider.openCustomDocument(
                vscode.Uri.file(docPath) as unknown as import("vscode").Uri,
            );
            provider.resolveCustomEditor(
                document,
                stubPanel as unknown as import("vscode").WebviewPanel,
            );

            vscode.__fireWebviewMessage(stubPanel, {
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00001-run-task",
                label: "Phase B Default Prompt Test",
                staticInput: {},
                formValues: {},
            });

            await new Promise((resolve) => setTimeout(resolve, 50));

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1, "must create exactly one terminal");
            const sentText = terminals[0]?.sentText[0] ?? "";
            assert.ok(
                sentText.includes("Vector MCP"),
                `command must contain 'Vector MCP' from the default prompt, got: ${sentText}`,
            );
            assert.ok(
                sentText.includes("get_instruction"),
                `command must reference get_instruction, got: ${sentText}`,
            );
            assert.ok(
                !sentText.includes("server"),
                `command must not contain the word 'server', got: ${sentText}`,
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("configured prompt with multiple <instruction-id> tokens: all are replaced in terminal command", async () => {
        const { dir, docPath } = makeAgentWorkspace([
            `prompt: "first=<instruction-id> second=<instruction-id>"`,
        ]);
        try {
            const provider = new GovernedDocumentEditorProvider(
                dir,
                vscode.Uri.file(extensionRoot) as unknown as import("vscode").Uri,
                () => true,
            );
            const stubPanel = vscode.window.createWebviewPanel("vector.documentPreview", docPath);
            const document = provider.openCustomDocument(
                vscode.Uri.file(docPath) as unknown as import("vscode").Uri,
            );
            provider.resolveCustomEditor(
                document,
                stubPanel as unknown as import("vscode").WebviewPanel,
            );

            vscode.__fireWebviewMessage(stubPanel, {
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00001-run-task",
                label: "Multi-Placeholder Test",
                staticInput: {},
                formValues: {},
            });

            await new Promise((resolve) => setTimeout(resolve, 50));

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1);
            const sentText = terminals[0]?.sentText[0] ?? "";
            assert.ok(
                !sentText.includes("<instruction-id>"),
                "no raw <instruction-id> token must remain in the command",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
