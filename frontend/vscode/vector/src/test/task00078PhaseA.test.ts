/**
 * Task 00078 Phase A — Configuration and Resolution Tests
 *
 * Tests for RFC 00044:
 *   - Loading optional .vector/actions.yaml and root validation
 *   - Independent entry validation and per-entry diagnostics
 *   - Fallback behavior for absent file or unconfigured action IDs
 *   - Strict failure without fallback for invalid configured action IDs
 *   - Profile resolution via .vector/agents.yaml
 *   - Governed prompt resolution
 *   - Variable precedence (configured input < action/form values < extension-generated)
 *   - Exclusion of profile and prompt from merged variables
 *   - Path normalization across platforms
 *   - Single-pass interpolation and unresolved variable rejection
 *   - Observability contract (no sensitive values in diagnostics)
 */

import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import {
    loadActionsConfig,
    mergeActionVariables,
    normalizeDocPath,
    interpolateActionPrompt,
    resolveConfiguredAction,
    ACTIONS_YAML_DISPLAY_PATH,
} from "../document-viewer/document-actions/actionsConfig.js";
import type { DiagnosticOutput } from "../document-viewer/document-actions/agentExecutor.js";

// ── Test Helpers ─────────────────────────────────────────────────────────────

function makeTempWorkspace(label: string): { dir: string; cleanup: () => void } {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), `vector-task78-${label}-`));
    return {
        dir,
        cleanup: () => {
            fs.rmSync(dir, { recursive: true, force: true });
        },
    };
}

