import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { fileURLToPath } from "url";
import * as yaml from "js-yaml";
import {
    loadActionsConfig,
    resolveConfiguredAction,
} from "../document-viewer/document-actions/actionsConfig.js";
import { documentActionMetadata } from "../document-viewer/document-actions/actionRuntimeContext.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

suite("Task 00079 Phase B — documented action inputs", () => {
    test("README examples load and resolve with documented runtime values", () => {
        const readme = fs.readFileSync(path.join(extensionRoot, "README.md"), "utf-8");
        const section = readme.split("### 8.8 Project action overrides\n")[1]?.split("\n## 9.")[0];
        assert.ok(section, "Project action override documentation must exist");
        const configExample = section.match(/```yaml\n([\s\S]*?)\n```/)?.[1];
        const actionExample = section.match(/```vector-agent-action\n([\s\S]*?)\n```/)?.[1];
        assert.ok(configExample, "actions.yaml example must exist");
        assert.ok(actionExample, "Markdown action example must exist");

        const root = fs.mkdtempSync(path.join(os.tmpdir(), "vector-task79-phaseb-"));
        try {
            fs.mkdirSync(path.join(root, ".vector"), { recursive: true });
            fs.writeFileSync(path.join(root, ".vector", "actions.yaml"), configExample);
            fs.writeFileSync(
                path.join(root, ".vector", "agents.yaml"),
                `instructions-dir: "\${system-temp}/vector-task79-phaseb/instructions"\nagents:\n  node:\n    type: cli\n    command: node '<instruction>'\nprofiles:\n  create-doc: [node]\n  code: [node]\n`,
            );
            const loaded = loadActionsConfig(root);
            assert.strictEqual(loaded.ok, true);
            assert.deepStrictEqual(loaded.config.diagnostics, []);

            const create = resolveConfiguredAction({
                workspaceRoot: root,
                actionId: "create-doc",
                actionsConfig: loaded,
                extensionGeneratedValues: { "doc-type": "task" },
                promptResolver: () => ({
                    content: "Create #{document-type} #{document-name}: #{message}",
                }),
                isAvailable: () => true,
            });
            assert.strictEqual(create.type, "resolved", JSON.stringify(create));
            assert.strictEqual(create.effectiveProfile, "create-doc");
            assert.strictEqual(create.mergedVariables["document-type"], "task");

            const block = yaml.load(actionExample) as Record<string, unknown>;
            const blockInput = block.input as Record<string, string>;
            const source = path.join(root, "doc", "task", "task-00079-source.md");
            const metadata = documentActionMetadata(root, source);
            assert.deepStrictEqual(metadata, {
                "file-path": "task/task-00079-source.md",
                "file-name": "task-00079-source.md",
            });
            const action = resolveConfiguredAction({
                workspaceRoot: root,
                actionId: "vector-action",
                actionsConfig: loaded,
                runtimeProfile: block.profile as string,
                actionOrFormValues: { ...blockInput, label: block.label as string },
                extensionGeneratedValues: metadata,
                promptResolver: () => ({
                    content: "#{task} #{phase} #{language} #{file-path} #{file-name}",
                }),
                isAvailable: () => true,
            });
            assert.strictEqual(action.type, "resolved", JSON.stringify(action));
            assert.strictEqual(action.effectiveProfile, "code");
            assert.strictEqual(action.mergedVariables.language, "Markdown, YAML");
            assert.strictEqual(action.mergedVariables["file-path"], metadata["file-path"]);
            assert.strictEqual(action.mergedVariables["file-name"], metadata["file-name"]);
        } finally {
            fs.rmSync(root, { recursive: true, force: true });
        }
    });
});
