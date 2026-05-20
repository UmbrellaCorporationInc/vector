import * as assert from "assert";
import {
    substituteVariables,
    findUnresolvedVariables,
} from "../document-viewer/document-actions/variableSubstitution.js";
import { renderFormBlock } from "../document-viewer/form-editor/formRenderer.js";
import { renderGovernedMarkdown } from "../document-viewer/markdownRenderer.js";
import { renderAgentBlock } from "../document-viewer/document-actions/agentBlockRenderer.js";
import { reconcileMentions } from "../document-viewer/chat-input/chatInputMention.js";

suite("Phase E — agent-action and agent-button consume only plain text", () => {
    test("chat-input value containing @mention text is treated as plain string", () => {
        const prompt = "Review #{body}";
        const formValues = { body: "Please review @src/formRenderer.ts and summarize" };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "Review Please review @src/formRenderer.ts and summarize");
    });

    test("chat-input value with multiple @mentions passes through without corruption", () => {
        const prompt = "#{task}";
        const formValues = {
            task: "Check @src/a.ts and compare with @src/b.ts",
        };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "Check @src/a.ts and compare with @src/b.ts");
    });

    test("chat-input value with Markdown syntax passes through unchanged", () => {
        const prompt = "#{body}";
        const formValues = { body: "## Review\n\nPlease review **this** and `that`" };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "## Review\n\nPlease review **this** and `that`");
    });

    test("empty chat-input value substitutes to empty string", () => {
        const prompt = "Prefix #{body} suffix";
        const result = substituteVariables(prompt, { body: "" });
        assert.strictEqual(result, "Prefix  suffix");
    });

    test("@mention text in chat-input value is not misdetected as an unresolved #{variable}", () => {
        const prompt = "#{body}";
        const formValues = { body: "@src/formRenderer.ts" };
        const unresolved = findUnresolvedVariables(prompt, formValues);
        assert.deepStrictEqual(unresolved, []);
    });
});

suite("Phase E — multiple-form collection contract", () => {
    test("two chat-input fields in a single block carry distinct data-form-key attributes", () => {
        const html = renderFormBlock(
            `prompt = chat-input("Prompt")\nsecondary = chat-input("Secondary")`,
        );
        assert.ok(
            html.includes('data-form-key="prompt"'),
            "first chat-input must carry its key as data-form-key",
        );
        assert.ok(
            html.includes('data-form-key="secondary"'),
            "second chat-input must carry its key as data-form-key",
        );
    });

    test("chat-input and regular input fields coexist without interfering", () => {
        const html = renderFormBlock(`title = input("Title")\nbody = chat-input("Body")`);
        assert.ok(html.includes('data-form-key="title"'), "regular input must keep its key");
        assert.ok(html.includes('data-form-key="body"'), "chat-input must keep its key");
        assert.ok(
            html.includes('class="vector-form-input"'),
            "regular input must still use vector-form-input",
        );
        assert.ok(
            html.includes('class="vector-chat-input-host"'),
            "chat-input must use the editor host",
        );
    });

    test("each editable chat-input host carries a unique data-chat-input-name", () => {
        const html = renderFormBlock(`alpha = chat-input("Alpha")\nbeta = chat-input("Beta")`);
        assert.ok(
            html.includes('data-chat-input-name="alpha"'),
            "first host must carry data-chat-input-name=alpha",
        );
        assert.ok(
            html.includes('data-chat-input-name="beta"'),
            "second host must carry data-chat-input-name=beta",
        );
    });

    test("chat-input collection uses the editable marker with a dedicated CodeMirror mount", () => {
        const html = renderFormBlock(`body = chat-input("Message")`);
        assert.ok(
            html.includes('data-chat-input-editable="true"'),
            "editable mount must carry data-chat-input-editable so the runtime can locate the CodeMirror host",
        );
    });
});

