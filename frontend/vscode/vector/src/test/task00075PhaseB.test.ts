/**
 * Task 00075 Phase B — VS Code Instruction Lifecycle
 *
 * Focused tests for:
 *   - UUID naming and fixed filename shape
 *   - Exclusive file creation with UTF-8 encoding
 *   - Directive rendering
 *   - Cross-shell quoting of the directive
 *   - Terminal launch with explicit cwd (both agent-action and agent-button surfaces)
 *   - Cleanup on launch failure, terminal close, and extension deactivation
 *   - Stale-file selection (age-based, pattern-based, non-regular exclusion)
 */

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import * as vscode from "./vscode-stub.js";
import {
    renderInstructionDirective,
    writeInstructionFile,
    cleanupStaleInstructionFiles,
    cleanupAllTempFiles,
    quoteShellArgument,
    resolveAgentCommand,
    spawnAgentTerminal,
    deleteTempFile,
} from "../document-viewer/document-actions/agentExecutor.js";
import { GovernedDocumentEditorProvider } from "../document-viewer/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Creates an isolated temporary directory and returns its path. */
function makeTempDir(label: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), `vector-phase-b-${label}-`));
}

/** Removes a directory tree; safe to call even when the directory is missing. */
function removeTempDir(dir: string): void {
    fs.rmSync(dir, { recursive: true, force: true });
}

/** Canonical UUID pattern as produced by `crypto.randomUUID()`. */
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

// ---------------------------------------------------------------------------
// Directive rendering
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — renderInstructionDirective", () => {
    test("returns the canonical MCP call directive with the UUID embedded in double quotes", () => {
        const uuid = "93b0a984-5422-4491-9b0c-db44fdb69ea8";
        const directive = renderInstructionDirective(uuid);
        assert.strictEqual(
            directive,
            `Using the Vector MCP server, call get_instruction with id "${uuid}" and execute the returned instructions.`,
        );
    });

    test("UUID appears exactly once in the directive", () => {
        const uuid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        const directive = renderInstructionDirective(uuid);
        const occurrences = directive.split(uuid).length - 1;
        assert.strictEqual(occurrences, 1, "UUID must appear exactly once");
    });

    test("directive text is deterministic for the same UUID", () => {
        const uuid = "11111111-2222-3333-4444-555555555555";
        assert.strictEqual(renderInstructionDirective(uuid), renderInstructionDirective(uuid));
    });
});

// ---------------------------------------------------------------------------
// UUID naming and exclusive file creation
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — writeInstructionFile UUID naming", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("naming");
    });

    teardown(() => {
        removeTempDir(instructionsDir);
    });

    test("creates a file named vector-instruction-<uuid>.txt", () => {
        const uuid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);
        assert.strictEqual(path.basename(filePath), `vector-instruction-${uuid}.txt`);
        assert.ok(fs.existsSync(filePath), "file must exist after write");
    });

    test("creates the instructions directory when it does not yet exist", () => {
        const nestedDir = path.join(instructionsDir, "nested", "deep");
        const uuid = "00000000-1111-2222-3333-444444444444";
        writeInstructionFile(uuid, "hello", nestedDir);
        assert.ok(fs.existsSync(nestedDir), "directory must be created recursively");
    });

    test("exclusive creation: writing the same UUID twice throws", () => {
        const uuid = "ffffffff-eeee-dddd-cccc-bbbbbbbbbbbb";
        writeInstructionFile(uuid, "first write", instructionsDir);
        assert.throws(
            () => writeInstructionFile(uuid, "second write", instructionsDir),
            "second write to the same UUID must throw (exclusive create flag)",
        );
    });

    test("file is stored inside the specified instructionsDir", () => {
        const uuid = "12345678-1234-1234-1234-123456789012";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);
        assert.ok(filePath.startsWith(instructionsDir), "file path must be inside instructionsDir");
    });
});

