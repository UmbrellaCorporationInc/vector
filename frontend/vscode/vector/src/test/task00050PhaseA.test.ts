import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import {
    parseAgentBlock,
    isAgentBlockParseError,
} from "../document-viewer/document-actions/agentBlockParser.js";
import { renderAgentBlock } from "../document-viewer/document-actions/agentBlockRenderer.js";
import { createGovernedMarkdownIt } from "../document-viewer/markdownRenderer.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const previewJs = fs.readFileSync(path.join(extensionRoot, "media", "preview.js"), "utf-8");

// ---------------------------------------------------------------------------
// parseAgentBlock — prompt-field
// ---------------------------------------------------------------------------

suite("Task 00050 Phase A — parseAgentBlock prompt-field", () => {
    test("promptField defaults to 'prompt-message' when prompt-field is absent", () => {
        const yaml = "label: Run\nprofile: code\nprompt: p\n";
        const result = parseAgentBlock(yaml);
        assert.ok(!isAgentBlockParseError(result));
        assert.strictEqual(result.promptField, "prompt-message");
    });

    test("promptField equals the specified value when prompt-field is present", () => {
        const yaml = "label: Create\nprofile: create-doc\nprompt: p\nprompt-field: message\n";
        const result = parseAgentBlock(yaml);
        assert.ok(!isAgentBlockParseError(result));
        assert.strictEqual(result.promptField, "message");
    });

    test("promptField trims whitespace from the prompt-field value", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\nprompt-field: '  context  '\n";
        const result = parseAgentBlock(yaml);
        assert.ok(!isAgentBlockParseError(result));
        assert.strictEqual(result.promptField, "context");
    });

    test("parseAgentBlock returns an error when prompt-field is an empty string", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\nprompt-field: ''\n";
        const result = parseAgentBlock(yaml);
        assert.ok(isAgentBlockParseError(result));
        assert.ok(result.error.includes("prompt-field"), "error must mention prompt-field");
    });

    test("parseAgentBlock returns an error when prompt-field is a non-string value", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\nprompt-field: 42\n";
        const result = parseAgentBlock(yaml);
        assert.ok(isAgentBlockParseError(result));
    });
});

// ---------------------------------------------------------------------------
// renderAgentBlock — inline-action variant
// ---------------------------------------------------------------------------

suite("Task 00050 Phase A — renderAgentBlock inline-action variant", () => {
    test("inline-action variant uses vector-agent-inline-action CSS class", () => {
        const yaml = "label: Create\nprofile: create-doc\nprompt: p\n";
        const html = renderAgentBlock(yaml, "inline-action");
        assert.ok(
            html.includes('class="vector-agent-inline-action"'),
            "must carry vector-agent-inline-action class",
        );
    });

    test("inline-action variant emits data-agent-prompt-field with default value", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\n";
        const html = renderAgentBlock(yaml, "inline-action");
        assert.ok(
            html.includes('data-agent-prompt-field="prompt-message"'),
            "must emit data-agent-prompt-field defaulting to prompt-message",
        );
    });

    test("inline-action variant emits data-agent-prompt-field with custom value", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\nprompt-field: message\n";
        const html = renderAgentBlock(yaml, "inline-action");
        assert.ok(
            html.includes('data-agent-prompt-field="message"'),
            "must emit data-agent-prompt-field with the custom field name",
        );
    });

    test("button variant does not emit data-agent-prompt-field", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\n";
        const html = renderAgentBlock(yaml, "button");
        assert.ok(
            !html.includes("data-agent-prompt-field"),
            "button variant must not emit data-agent-prompt-field",
        );
    });

    test("action variant does not emit data-agent-prompt-field", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\n";
        const html = renderAgentBlock(yaml, "action");
        assert.ok(
            !html.includes("data-agent-prompt-field"),
            "action variant must not emit data-agent-prompt-field",
        );
    });

    test("inline-action variant escapes HTML in prompt-field", () => {
        const yaml = "label: L\nprofile: p\nprompt: q\nprompt-field: 'a\"b'\n";
        const html = renderAgentBlock(yaml, "inline-action");
        assert.ok(!html.includes('"b"'), "double quote in prompt-field must be escaped");
    });
});