function writeActionsYaml(workspaceDir: string, content: string): void {
    const vectorDir = path.join(workspaceDir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    fs.writeFileSync(path.join(vectorDir, "actions.yaml"), content, "utf-8");
}

function writeAgentsYaml(workspaceDir: string): void {
    const vectorDir = path.join(workspaceDir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    const content = [
        `instructions-dir: "\${system-temp}/vector/instructions"`,
        `prompt: "custom agent prompt <instruction-id>"`,
        `agents:`,
        `  claude:`,
        `    type: cli`,
        `    command: "claude <instruction>"`,
        `profiles:`,
        `  code:`,
        `    - claude`,
        `  create-doc:`,
        `    - claude`,
    ].join("\n");
    fs.writeFileSync(path.join(vectorDir, "agents.yaml"), content, "utf-8");
}

class TestOutput implements DiagnosticOutput {
    public lines: string[] = [];
    appendLine(value: string): void {
        this.lines.push(value);
    }
}

// ── Suite 1: File Loading and Root Validation ─────────────────────────────────

suite("Task 00078 Phase A — Loading and Root Schema Validation", () => {
    test("absent .vector/actions.yaml returns ok: false, missing: true", () => {
        const { dir, cleanup } = makeTempWorkspace("missing");
        try {
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            assert.strictEqual((result as { missing: boolean }).missing, true);
        } finally {
            cleanup();
        }
    });

    test("malformed YAML returns actionable parse error", () => {
        const { dir, cleanup } = makeTempWorkspace("malformed");
        try {
            writeActionsYaml(dir, "actions: [invalid, yaml: :");
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            const errRes = result as { missing: boolean; error: string };
            assert.strictEqual(errRes.missing, false);
            assert.ok(
                errRes.error.includes("YAML parse error"),
                `error should mention parse error: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("non-mapping root is rejected", () => {
        const { dir, cleanup } = makeTempWorkspace("non-map");
        try {
            writeActionsYaml(dir, "- item1\n- item2");
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            const errRes = result as { missing: boolean; error: string };
            assert.strictEqual(errRes.missing, false);
            assert.ok(
                errRes.error.includes("must be a YAML mapping"),
                `expected mapping error: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("unknown root fields are rejected to guard schema evolution", () => {
        const { dir, cleanup } = makeTempWorkspace("unknown-root");
        try {
            writeActionsYaml(
                dir,
                "version: 1\nactions:\n  create-doc:\n    profile: code\n    prompt: p1",
            );
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            const errRes = result as { missing: boolean; error: string };
            assert.strictEqual(errRes.missing, false);
            assert.ok(
                errRes.error.includes("unknown field 'version'"),
                `expected unknown field error: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("missing actions mapping is rejected", () => {
        const { dir, cleanup } = makeTempWorkspace("missing-actions");
        try {
            writeActionsYaml(dir, "{}\n");
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            const errRes = result as { missing: boolean; error: string };
            assert.strictEqual(errRes.missing, false);
            assert.ok(
                errRes.error.includes("'actions' is required"),
                `expected actions required: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("actions field that is not a mapping is rejected", () => {
        const { dir, cleanup } = makeTempWorkspace("actions-not-map");
        try {
            writeActionsYaml(dir, "actions: [one, two]");
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, false);
            const errRes = result as { missing: boolean; error: string };
            assert.strictEqual(errRes.missing, false);
            assert.ok(
                errRes.error.includes("'actions' must be a mapping"),
                `expected actions must be mapping: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });
});

// ── Suite 2: Independent Entry Validation ─────────────────────────────────────

suite("Task 00078 Phase A — Independent Entry Validation", () => {
    test("valid entries are loaded while invalid entries are tracked independently", () => {
        const { dir, cleanup } = makeTempWorkspace("independent-validation");
        try {
            const yaml = [
                `actions:`,
                `  valid-action:`,
                `    profile: code`,
                `    prompt: prompts-00001-test`,
                `    input:`,
                `      doc-type: rfc`,
                `  invalid-profile:`,
                `    profile: null`,
                `    prompt: prompts-00002-test`,
                `  missing-prompt:`,
                `    profile: code`,
                `  unknown-setting:`,
                `    profile: code`,
                `    prompt: prompts-00003-test`,
                `    extra-field: unexpected`,
                `  invalid-input:`,
                `    profile: code`,
                `    prompt: prompts-00004-test`,
                `    input: [not, a, map]`,
            ].join("\n");
            writeActionsYaml(dir, yaml);
            const output = new TestOutput();
            const result = loadActionsConfig(dir, output);

            assert.strictEqual(result.ok, true);
            const okRes = result;

            // valid-action remains usable
            assert.ok("valid-action" in okRes.config.actions);
            assert.strictEqual(okRes.config.actions["valid-action"].profile, "code");
            assert.strictEqual(okRes.config.actions["valid-action"].prompt, "prompts-00001-test");
            assert.strictEqual(okRes.config.actions["valid-action"].input?.["doc-type"], "rfc");

            // Invalid actions are marked
            assert.ok(okRes.config.invalidActionIds.has("invalid-profile"));
            assert.ok(okRes.config.invalidActionIds.has("missing-prompt"));
            assert.ok(okRes.config.invalidActionIds.has("unknown-setting"));
            assert.ok(okRes.config.invalidActionIds.has("invalid-input"));

            // Diagnostics report actionId, invalidField, and configFile
            const profileDiag = okRes.config.diagnostics.find(
                (d) => d.actionId === "invalid-profile" && d.invalidField === "profile",
            );
            assert.ok(profileDiag, "profile diagnostic must be present");
            assert.strictEqual(profileDiag.configFile, ACTIONS_YAML_DISPLAY_PATH);

            const promptDiag = okRes.config.diagnostics.find(
                (d) => d.actionId === "missing-prompt" && d.invalidField === "prompt",
            );
            assert.ok(promptDiag, "prompt diagnostic must be present");

            const unknownDiag = okRes.config.diagnostics.find(
                (d) => d.actionId === "unknown-setting" && d.invalidField === "extra-field",
            );
            assert.ok(unknownDiag, "unknown setting diagnostic must be present");

            const inputDiag = okRes.config.diagnostics.find(
                (d) => d.actionId === "invalid-input" && d.invalidField === "input",
            );
            assert.ok(inputDiag, "input diagnostic must be present");

            // Diagnostics are reported to output
            assert.ok(output.lines.length >= 4, "all diagnostics should be written to output");
        } finally {
            cleanup();
        }
    });

    test("empty action ID is rejected as invalid", () => {
        const { dir, cleanup } = makeTempWorkspace("empty-action-id");
        try {
            writeActionsYaml(dir, "actions:\n  ' ': \n    profile: code\n    prompt: p1");
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, true);
            const okRes = result;
            assert.strictEqual(Object.keys(okRes.config.actions).length, 0);
            assert.ok(okRes.config.diagnostics.some((d) => d.invalidField === "action-id"));
        } finally {
            cleanup();
        }
    });

    test("non-scalar input values are diagnosed as invalid input", () => {
        const { dir, cleanup } = makeTempWorkspace("non-scalar-input");
        try {
            const yaml = [
                `actions:`,
                `  nested-input:`,
                `    profile: code`,
                `    prompt: p1`,
                `    input:`,
                `      nested:`,
                `        deep: true`,
            ].join("\n");
            writeActionsYaml(dir, yaml);
            const result = loadActionsConfig(dir);
            assert.strictEqual(result.ok, true);
            const okRes = result;
            assert.ok(okRes.config.invalidActionIds.has("nested-input"));
            const diag = okRes.config.diagnostics.find((d) => d.actionId === "nested-input");
            assert.ok(diag && diag.invalidField === "input");
        } finally {
            cleanup();
        }
    });
});

// ── Suite 3: Variable Merging and Path Normalization ──────────────────────────

suite("Task 00078 Phase A — Variable Merging and Normalization", () => {
    test("merge order: configured input < action/form values < extension-generated values", () => {
        const configuredInput = {
            key1: "conf1",
            key2: "conf2",
            key3: "conf3",
        };
        const actionValues = {
            key2: "action2",
            key4: "action4",
        };
        const extensionValues = {
            key3: "ext3",
            key5: "ext5",
        };

        const merged = mergeActionVariables(configuredInput, actionValues, extensionValues);

        assert.strictEqual(merged["key1"], "conf1");
        assert.strictEqual(merged["key2"], "action2");
        assert.strictEqual(merged["key3"], "ext3");
        assert.strictEqual(merged["key4"], "action4");
        assert.strictEqual(merged["key5"], "ext5");
    });

    test("profile and prompt are stripped from merged variables", () => {
        const configuredInput = {
            profile: "configured-profile",
            prompt: "configured-prompt",
            data: "val",
        };
        const actionValues = {
            profile: "action-profile",
            prompt: "action-prompt",
            actionData: "actVal",
        };
        const extensionValues = {
            profile: "ext-profile",
            prompt: "ext-prompt",
        };

        const merged = mergeActionVariables(configuredInput, actionValues, extensionValues);

        assert.strictEqual(merged["profile"], undefined);
        assert.strictEqual(merged["prompt"], undefined);
        assert.strictEqual(merged["data"], "val");
        assert.strictEqual(merged["actionData"], "actVal");
    });

    test("extension-generated file-path and file-name override configured and action values", () => {
        const configuredInput = {
            "file-path": "configured/doc.md",
            "file-name": "configured.md",
        };
        const actionValues = {
            "file-path": "action/doc.md",
            "file-name": "action.md",
        };
        const extensionValues = {
            "file-path": "rfc/accepted/rfc-00044.md",
            "file-name": "rfc-00044.md",
        };

        const merged = mergeActionVariables(configuredInput, actionValues, extensionValues);

        assert.strictEqual(merged["file-path"], "rfc/accepted/rfc-00044.md");
        assert.strictEqual(merged["file-name"], "rfc-00044.md");
    });

    test("normalizeDocPath converts backslashes to forward slashes and strips leading slashes", () => {
        assert.strictEqual(
            normalizeDocPath("task\\in-progress\\task-00078.md"),
            "task/in-progress/task-00078.md",
        );
        assert.strictEqual(
            normalizeDocPath(".\\rfc\\accepted\\rfc-00044.md"),
            "rfc/accepted/rfc-00044.md",
        );
        assert.strictEqual(normalizeDocPath("/doc/task/task-00078.md"), "doc/task/task-00078.md");
    });
});

// ── Suite 4: Single-Pass Interpolation ────────────────────────────────────────

suite("Task 00078 Phase A — Prompt Interpolation", () => {
    test("replaces placeholders in a single pass", () => {
        const template = "Create doc #{doc-type} at #{file-path}";
        const variables = {
            "doc-type": "task",
            "file-path": "task/in-progress/task-00078.md",
        };
        const result = interpolateActionPrompt(template, variables);
        assert.strictEqual(result.ok, true);
        const okRes = result;
        assert.strictEqual(okRes.rendered, "Create doc task at task/in-progress/task-00078.md");
    });

    test("single pass does not recursively expand substituted values", () => {
        const template = "Initial #{token}";
        const variables = {
            token: "#{secondary}",
            secondary: "SHOULD_NOT_APPEAR",
        };
        const result = interpolateActionPrompt(template, variables);
        assert.strictEqual(result.ok, true);
        const okRes = result;
        assert.strictEqual(okRes.rendered, "Initial #{secondary}");
    });

    test("unresolved placeholders cause failure with actionable error without exposing sensitive values", () => {
        const template = "Run #{action} on #{secret-target}";
        const variables = {
            action: "execute",
        };
        const result = interpolateActionPrompt(template, variables);
        assert.strictEqual(result.ok, false);
        const errRes = result;
        assert.ok(
            errRes.error.includes("unresolved variables: secret-target"),
            `expected unresolved variable in message: ${errRes.error}`,
        );
        // Must not include prompt contents or runtime values
        assert.ok(!errRes.error.includes("execute"), "error must not leak variable values");
        assert.ok(!errRes.error.includes("Run"), "error must not leak prompt template text");
    });
});

// ── Suite 5: Action Resolution Workflow ───────────────────────────────────────

suite("Task 00078 Phase A — Action Resolution", () => {
    test("returns fallback when .vector/actions.yaml does not exist", () => {
        const { dir, cleanup } = makeTempWorkspace("res-missing");
        try {
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "create-doc",
            });
            assert.strictEqual(res.type, "fallback");
            assert.strictEqual(res.actionId, "create-doc");
        } finally {
            cleanup();
        }
    });

    test("returns fallback when action ID is not configured in actions.yaml", () => {
        const { dir, cleanup } = makeTempWorkspace("res-unconfigured");
        try {
            writeActionsYaml(
                dir,
                "actions:\n  other-action:\n    profile: code\n    prompt: prompts-00001",
            );
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "create-doc",
            });
            assert.strictEqual(res.type, "fallback");
            assert.strictEqual(res.actionId, "create-doc");
        } finally {
            cleanup();
        }
    });

    test("returns error without fallback when configured action ID is invalid", () => {
        const { dir, cleanup } = makeTempWorkspace("res-invalid-no-fallback");
        try {
            const yaml = [
                `actions:`,
                `  create-doc:`,
                `    profile: code`,
                `    # missing prompt!`,
            ].join("\n");
            writeActionsYaml(dir, yaml);
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "create-doc",
            });
            assert.strictEqual(res.type, "error");
            assert.strictEqual(res.actionId, "create-doc");
            const errRes = res;
            assert.ok(
                errRes.error.includes("missing required field 'prompt'"),
                `error should mention missing prompt: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("successfully resolves configured action, agents, and interpolated prompt", () => {
        const { dir, cleanup } = makeTempWorkspace("res-success");
        try {
            writeAgentsYaml(dir);
            const yaml = [
                `actions:`,
                `  create-doc:`,
                `    profile: create-doc`,
                `    prompt: prompts-00005-create-document`,
                `    input:`,
                `      document-type: "#{doc-type}"`,
                `      extra-default: "base"`,
            ].join("\n");
            writeActionsYaml(dir, yaml);

            const promptResolver = (_root: string, promptId: string) => {
                if (promptId === "prompts-00005-create-document") {
                    return {
                        content:
                            "Generate new document #{document-type} with mode #{mode} (#{extra-default})",
                    };
                }
                return null;
            };

            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "create-doc",
                actionOrFormValues: {
                    mode: "fast",
                },
                extensionGeneratedValues: {
                    "doc-type": "task",
                },
                promptResolver,
                isAvailable: () => true,
            });

            assert.strictEqual(res.type, "resolved");
            const okRes = res;
            assert.strictEqual(okRes.actionId, "create-doc");
            assert.strictEqual(okRes.definition.profile, "create-doc");
            assert.strictEqual(okRes.definition.prompt, "prompts-00005-create-document");
            assert.strictEqual(okRes.agents.length, 1);
            const firstAgent = okRes.agents[0];
            assert.ok(firstAgent);
            assert.strictEqual(firstAgent.name, "claude");
            assert.strictEqual(firstAgent.available, true);
            assert.strictEqual(okRes.launchPromptTemplate, "custom agent prompt <instruction-id>");
            assert.strictEqual(
                okRes.renderedPrompt,
                "Generate new document task with mode fast (base)",
            );
        } finally {
            cleanup();
        }
    });

    test("fails when profile cannot be resolved in agents.yaml", () => {
        const { dir, cleanup } = makeTempWorkspace("res-bad-profile");
        try {
            writeAgentsYaml(dir);
            writeActionsYaml(
                dir,
                "actions:\n  my-act:\n    profile: non-existent-profile\n    prompt: p1",
            );
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "my-act",
                promptResolver: () => ({ content: "plain" }),
            });
            assert.strictEqual(res.type, "error");
            const errRes = res;
            assert.ok(
                errRes.error.includes("profile 'non-existent-profile'"),
                `expected profile error: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("fails when prompt cannot be resolved", () => {
        const { dir, cleanup } = makeTempWorkspace("res-bad-prompt");
        try {
            writeAgentsYaml(dir);
            writeActionsYaml(
                dir,
                "actions:\n  my-act:\n    profile: code\n    prompt: prompts-99999-missing",
            );
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "my-act",
                promptResolver: () => null,
            });
            assert.strictEqual(res.type, "error");
            const errRes = res;
            assert.ok(
                errRes.error.includes("cannot resolve prompt document 'prompts-99999-missing'"),
                `expected prompt resolution error: ${errRes.error}`,
            );
        } finally {
            cleanup();
        }
    });

    test("fails when prompt has unresolved placeholders without leaking contents", () => {
        const { dir, cleanup } = makeTempWorkspace("res-unresolved");
        try {
            writeAgentsYaml(dir);
            writeActionsYaml(dir, "actions:\n  my-act:\n    profile: code\n    prompt: p1");
            const res = resolveConfiguredAction({
                workspaceRoot: dir,
                actionId: "my-act",
                promptResolver: () => ({ content: "Run #{action} on #{unsupplied-variable}" }),
                actionOrFormValues: { action: "build" },
            });
            assert.strictEqual(res.type, "error");
            const errRes = res;
            assert.ok(
                errRes.error.includes("unresolved variables: unsupplied-variable"),
                `expected unresolved error: ${errRes.error}`,
            );
            assert.ok(!errRes.error.includes("build"), "must not leak runtime values");
        } finally {
            cleanup();
        }
    });
});