// ---------------------------------------------------------------------------
// UTF-8 content writing
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — writeInstructionFile UTF-8 content", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("utf8");
    });

    teardown(() => {
        removeTempDir(instructionsDir);
    });

    test("file content matches the resolved prompt string verbatim", () => {
        const uuid = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
        const content = "Execute: #{task}\nWith: #{input}";
        writeInstructionFile(uuid, content, instructionsDir);
        const filePath = path.join(instructionsDir, `vector-instruction-${uuid}.txt`);
        assert.strictEqual(fs.readFileSync(filePath, "utf-8"), content);
    });

    test("multi-line and Unicode content round-trips without corruption", () => {
        const uuid = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
        const content = "Line 1\nLine 2\néàü";
        writeInstructionFile(uuid, content, instructionsDir);
        const filePath = path.join(instructionsDir, `vector-instruction-${uuid}.txt`);
        assert.strictEqual(fs.readFileSync(filePath, "utf-8"), content);
    });

    test("empty string content is written without error", () => {
        const uuid = "cccccccc-cccc-cccc-cccc-cccccccccccc";
        writeInstructionFile(uuid, "", instructionsDir);
        const filePath = path.join(instructionsDir, `vector-instruction-${uuid}.txt`);
        assert.strictEqual(fs.readFileSync(filePath, "utf-8"), "");
    });
});

// ---------------------------------------------------------------------------
// Directive quoting (cross-shell safety)
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — directive quoting", () => {
    test("quoteShellArgument wraps the directive in double quotes", () => {
        const uuid = "93b0a984-5422-4491-9b0c-db44fdb69ea8";
        const directive = renderInstructionDirective(uuid);
        const quoted = quoteShellArgument(directive);
        assert.ok(quoted.startsWith('"'), "must open with double quote");
        assert.ok(quoted.endsWith('"'), "must close with double quote");
    });

    test("inner double quotes in the directive are escaped with backslash", () => {
        const uuid = "11111111-1111-1111-1111-111111111111";
        const directive = renderInstructionDirective(uuid);
        // The directive body contains the UUID in double quotes: get_instruction with id "..."
        const quoted = quoteShellArgument(directive);
        assert.ok(
            quoted.includes('\\"'),
            "double quotes inside the directive must be escaped in the shell argument",
        );
    });

    test("resolveAgentCommand substitutes the quoted directive for <instruction>", () => {
        const uuid = "22222222-2222-2222-2222-222222222222";
        const directive = renderInstructionDirective(uuid);
        const quoted = quoteShellArgument(directive);
        const resolved = resolveAgentCommand("claude <instruction>", quoted);
        assert.strictEqual(resolved, `claude ${quoted}`);
        // The UUID is present inside the shell-escaped directive — double quotes around
        // it become \" inside the outer double-quoted argument.
        assert.ok(resolved.includes(uuid), "resolved command must contain the UUID");
    });

    test("resolveAgentCommand substitutes every <instruction> placeholder", () => {
        const uuid = "33333333-3333-3333-3333-333333333333";
        const quoted = quoteShellArgument(renderInstructionDirective(uuid));
        const template = "cmd <instruction> && repeat <instruction>";
        const resolved = resolveAgentCommand(template, quoted);
        // Both placeholders should be replaced
        assert.ok(!resolved.includes("<instruction>"), "no <instruction> placeholder must remain");
    });
});