// ---------------------------------------------------------------------------
// markdownRenderer — vector-agent-inline-action fence
// ---------------------------------------------------------------------------

suite("Task 00050 Phase A — vector-agent-inline-action fence block", () => {
    test("createGovernedMarkdownIt renders vector-agent-inline-action as an inline-action button", () => {
        const md = createGovernedMarkdownIt();
        const src =
            "```vector-agent-inline-action\nlabel: Create\nprofile: create-doc\nprompt: p\n```";
        const html = md.render(src);
        assert.ok(
            html.includes('class="vector-agent-inline-action"'),
            "fence must render as vector-agent-inline-action button",
        );
        assert.ok(html.includes("<button"), "must render a button element");
    });

    test("vector-agent-inline-action fence propagates prompt-field into data attribute", () => {
        const md = createGovernedMarkdownIt();
        const src =
            "```vector-agent-inline-action\nlabel: Create\nprofile: create-doc\nprompt: p\nprompt-field: message\n```";
        const html = md.render(src);
        assert.ok(
            html.includes('data-agent-prompt-field="message"'),
            "fence must emit data-agent-prompt-field attribute",
        );
    });

    test("vector-agent-inline-action fence defaults data-agent-prompt-field to prompt-message", () => {
        const md = createGovernedMarkdownIt();
        const src = "```vector-agent-inline-action\nlabel: L\nprofile: p\nprompt: q\n```";
        const html = md.render(src);
        assert.ok(
            html.includes('data-agent-prompt-field="prompt-message"'),
            "fence must default data-agent-prompt-field to prompt-message",
        );
    });
});

// ---------------------------------------------------------------------------
// preview.js — promptField wiring
// ---------------------------------------------------------------------------

suite("Task 00050 Phase A — preview.js promptField wiring", () => {
    test("preview.js reads dataset.agentPromptField from the clicked inline-action element", () => {
        assert.ok(
            previewJs.includes("dataset.agentPromptField"),
            "click handler must read agentPromptField from dataset",
        );
    });

    test("preview.js passes promptField to openInlineOverlay", () => {
        assert.ok(
            previewJs.includes("promptField: promptField"),
            "openInlineOverlay call must include promptField",
        );
    });

    test("preview.js builds form content using action.promptField instead of a hardcoded field name", () => {
        assert.ok(
            previewJs.includes("action.promptField"),
            "form content must reference action.promptField",
        );
        assert.ok(
            previewJs.includes("OVERLAY_FORM_CONTENT_TEMPLATE"),
            "form content must use the template constant",
        );
    });

    test("preview.js uses action.promptField as the key when injecting extra input into staticInput", () => {
        const submitFnStart = previewJs.indexOf("function submitOverlay");
        const submitFnEnd = previewJs.indexOf("\n  }", submitFnStart);
        const submitFn = previewJs.slice(submitFnStart, submitFnEnd);
        assert.ok(
            submitFn.includes("staticInput[promptField]"),
            "submitOverlay must inject extra input under promptField key",
        );
    });

    test("preview.js defaults promptField to prompt-message when dataset attribute is absent", () => {
        assert.ok(
            previewJs.includes('|| "prompt-message"'),
            "promptField fallback must default to prompt-message",
        );
    });

    test("preview.js saves promptField before clearing overlayPendingAction in closeInlineOverlay", () => {
        const closeFnStart = previewJs.indexOf("function closeInlineOverlay");
        const closeFnEnd = previewJs.indexOf("\n  }", closeFnStart);
        const closeFn = previewJs.slice(closeFnStart, closeFnEnd);
        const promptFieldIdx = closeFn.indexOf("promptField");
        const clearActionIdx = closeFn.indexOf("overlayPendingAction = null");
        assert.ok(
            promptFieldIdx < clearActionIdx,
            "promptField must be saved before overlayPendingAction is cleared",
        );
    });
});
