/**
 * Task 00076 Phase A — Configuration and Prompt Resolution
 *
 * Tests for:
 *   - `DEFAULT_AGENT_PROMPT`: wording contract (Vector MCP, no "server", has placeholder)
 *   - `resolveAgentPrompt`: fallback to default, configured override, placeholder substitution,
 *     configured prompts without the placeholder, multiple occurrences of the placeholder
 *   - `loadAgentsConfig`: valid `prompt` field, absent `prompt` field, non-string `prompt` error
 */

import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import {
    DEFAULT_AGENT_PROMPT,
    resolveAgentPrompt,
} from "../document-viewer/document-actions/agentExecutor.js";
import { loadAgentsConfig } from "../document-viewer/document-actions/agentsConfig.js";

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Creates an isolated workspace directory and returns its path with a cleanup callback. */
function makeTempWorkspace(label: string): { dir: string; cleanup: () => void } {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `vector-task76-${label}-`));
    return {
        dir,
        cleanup: () => {
            fs.rmSync(dir, { recursive: true, force: true });
        },
    };
}

/** Writes a minimal `.vector/agents.yaml` with optional extra lines appended at the root. */
function writeAgentsYaml(workspaceDir: string, extraLines: string[] = []): void {
    const vectorDir = path.join(workspaceDir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    const lines = [
        `instructions-dir: "\${system-temp}/vector/instructions"`,
        `agents:`,
        `  claude:`,
        `    type: cli`,
        `    command: "claude <instruction>"`,
        `profiles:`,
        `  code:`,
        `    - claude`,
        ...extraLines,
    ];
    fs.writeFileSync(path.join(vectorDir, "agents.yaml"), lines.join("\n"), "utf-8");
}

// ── DEFAULT_AGENT_PROMPT wording ──────────────────────────────────────────────

suite("Task 00076 Phase A — DEFAULT_AGENT_PROMPT wording contract", () => {
    test("contains 'vector mcp'", () => {
        assert.ok(
            DEFAULT_AGENT_PROMPT.toLowerCase().includes("vector mcp"),
            "must reference vector mcp",
        );
    });

    test("does not contain the word 'server'", () => {
        assert.ok(!DEFAULT_AGENT_PROMPT.includes("server"), "must not contain the word 'server'");
    });

    test("contains the <instruction-id> placeholder", () => {
        assert.ok(
            DEFAULT_AGENT_PROMPT.includes("<instruction-id>"),
            "must contain the <instruction-id> placeholder",
        );
    });

    test("references instruction", () => {
        assert.ok(
            DEFAULT_AGENT_PROMPT.includes("instruction"),
            "must reference instruction",
        );
    });
});

// ── resolveAgentPrompt ────────────────────────────────────────────────────────

suite("Task 00076 Phase A — resolveAgentPrompt: default fallback", () => {
    test("uses DEFAULT_AGENT_PROMPT when configuredPrompt is undefined", () => {
        const uuid = "11111111-2222-3333-4444-555555555555";
        const result = resolveAgentPrompt(undefined, uuid);
        const expected = DEFAULT_AGENT_PROMPT.replaceAll("<instruction-id>", uuid);
        assert.strictEqual(result, expected);
    });

    test("embeds the UUID in place of <instruction-id> in the default prompt", () => {
        const uuid = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        const result = resolveAgentPrompt(undefined, uuid);
        assert.ok(result.includes(uuid), "resolved default prompt must contain the UUID");
        assert.ok(
            !result.includes("<instruction-id>"),
            "resolved default prompt must not contain the raw placeholder",
        );
    });

    test("resolved default prompt contains 'vector mcp' and not 'server'", () => {
        const uuid = "00000000-0000-0000-0000-000000000000";
        const result = resolveAgentPrompt(undefined, uuid);
        assert.ok(
            result.toLowerCase().includes("vector mcp"),
            "resolved prompt must reference vector mcp",
        );
        assert.ok(!result.includes("server"), "resolved prompt must not contain 'server'");
    });

    test("result is deterministic for the same UUID", () => {
        const uuid = "12345678-1234-1234-1234-123456789012";
        assert.strictEqual(
            resolveAgentPrompt(undefined, uuid),
            resolveAgentPrompt(undefined, uuid),
        );
    });
});

suite("Task 00076 Phase A — resolveAgentPrompt: configured prompt override", () => {
    test("uses the configured prompt when provided, not the default", () => {
        const uuid = "cccccccc-dddd-eeee-ffff-aaaaaaaaaaaa";
        const configured = "Custom prompt: use id <instruction-id> now";
        const result = resolveAgentPrompt(configured, uuid);
        assert.ok(result.includes("Custom prompt"), "must use the configured prompt text");
        assert.ok(!result.includes("Vector MCP"), "must not fall back to the default wording");
    });

    test("substitutes all <instruction-id> occurrences in a configured prompt", () => {
        const uuid = "12345678-abcd-ef01-2345-6789abcdef01";
        const configured = "id=<instruction-id> and again <instruction-id>";
        const result = resolveAgentPrompt(configured, uuid);
        assert.strictEqual(result, `id=${uuid} and again ${uuid}`);
    });

    test("preserves configured prompt text exactly when there is no placeholder", () => {
        const uuid = "ffffffff-0000-1111-2222-333333333333";
        const configured = "Just run the task without any placeholder";
        const result = resolveAgentPrompt(configured, uuid);
        assert.strictEqual(
            result,
            configured,
            "prompt without placeholder must be returned verbatim",
        );
    });

    test("preserves surrounding configured text around the substituted placeholder", () => {
        const uuid = "abcdef12-3456-7890-abcd-ef1234567890";
        const configured = "prefix <instruction-id> suffix";
        const result = resolveAgentPrompt(configured, uuid);
        assert.strictEqual(result, `prefix ${uuid} suffix`);
    });
});

// ── loadAgentsConfig: prompt field validation ─────────────────────────────────

suite("Task 00076 Phase A — loadAgentsConfig: prompt field", () => {
    test("succeeds and exposes prompt when a valid string prompt is configured", () => {
        const { dir, cleanup } = makeTempWorkspace("valid-prompt");
        try {
            writeAgentsYaml(dir, [`prompt: "Run <instruction-id> in the agent"`]);
            const result = loadAgentsConfig(dir);
            if (!result.ok) {
                assert.fail(
                    `config must load successfully: ${result.missing ? "file missing" : result.error}`,
                );
                return;
            }
            assert.strictEqual(result.config.prompt, "Run <instruction-id> in the agent");
        } finally {
            cleanup();
        }
    });

    test("succeeds with prompt absent and exposes prompt as undefined", () => {
        const { dir, cleanup } = makeTempWorkspace("no-prompt");
        try {
            writeAgentsYaml(dir);
            const result = loadAgentsConfig(dir);
            if (!result.ok) {
                assert.fail(
                    `config must load successfully: ${result.missing ? "file missing" : result.error}`,
                );
                return;
            }
            assert.strictEqual(result.config.prompt, undefined);
        } finally {
            cleanup();
        }
    });

    test("returns a configuration error when prompt is a non-string (number)", () => {
        const { dir, cleanup } = makeTempWorkspace("invalid-prompt-number");
        try {
            writeAgentsYaml(dir, [`prompt: 42`]);
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok, "config load must fail for a non-string prompt");
            assert.ok(!result.missing, "result must not be treated as missing");
            assert.ok(
                result.error.includes("prompt"),
                `error message must reference 'prompt', got: ${result.error}`,
            );
            assert.ok(
                result.error.includes("string"),
                `error message must mention 'string', got: ${result.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("returns a configuration error when prompt is a non-string (boolean)", () => {
        const { dir, cleanup } = makeTempWorkspace("invalid-prompt-bool");
        try {
            writeAgentsYaml(dir, [`prompt: true`]);
            const result = loadAgentsConfig(dir);
            assert.ok(!result.ok, "config load must fail for a boolean prompt");
            assert.ok(!result.missing, "result must not be treated as missing");
            assert.ok(
                result.error.includes("prompt"),
                `error must mention 'prompt': ${result.error}`,
            );
        } finally {
            cleanup();
        }
    });
});