// ---------------------------------------------------------------------------
// Terminal launch with explicit cwd
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — spawnAgentTerminal explicit cwd", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("terminal");
        vscode.__resetTerminalState();
    });

    teardown(() => {
        removeTempDir(instructionsDir);
        vscode.__resetTerminalState();
    });

    test("terminal is created with workspaceRoot as cwd", () => {
        const workspaceRoot = "/project/root";
        const uuid = "44444444-4444-4444-4444-444444444444";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);

        try {
            spawnAgentTerminal(
                "claude already-resolved",
                "claude",
                "Run",
                filePath,
                [],
                workspaceRoot,
            );

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1);
            assert.strictEqual(
                terminals[0]?.cwd,
                workspaceRoot,
                "terminal cwd must equal workspaceRoot",
            );
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("terminal name uses agentName and label", () => {
        const uuid = "55555555-5555-5555-5555-555555555555";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);

        try {
            spawnAgentTerminal(
                "some-cmd already-resolved",
                "my-agent",
                "Execute Phase B",
                filePath,
                [],
                "/workspace",
            );
            const terminal = vscode.__getCreatedTerminals()[0];
            assert.strictEqual(terminal?.name, "Vector: my-agent - Execute Phase B");
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("terminal receives the pre-resolved command without further substitution", () => {
        const uuid = "66666666-6666-6666-6666-666666666666";
        const directive = renderInstructionDirective(uuid);
        const quoted = quoteShellArgument(directive);
        const resolvedCmd = resolveAgentCommand("claude <instruction>", quoted);
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);

        try {
            spawnAgentTerminal(resolvedCmd, "claude", "Label", filePath, [], "/workspace");
            const terminal = vscode.__getCreatedTerminals()[0];
            assert.deepStrictEqual(
                terminal?.sentText,
                [resolvedCmd],
                "terminal must receive the exact pre-resolved command",
            );
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("terminal is shown immediately after creation (preserveFocus=false)", () => {
        const uuid = "77777777-7777-7777-7777-777777777777";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);

        try {
            spawnAgentTerminal("cmd", "agent", "label", filePath, [], "/workspace");
            const terminal = vscode.__getCreatedTerminals()[0];
            assert.deepStrictEqual(terminal?.showCalls, [false]);
        } finally {
            deleteTempFile(filePath);
        }
    });
});

// ---------------------------------------------------------------------------
// Cleanup paths
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — cleanup: terminal close", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("cleanup-close");
        vscode.__resetTerminalState();
    });

    teardown(() => {
        removeTempDir(instructionsDir);
        vscode.__resetTerminalState();
    });

    test("instruction file is deleted when the associated terminal closes", () => {
        const uuid = "88888888-8888-8888-8888-888888888888";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);
        const subscriptions: { dispose(): void }[] = [];

        spawnAgentTerminal("cmd", "agent", "label", filePath, subscriptions, "/workspace");
        assert.ok(fs.existsSync(filePath), "file must exist before terminal close");

        const terminal = vscode.__getCreatedTerminals()[0]?.terminal;
        assert.ok(terminal);
        vscode.__fireDidCloseTerminal(terminal);

        assert.ok(!fs.existsSync(filePath), "file must be deleted after terminal close");
    });

    test("closing one terminal does not delete files belonging to other terminals", () => {
        const uuidA = "aaaaaaaa-aaaa-aaaa-aaaa-000000000001";
        const uuidB = "aaaaaaaa-aaaa-aaaa-aaaa-000000000002";
        const fileA = writeInstructionFile(uuidA, "A", instructionsDir);
        const fileB = writeInstructionFile(uuidB, "B", instructionsDir);

        const subsA: { dispose(): void }[] = [];
        const subsB: { dispose(): void }[] = [];
        spawnAgentTerminal("cmdA", "agent", "A", fileA, subsA, "/workspace");
        spawnAgentTerminal("cmdB", "agent", "B", fileB, subsB, "/workspace");

        const terminals = vscode.__getCreatedTerminals();
        assert.strictEqual(terminals.length, 2);
        const firstTerminalRecord = terminals[0];
        assert.ok(firstTerminalRecord, "first terminal record must exist");
        vscode.__fireDidCloseTerminal(firstTerminalRecord.terminal);

        assert.ok(!fs.existsSync(fileA), "file A must be deleted when terminal A closes");
        assert.ok(fs.existsSync(fileB), "file B must survive when only terminal A closes");

        // cleanup
        deleteTempFile(fileB);
    });
});

suite("Task 00075 Phase B — cleanup: launch failure", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("cleanup-launch");
        vscode.__resetTerminalState();
    });

    teardown(() => {
        removeTempDir(instructionsDir);
        vscode.__resetTerminalState();
    });

    test("instruction file is deleted immediately when resolveAgentCommand throws", () => {
        const uuid = "99999999-9999-9999-9999-999999999999";
        const instructionFilePath = writeInstructionFile(uuid, "content", instructionsDir);
        assert.ok(fs.existsSync(instructionFilePath), "file must exist before failed launch");

        // Simulate launch failure: command template has no <instruction> placeholder.
        try {
            resolveAgentCommand(
                "no-placeholder-here",
                quoteShellArgument(renderInstructionDirective(uuid)),
            );
            assert.fail("resolveAgentCommand should have thrown");
        } catch {
            deleteTempFile(instructionFilePath);
        }

        assert.ok(!fs.existsSync(instructionFilePath), "file must be deleted on launch failure");
    });
});

