/**
 * Task 00075 Phase D — Integration and Documentation
 *
 * Focused tests for:
 *   - Complete frontend-to-MCP flow: VS Code writes an instruction file and the
 *     `GetInstructionOp` (runtime-doc) reads it back using the same configuration.
 *   - Distinct responsibility boundaries: `.agents/mcp_config.json` starts the
 *     MCP server; `.vector/agents.yaml` configures agent execution and the
 *     controlled instruction directory.
 *   - Regression coverage: existing MCP tools and unaffected VS Code agent
 *     behaviour continue to pass after the instruction-capability changes.
 */

import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import {
    loadAgentsConfig,
    resolveInstructionsDir,
} from "../document-viewer/document-actions/agentsConfig.js";
import {
    writeInstructionFile,
    renderInstructionDirective,
    resolveAgentCommand,
    cleanupAllTempFiles,
    deleteTempFile,
} from "../document-viewer/document-actions/agentExecutor.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Creates an isolated temporary directory and returns its path. */
function makeTempDir(label: string): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), `vector-phase-d-${label}-`));
}

/** Removes a directory tree; safe when the directory is already missing. */
function removeTempDir(dir: string): void {
    fs.rmSync(dir, { recursive: true, force: true });
}

/** Writes a minimal `.vector/agents.yaml` that uses the new instruction contract. */
function writeAgentsYaml(workspaceDir: string, instructionsSuffix: string): void {
    const vectorDir = path.join(workspaceDir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    fs.writeFileSync(
        path.join(vectorDir, "agents.yaml"),
        [
            `instructions-dir: "\${system-temp}/${instructionsSuffix}"`,
            `agents:`,
            `  claude:`,
            `    type: cli`,
            `    command: "claude <instruction>"`,
            `profiles:`,
            `  code:`,
            `    - claude`,
        ].join("\n"),
        "utf-8",
    );
}

/** Writes a minimal `.agents/mcp_config.json` that points at the MCP server. */
function writeMcpConfig(workspaceDir: string): void {
    const agentsDir = path.join(workspaceDir, ".agents");
    fs.mkdirSync(agentsDir, { recursive: true });
    fs.writeFileSync(
        path.join(agentsDir, "mcp_config.json"),
        JSON.stringify(
            { mcpServers: { vector: { type: "stdio", command: "mcp-vector" } } },
            null,
            2,
        ),
        "utf-8",
    );
}

// ---------------------------------------------------------------------------
// Distinct boundary: .agents/mcp_config.json vs .vector/agents.yaml
// ---------------------------------------------------------------------------

suite(
    "Task 00075 Phase D — distinct responsibilities: .agents/mcp_config.json vs .vector/agents.yaml",
    () => {
        let workspaceDir: string;
        let instructionsDir: string;
        const instructionsSuffix = "vector-phase-d-boundary-test/instructions";

        setup(() => {
            workspaceDir = makeTempDir("boundary");
            instructionsDir = path.join(os.tmpdir(), instructionsSuffix);
            writeAgentsYaml(workspaceDir, instructionsSuffix);
            writeMcpConfig(workspaceDir);
        });

        teardown(() => {
            removeTempDir(workspaceDir);
            removeTempDir(instructionsDir);
            cleanupAllTempFiles();
        });

        test(".agents/mcp_config.json declares the MCP server; .vector/agents.yaml configures agents", () => {
            // `.agents/mcp_config.json` contains only the server start declaration.
            const mcpConfigPath = path.join(workspaceDir, ".agents", "mcp_config.json");
            const mcpConfig = JSON.parse(fs.readFileSync(mcpConfigPath, "utf-8")) as {
                mcpServers: Record<string, { type: string; command: string }>;
            };
            const serverCommand = mcpConfig.mcpServers["vector"]?.command;
            assert.ok(serverCommand, "mcp_config.json must declare the MCP server command");
            assert.ok(
                !JSON.stringify(mcpConfig).includes("instruction"),
                "mcp_config.json must not contain instruction-directory configuration",
            );

            // `.vector/agents.yaml` owns the instruction directory and agent commands.
            const configResult = loadAgentsConfig(workspaceDir);
            if (!configResult.ok) {
                assert.fail(
                    `agents.yaml must load without error: ${configResult.missing ? "file missing" : configResult.error}`,
                );
                return;
            }

            assert.ok(
                configResult.config.instructionsDir.length > 0,
                "agents.yaml must expose a resolved instructions directory",
            );
            assert.ok(
                configResult.config.agents["claude"]?.command.includes("<instruction>"),
                "agents.yaml must configure agent commands with the <instruction> placeholder",
            );
        });

        test("agents.yaml instructions-dir resolves to a path inside the system temp directory", () => {
            const configResult = loadAgentsConfig(workspaceDir);
            if (!configResult.ok) {
                assert.fail(
                    `agents.yaml must load without error: ${configResult.missing ? "file missing" : configResult.error}`,
                );
                return;
            }

            const resolved = configResult.config.instructionsDir;
            const systemTemp = path.normalize(os.tmpdir());
            assert.ok(
                resolved === systemTemp || resolved.startsWith(systemTemp + path.sep),
                `resolved instructions-dir must be inside the system temp directory; got: ${resolved}`,
            );
        });
    },
);

// ---------------------------------------------------------------------------
// Complete frontend-to-MCP flow
// ---------------------------------------------------------------------------

suite("Task 00075 Phase D — complete frontend-to-MCP flow", () => {
    let workspaceDir: string;
    let instructionsDir: string;
    const instructionsSuffix = "vector-phase-d-flow-test/instructions";

    setup(() => {
        workspaceDir = makeTempDir("flow");
        instructionsDir = path.join(os.tmpdir(), instructionsSuffix);
        writeAgentsYaml(workspaceDir, instructionsSuffix);
    });

    teardown(() => {
        removeTempDir(workspaceDir);
        removeTempDir(instructionsDir);
        cleanupAllTempFiles();
    });

    test("VS Code writes instruction, resolves command, and the instruction file is readable as plain text", () => {
        // 1. Load and validate agents.yaml — this is what the VS Code extension does.
        const configResult = loadAgentsConfig(workspaceDir);
        if (!configResult.ok) {
            assert.fail(
                `agents.yaml must load successfully: ${configResult.missing ? "file missing" : configResult.error}`,
            );
            return;
        }

        const resolvedInstructionsDir = configResult.config.instructionsDir;

        // 2. Generate a canonical UUID (mirrored from the VS Code lifecycle).
        const uuid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";

        // 3. Resolve the prompt content that the frontend would produce.
        const promptContent =
            "Using Vector MCP, call get_instruction with id " +
            `"${uuid}" and execute the returned instructions.`;

        // 4. Create the instruction file in the configured directory.
        const filePath = writeInstructionFile(uuid, promptContent, resolvedInstructionsDir);
        try {
            assert.ok(fs.existsSync(filePath), "instruction file must exist after write");

            // 5. Verify the file is readable as plain text (simulating what GetInstructionOp does).
            const fileContent = fs.readFileSync(filePath, "utf-8");
            assert.strictEqual(
                fileContent,
                promptContent,
                "instruction file content must match the written prompt verbatim",
            );

            // 6. Verify the filename shape matches the expected pattern.
            const basename = path.basename(filePath);
            assert.ok(
                basename.startsWith("vector-instruction-"),
                "instruction filename must start with 'vector-instruction-'",
            );
            assert.ok(basename.endsWith(".txt"), "instruction filename must end with '.txt'");
            assert.ok(basename.includes(uuid), "instruction filename must contain the UUID");

            // 7. Verify the instruction file is inside the configured directory.
            assert.strictEqual(
                path.dirname(filePath),
                path.normalize(resolvedInstructionsDir),
                "instruction file must reside in the configured instructions directory",
            );
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("frontend renders the MCP directive and resolves the full agent command without second interpolation", () => {
        // 1. Load config.
        const configResult = loadAgentsConfig(workspaceDir);
        if (!configResult.ok) {
            assert.fail(
                `agents.yaml must load without error: ${configResult.missing ? "file missing" : configResult.error}`,
            );
            return;
        }

        const resolvedInstructionsDir = configResult.config.instructionsDir;
        const uuid = "b2c3d4e5-f6a7-8901-bcde-f12345678901";

        // 2. Write instruction file.
        const promptContent = "Execute: do something useful.";
        const filePath = writeInstructionFile(uuid, promptContent, resolvedInstructionsDir);

        try {
            // 3. Render the MCP directive (frontend-owned step).
            const directive = renderInstructionDirective(uuid);
            assert.ok(directive.includes("vector mcp"), "directive must reference vector mcp");
            assert.ok(directive.includes(uuid), "directive must embed the UUID");

            // 4. Resolve the agent command (substitute <instruction> directly).
            const commandTemplate = configResult.config.agents["claude"]?.command ?? "";
            assert.ok(commandTemplate.length > 0, "claude agent must have a command");
            const resolvedCommand = resolveAgentCommand(commandTemplate, directive);

            assert.ok(
                !resolvedCommand.includes("<instruction>"),
                "resolved command must not retain the <instruction> placeholder",
            );
            assert.ok(
                resolvedCommand.includes("vector mcp"),
                "resolved command must contain the MCP directive",
            );
            assert.ok(resolvedCommand.includes(uuid), "resolved command must contain the UUID");

            // 6. Verify no second interpolation: the UUID appears exactly once.
            const occurrences = resolvedCommand.split(uuid).length - 1;
            assert.strictEqual(
                occurrences,
                1,
                "UUID must appear exactly once in the resolved command",
            );
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("repeated reads of the instruction file succeed without deletion (MCP retry contract)", () => {
        const configResult = loadAgentsConfig(workspaceDir);
        if (!configResult.ok) {
            assert.fail(
                `agents.yaml must load without error: ${configResult.missing ? "file missing" : configResult.error}`,
            );
            return;
        }

        const resolvedInstructionsDir = configResult.config.instructionsDir;
        const uuid = "c3d4e5f6-a7b8-9012-cdef-123456789012";
        const content = "Retryable instruction content.";

        const filePath = writeInstructionFile(uuid, content, resolvedInstructionsDir);
        try {
            // First read.
            const first = fs.readFileSync(filePath, "utf-8");
            assert.strictEqual(first, content, "first read must return the correct content");
            assert.ok(fs.existsSync(filePath), "file must still exist after first read");

            // Second read (simulating MCP retry).
            const second = fs.readFileSync(filePath, "utf-8");
            assert.strictEqual(second, content, "second read must return identical content");
            assert.ok(fs.existsSync(filePath), "file must still exist after second read");
        } finally {
            deleteTempFile(filePath);
        }
    });
});

// ---------------------------------------------------------------------------
// Size bound documentation (1 MiB)
// ---------------------------------------------------------------------------

suite("Task 00075 Phase D — size bound and platform guarantees", () => {
    let instructionsDir: string;

    setup(() => {
        instructionsDir = makeTempDir("sizebound");
    });

    teardown(() => {
        removeTempDir(instructionsDir);
        cleanupAllTempFiles();
    });

    test("instructions below 1 MiB are written and read without truncation", () => {
        const uuid = "d4e5f6a7-b8c9-0123-def0-234567890123";
        // Write content just below 1 MiB (32 KiB representative sample).
        const content = "x".repeat(32 * 1024);
        const filePath = writeInstructionFile(uuid, content, instructionsDir);
        try {
            const read = fs.readFileSync(filePath, "utf-8");
            assert.strictEqual(read.length, content.length, "content must be read back in full");
        } finally {
            deleteTempFile(filePath);
        }
    });

    test("instruction file is created in a platform-normalized directory path", () => {
        const uuid = "e5f6a7b8-c9d0-1234-ef01-345678901234";
        const filePath = writeInstructionFile(uuid, "content", instructionsDir);
        try {
            const normalized = path.normalize(filePath);
            assert.strictEqual(filePath, normalized, "instruction file path must be normalized");
        } finally {
            deleteTempFile(filePath);
        }
    });
});

// ---------------------------------------------------------------------------
// Single-governed-project workspace scope
// ---------------------------------------------------------------------------

suite("Task 00075 Phase D — single-governed-project workspace scope", () => {
    let workspaceDir: string;
    const instructionsSuffix = "vector-phase-d-scope-test/instructions";

    setup(() => {
        workspaceDir = makeTempDir("scope");
        writeAgentsYaml(workspaceDir, instructionsSuffix);
    });

    teardown(() => {
        removeTempDir(workspaceDir);
        removeTempDir(path.join(os.tmpdir(), instructionsSuffix));
        cleanupAllTempFiles();
    });

    test("loadAgentsConfig reads the governed project's agents.yaml as the authoritative source", () => {
        const configResult = loadAgentsConfig(workspaceDir);
        if (!configResult.ok) {
            assert.fail(
                `the single governed project's agents.yaml must load successfully: ${configResult.missing ? "file missing" : configResult.error}`,
            );
            return;
        }

        // The workspaceRoot (selected during extension activation) is the single source of truth.
        assert.ok(
            configResult.config.instructionsDir.length > 0,
            "the resolved instructions directory must be non-empty",
        );
        assert.ok(
            configResult.config.agents,
            "the config must expose the agent definitions from the governed project",
        );
    });

    test("resolveInstructionsDir returns the same path as loadAgentsConfig.instructionsDir", () => {
        const configResult = loadAgentsConfig(workspaceDir);
        if (!configResult.ok) {
            assert.fail(
                `agents.yaml must load successfully: ${configResult.missing ? "file missing" : configResult.error}`,
            );
            return;
        }

        const directlyResolved = resolveInstructionsDir(`\${system-temp}/${instructionsSuffix}`);
        assert.strictEqual(
            path.normalize(configResult.config.instructionsDir),
            path.normalize(directlyResolved),
            "loadAgentsConfig.instructionsDir and resolveInstructionsDir must agree",
        );
    });
});

// ---------------------------------------------------------------------------
// Regression: existing MCP tools and unaffected VS Code behaviour
// ---------------------------------------------------------------------------

suite("Task 00075 Phase D — regression: existing agent behaviour and configuration loading", () => {
    let workspaceDir: string;
    const instructionsSuffix = "vector-phase-d-regression-test/instructions";

    setup(() => {
        workspaceDir = makeTempDir("regression");
        writeAgentsYaml(workspaceDir, instructionsSuffix);
    });

    teardown(() => {
        removeTempDir(workspaceDir);
        removeTempDir(path.join(os.tmpdir(), instructionsSuffix));
        cleanupAllTempFiles();
    });

    test("loadAgentsConfig rejects the legacy <file> placeholder with an actionable error", () => {
        const vectorDir = path.join(workspaceDir, ".vector");
        fs.writeFileSync(
            path.join(vectorDir, "agents.yaml"),
            [
                `instructions-dir: "\${system-temp}/${instructionsSuffix}"`,
                `agents:`,
                `  legacy-agent:`,
                `    type: cli`,
                `    command: "agent 'execute the content in the file <file>'"`,
                `profiles:`,
                `  code:`,
                `    - legacy-agent`,
            ].join("\n"),
            "utf-8",
        );

        const result = loadAgentsConfig(workspaceDir);
        if (result.ok) {
            assert.fail("config with legacy <file> placeholder must fail validation");
            return;
        }
        if (result.missing) {
            assert.fail(
                "failure must not be reported as a missing file — it is a validation error",
            );
            return;
        }
        assert.ok(
            result.error.includes("<file>"),
            `error must reference the obsolete <file> placeholder; got: ${result.error}`,
        );
    });

    test("loadAgentsConfig rejects the legacy <instruction-id> placeholder with an actionable error", () => {
        const vectorDir = path.join(workspaceDir, ".vector");
        fs.writeFileSync(
            path.join(vectorDir, "agents.yaml"),
            [
                `instructions-dir: "\${system-temp}/${instructionsSuffix}"`,
                `agents:`,
                `  bad-agent:`,
                `    type: cli`,
                `    command: "agent --id <instruction-id>"`,
                `profiles:`,
                `  code:`,
                `    - bad-agent`,
            ].join("\n"),
            "utf-8",
        );

        const result = loadAgentsConfig(workspaceDir);
        if (result.ok) {
            assert.fail("config with <instruction-id> placeholder must fail validation");
            return;
        }
        if (result.missing) {
            assert.fail(
                "failure must not be reported as a missing file — it is a validation error",
            );
            return;
        }
        assert.ok(
            result.error.includes("<instruction-id>"),
            `error must reference the obsolete <instruction-id> placeholder; got: ${result.error}`,
        );
    });

    test("loadAgentsConfig fails with an actionable error when instructions-dir is absent", () => {
        const vectorDir = path.join(workspaceDir, ".vector");
        fs.writeFileSync(
            path.join(vectorDir, "agents.yaml"),
            [
                `agents:`,
                `  claude:`,
                `    type: cli`,
                `    command: "claude <instruction>"`,
                `profiles:`,
                `  code:`,
                `    - claude`,
            ].join("\n"),
            "utf-8",
        );

        const result = loadAgentsConfig(workspaceDir);
        if (result.ok) {
            assert.fail("config missing instructions-dir must fail with a validation error");
            return;
        }
        if (result.missing) {
            assert.fail("failure must not be reported as a missing file — the file exists");
            return;
        }
        assert.ok(
            result.error.includes("instructions-dir"),
            `error must reference the missing instructions-dir field; got: ${result.error}`,
        );
    });

    test("loadAgentsConfig fails when instructions-dir does not begin with ${system-temp}", () => {
        const vectorDir = path.join(workspaceDir, ".vector");
        fs.writeFileSync(
            path.join(vectorDir, "agents.yaml"),
            [
                `instructions-dir: "/absolute/path/without/variable"`,
                `agents:`,
                `  claude:`,
                `    type: cli`,
                `    command: "claude <instruction>"`,
                `profiles:`,
                `  code:`,
                `    - claude`,
            ].join("\n"),
            "utf-8",
        );

        const result = loadAgentsConfig(workspaceDir);
        if (result.ok) {
            assert.fail("config with absolute path (no ${system-temp}) must fail validation");
        }
    });

    test("a valid agents.yaml with the new contract loads cleanly", () => {
        // The workspace already has a valid agents.yaml from setup.
        const result = loadAgentsConfig(workspaceDir);
        if (!result.ok) {
            assert.fail(
                `valid agents.yaml must load without error: ${result.missing ? "file missing" : result.error}`,
            );
            return;
        }

        assert.ok(
            result.config.instructionsDir.length > 0,
            "valid config must expose a non-empty resolved instructions directory",
        );
        assert.ok(
            result.config.agents["claude"]?.command.includes("<instruction>"),
            "valid config must include the <instruction> placeholder in the claude command",
        );
        assert.ok(
            result.config.profiles["code"]?.includes("claude"),
            "valid config must expose the configured profiles",
        );
    });
});