suite("Phase E — unresolved and unsupported mentions do not corrupt submission", () => {
    test("unresolved @mention left as raw text does not break variable substitution", () => {
        const prompt = "Task: #{body}";
        const formValues = { body: "Analyze @missing-file.ts please" };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "Task: Analyze @missing-file.ts please");
    });

    test("reconcileMentions filters out mentions not present in the current text", () => {
        const text = "No mentions here anymore";
        const stale = [{ type: "file" as const, label: "formRenderer.ts", path: "src/a.ts" }];
        const result = reconcileMentions(text, stale);
        assert.deepStrictEqual(result, [], "stale mentions must be filtered out before submission");
    });

    test("reconcileMentions keeps only mentions whose label appears in the text", () => {
        const text = "Please review @src/a.ts now";
        const mentions = [
            { type: "file" as const, label: "a.ts", path: "src/a.ts" },
            { type: "file" as const, label: "other.ts", path: "src/other.ts" },
        ];
        const result = reconcileMentions(text, mentions);
        assert.strictEqual(result.length, 1);
        assert.strictEqual(result[0]?.label, "a.ts");
    });

    test("empty mentions list produces no corruption even with @-like text", () => {
        const prompt = "#{body}";
        const formValues = { body: "user@example.com is not a mention" };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "user@example.com is not a mention");
    });

    test("raw unresolved @query text is preserved intact in the substituted prompt", () => {
        const prompt = "Prompt: #{q}";
        const formValues = { q: "what about @nonexistent-path/file.ts?" };
        const result = substituteVariables(prompt, formValues);
        assert.strictEqual(result, "Prompt: what about @nonexistent-path/file.ts?");
    });
});

suite("Phase E — governed preview outside vector-form blocks is preserved", () => {
    test("Markdown headings outside form blocks render correctly", () => {
        const html = renderGovernedMarkdown("# Title\n\nBody paragraph.");
        assert.ok(html.includes("<h1"), "heading must be present outside form blocks");
        assert.ok(html.includes("Title"), "heading text must appear");
    });

    test("vector-form block does not pollute surrounding Markdown content", () => {
        const source = [
            "# Doc Title",
            "",
            "```vector-form",
            `body = chat-input("Message")`,
            "```",
            "",
            "Trailing paragraph.",
        ].join("\n");
        const html = renderGovernedMarkdown(source);
        assert.ok(html.includes("Doc Title"), "heading before form must render");
        assert.ok(html.includes("vector-form"), "form block must render");
        assert.ok(html.includes("Trailing paragraph"), "paragraph after form must render");
    });

    test("a document with no vector-form blocks renders as normal governed Markdown", () => {
        const source = "## Subtitle\n\nSome **bold** text.";
        const html = renderGovernedMarkdown(source);
        assert.ok(html.includes("<h2"), "subheading must render");
        assert.ok(!html.includes("vector-form"), "form class must not appear");
    });

    test("agent action block renders data attributes independently from form collection", () => {
        const yaml = [
            "label: Run Agent",
            "profile: code",
            "prompt: prompts-00001-test",
            "input:",
            "  task: task-00037",
        ].join("\n");
        const html = renderAgentBlock(yaml, "action");
        assert.ok(
            html.includes('data-agent-profile="code"'),
            "agent block must carry data-agent-profile",
        );
        assert.ok(
            html.includes('data-agent-prompt="prompts-00001-test"'),
            "agent block must carry data-agent-prompt",
        );
        assert.ok(
            html.includes("data-agent-input="),
            "agent block must carry JSON-encoded static input",
        );
        assert.ok(
            !html.includes("formValues"),
            "agent block HTML must not reference formValues — that is resolved at click time",
        );
    });
});

suite("Phase E — prompt substitution compatibility with chat-input fields", () => {
    test("#{key} in prompt content is replaced by the chat-input plain text value", () => {
        const promptContent = "---\ntitle: Prompt\n---\n\nExecute: #{body}";
        const formValues = { body: "Fix the bug in @frontend/vscode/vector/src/main.ts" };
        const result = substituteVariables(promptContent, formValues);
        assert.ok(
            result.includes("Fix the bug in @frontend/vscode/vector/src/main.ts"),
            "substituted prompt must contain the full chat-input plain text",
        );
        assert.ok(!result.includes("#{body}"), "#{body} placeholder must be replaced");
    });

    test("unresolved #{key} without a matching form field remains unchanged", () => {
        const prompt = "Do #{task} using #{body}";
        const formValues = { task: "review" };
        const result = substituteVariables(prompt, formValues);
        assert.ok(result.includes("#{body}"), "unresolved #{body} must remain in output");
        assert.ok(!result.includes("#{task}"), "resolved #{task} must be substituted");
    });

    test("findUnresolvedVariables reports keys missing from merged form values", () => {
        const prompt = "Do #{task} and #{body}";
        const formValues = { task: "check it" };
        const unresolved = findUnresolvedVariables(prompt, formValues);
        assert.deepStrictEqual(unresolved, ["body"]);
    });

    test("no unresolved variables when all #{keys} are covered by form values", () => {
        const prompt = "#{greeting} world, #{action}";
        const formValues = { greeting: "hello", action: "done" };
        const unresolved = findUnresolvedVariables(prompt, formValues);
        assert.deepStrictEqual(unresolved, []);
    });
});