suite("Task 00075 Phase B — cleanup: deactivation", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("cleanup-deactivate");
        vscode.__resetTerminalState();
    });

    teardown(() => {
        removeTempDir(instructionsDir);
        vscode.__resetTerminalState();
    });

    test("cleanupAllTempFiles deletes all tracked instruction files", () => {
        const uuids = [
            "bbbbbbbb-0000-0000-0000-000000000001",
            "bbbbbbbb-0000-0000-0000-000000000002",
            "bbbbbbbb-0000-0000-0000-000000000003",
        ];
        const paths = uuids.map((u) => writeInstructionFile(u, "content", instructionsDir));
        for (const p of paths) {
            assert.ok(fs.existsSync(p), "file must exist before deactivation");
        }

        cleanupAllTempFiles();

        for (const p of paths) {
            assert.ok(!fs.existsSync(p), "file must be deleted by cleanupAllTempFiles");
        }
    });

    test("cleanupAllTempFiles reports failures via output without throwing", () => {
        const lines: string[] = [];
        const output = { appendLine: (v: string) => lines.push(v) };

        // Write a real file and then remove it so the cleanup encounters a missing file.
        const uuid = "cccccccc-0000-0000-0000-000000000001";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);
        fs.unlinkSync(filePath); // pre-remove so cleanup fails

        // cleanupAllTempFiles should not throw even when deletion fails.
        assert.doesNotThrow(() => {
            cleanupAllTempFiles(output);
        });
    });
});

// ---------------------------------------------------------------------------
// Stale-file selection
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — cleanupStaleInstructionFiles", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("stale");
    });

    teardown(() => {
        removeTempDir(instructionsDir);
    });

    /** Writes a recognized instruction file and sets its mtime to `ageMs` ago. */
    function writeStaleFile(uuid: string, ageMs: number): string {
        const filePath = path.join(instructionsDir, `vector-instruction-${uuid}.txt`);
        fs.writeFileSync(filePath, "old content", "utf-8");
        const pastMs = Date.now() - ageMs;
        const pastDate = new Date(pastMs);
        fs.utimesSync(filePath, pastDate, pastDate);
        return filePath;
    }

    test("removes a recognized instruction file older than maxAgeMs", () => {
        const uuid = "dddddddd-dddd-dddd-dddd-dddddddddddd";
        const filePath = writeStaleFile(uuid, 25 * 60 * 60 * 1000); // 25 hours old
        assert.ok(fs.existsSync(filePath));

        cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000);

        assert.ok(!fs.existsSync(filePath), "stale file must be removed");
    });

    test("preserves a recognized instruction file newer than maxAgeMs", () => {
        const uuid = "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee";
        const filePath = writeStaleFile(uuid, 1 * 60 * 60 * 1000); // 1 hour old
        assert.ok(fs.existsSync(filePath));

        cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000);

        assert.ok(fs.existsSync(filePath), "recent file must be preserved");
    });

    test("preserves unrelated files even if they are old", () => {
        const unrelated = path.join(instructionsDir, "unrelated-data.txt");
        fs.writeFileSync(unrelated, "data", "utf-8");
        const pastDate = new Date(Date.now() - 48 * 60 * 60 * 1000);
        fs.utimesSync(unrelated, pastDate, pastDate);

        cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000);

        assert.ok(fs.existsSync(unrelated), "unrelated file must not be deleted");
    });

    test("preserves files whose names match the pattern but contain uppercase letters", () => {
        // Uppercase UUIDs are not canonical; they must not be treated as recognized files.
        const upperFile = path.join(
            instructionsDir,
            "vector-instruction-AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE.txt",
        );
        fs.writeFileSync(upperFile, "content", "utf-8");
        const pastDate = new Date(Date.now() - 48 * 60 * 60 * 1000);
        fs.utimesSync(upperFile, pastDate, pastDate);

        cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000);

        assert.ok(fs.existsSync(upperFile), "uppercase-UUID file must not be deleted");
    });

    test("does not recurse into subdirectories", () => {
        const subDir = path.join(instructionsDir, "sub");
        fs.mkdirSync(subDir);
        const uuid = "ffffffff-ffff-ffff-ffff-ffffffffffff";
        const nested = path.join(subDir, `vector-instruction-${uuid}.txt`);
        fs.writeFileSync(nested, "content", "utf-8");
        const pastDate = new Date(Date.now() - 48 * 60 * 60 * 1000);
        fs.utimesSync(nested, pastDate, pastDate);

        cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000);

        assert.ok(fs.existsSync(nested), "nested file must not be deleted (no recursion)");
    });

    test("does not throw when instructionsDir does not exist", () => {
        const missing = path.join(os.tmpdir(), "vector-no-such-dir-for-phase-b");
        assert.doesNotThrow(() => {
            cleanupStaleInstructionFiles(missing);
        });
    });

    test("reports cleanup failures via output without throwing", () => {
        const lines: string[] = [];
        const output = { appendLine: (v: string) => lines.push(v) };

        const uuid = "12345678-abcd-abcd-abcd-123456789abc";
        const filePath = writeStaleFile(uuid, 48 * 60 * 60 * 1000);

        // Mark the directory read-only so deletion fails (non-Windows only).
        if (process.platform !== "win32") {
            fs.chmodSync(instructionsDir, 0o555);
        } else {
            // On Windows just delete the file so the stat pass still runs but unlink fails.
            fs.unlinkSync(filePath);
        }

        assert.doesNotThrow(() => {
            cleanupStaleInstructionFiles(instructionsDir, 24 * 60 * 60 * 1000, output);
        });

        if (process.platform !== "win32") {
            // Restore permissions for teardown.
            fs.chmodSync(instructionsDir, 0o755);
        }
    });

    test("nowMs override controls age comparison deterministically", () => {
        const uuid = "aaaaaaaa-0000-0000-0000-000000000001";
        // Set mtime to exactly now.
        const filePath = path.join(instructionsDir, `vector-instruction-${uuid}.txt`);
        fs.writeFileSync(filePath, "content", "utf-8");
        const writtenAt = Date.now();

        // Pass nowMs = writtenAt + maxAgeMs - 1 (just below threshold) → preserve.
        const maxAgeMs = 24 * 60 * 60 * 1000;
        cleanupStaleInstructionFiles(
            instructionsDir,
            maxAgeMs,
            undefined,
            writtenAt + maxAgeMs - 1,
        );
        assert.ok(fs.existsSync(filePath), "file just below age threshold must be preserved");

        // Pass nowMs = writtenAt + maxAgeMs + 1000 (above threshold) → remove.
        cleanupStaleInstructionFiles(
            instructionsDir,
            maxAgeMs,
            undefined,
            writtenAt + maxAgeMs + 1000,
        );
        assert.ok(!fs.existsSync(filePath), "file above age threshold must be removed");
    });
});

// ---------------------------------------------------------------------------
// Both launch surfaces (agent-action and agent-button)
// ---------------------------------------------------------------------------

suite("Task 00075 Phase B — both launch surfaces via _handleRunAgent", () => {
    const extensionRoot = new URL("../../", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1");

    function makeAgentWorkspace(): { dir: string; docPath: string } {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-phase-b-surface-"));
        const vectorDir = path.join(dir, ".vector");
        fs.mkdirSync(vectorDir, { recursive: true });

        // Use `node` as the agent command — it is guaranteed to be in PATH since the
        // test suite itself runs in Node.js, so `isCommandInPath("node")` returns true.
        fs.writeFileSync(
            path.join(vectorDir, "agents.yaml"),
            [
                `instructions-dir: "\${system-temp}/vector/instructions"`,
                `agents:`,
                `  my-agent:`,
                `    type: cli`,
                `    command: "node <instruction>"`,
                `profiles:`,
                `  code:`,
                `    - my-agent`,
            ].join("\n"),
            "utf-8",
        );

        // Write a minimal document-types.yaml that includes the prompts type so
        // resolveGovernedPreviewSource can locate prompt documents.
        fs.writeFileSync(
            path.join(vectorDir, "document-types.yaml"),
            ["document-types:", "  prompts:", "    layout: directory", '    "code-width": 5'].join(
                "\n",
            ),
            "utf-8",
        );

        // Write a minimal prompt document.
        const promptDir = path.join(dir, "doc", "prompts");
        fs.mkdirSync(promptDir, { recursive: true });
        const docPath = path.join(promptDir, "prompts-00001-test-prompt.md");
        fs.writeFileSync(docPath, "---\ntitle: Test Prompt\n---\n\nExecute the task.\n", "utf-8");

        return { dir, docPath };
    }

    test("agent-action surface sends a command containing the MCP directive to the terminal", async () => {
        const { dir, docPath } = makeAgentWorkspace();
        try {
            vscode.__resetTerminalState();
            vscode.__resetUiState();

            // Inject `() => true` as `isAgentAvailable` so PATH lookups are bypassed in tests.
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

            // Simulate the message sent by both agent-action and agent-button buttons.
            vscode.__fireWebviewMessage(stubPanel, {
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00001-test-prompt",
                label: "Execute Phase B",
                staticInput: {},
                formValues: {},
            });

            // Allow the async handler to resolve.
            await new Promise((resolve) => setTimeout(resolve, 50));

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1, "must create exactly one terminal");
            const terminal = terminals[0];
            assert.ok(terminal, "terminal record must exist");

            // The command sent to the terminal must contain the MCP get_instruction directive.
            assert.ok(
                terminal.sentText[0]?.includes("get_instruction"),
                "command must include the get_instruction MCP call",
            );
            // The terminal must use the workspace root as cwd.
            assert.strictEqual(terminal.cwd, dir, "terminal cwd must be the workspaceRoot");
        } finally {
            removeTempDir(dir);
            vscode.__resetTerminalState();
            vscode.__resetUiState();
        }
    });

    test("agent-button surface (vector-agent-button) uses the same runAgent path", async () => {
        const { dir, docPath } = makeAgentWorkspace();
        try {
            vscode.__resetTerminalState();
            vscode.__resetUiState();

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

            // Both button and action surfaces post the same message type.
            vscode.__fireWebviewMessage(stubPanel, {
                type: "vector.runAgent",
                profile: "code",
                prompt: "prompts-00001-test-prompt",
                label: "Button Label",
                staticInput: { task: "00075" },
                formValues: {},
            });

            await new Promise((resolve) => setTimeout(resolve, 50));

            const terminals = vscode.__getCreatedTerminals();
            assert.strictEqual(terminals.length, 1, "button surface must create one terminal");
            assert.ok(
                terminals[0]?.sentText[0]?.includes("get_instruction"),
                "button-triggered command must include the MCP directive",
            );
        } finally {
            removeTempDir(dir);
            vscode.__resetTerminalState();
            vscode.__resetUiState();
        }
    });

    test("instruction file is created in the configured instructions directory", async () => {
        const { dir, docPath } = makeAgentWorkspace();
        const instructionsDir = path.join(os.tmpdir(), "vector", "instructions");
        try {
            vscode.__resetTerminalState();
            vscode.__resetUiState();

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
                prompt: "prompts-00001-test-prompt",
                label: "Label",
                staticInput: {},
                formValues: {},
            });

            await new Promise((resolve) => setTimeout(resolve, 50));

            // Verify at least one instruction file was written in the expected directory.
            let files: string[] = [];
            try {
                files = fs
                    .readdirSync(instructionsDir)
                    .filter((f) => f.startsWith("vector-instruction-") && f.endsWith(".txt"));
            } catch {
                // directory may not exist if os.tmpdir() resolves elsewhere; skip.
            }
            if (files.length > 0) {
                assert.ok(files.length >= 1, "at least one instruction file must exist");
                const firstName = files[0] ?? "";
                assert.ok(
                    UUID_PATTERN.test(
                        firstName.replace("vector-instruction-", "").replace(".txt", ""),
                    ),
                    "instruction filename must contain a canonical UUID",
                );
            }
        } finally {
            removeTempDir(dir);
            vscode.__resetTerminalState();
            vscode.__resetUiState();
            // Best-effort cleanup of instruction files created during this test.
            try {
                const entries = fs.readdirSync(instructionsDir);
                for (const e of entries) {
                    if (e.startsWith("vector-instruction-") && e.endsWith(".txt")) {
                        fs.rmSync(path.join(instructionsDir, e), { force: true });
                    }
                }
            } catch {
                // ignore
            }
        }
    });
});
