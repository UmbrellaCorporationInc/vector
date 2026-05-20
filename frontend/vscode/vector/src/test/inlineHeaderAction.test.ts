import * as assert from "assert";
import { renderGovernedMarkdown } from "../document-viewer/markdownRenderer.js";
import { renderGovernedMarkdownAnalysis } from "../document-viewer/markdownRenderer.js";
import { renderInlineHeaderAction } from "../document-viewer/document-actions/inlineHeaderActionRenderer.js";

suite("Phase B — inline header action rendering", () => {
    test("no inline action is rendered when documentStem is absent", () => {
        const html = renderGovernedMarkdown("# Section Title");
        assert.ok(!html.includes("vector-agent-inline-action"), "no inline action without a stem");
    });

    test("inline action is rendered on h1 when documentStem is provided", () => {
        const html = renderGovernedMarkdownAnalysis("# Section Title", {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(
            html.includes("vector-agent-inline-action"),
            "inline action must appear inside the heading",
        );
    });

    test("inline action carries the correct agent profile", () => {
        const html = renderGovernedMarkdownAnalysis("# Title", {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(html.includes('data-agent-profile="create-doc"'), "profile must be create-doc");
    });

    test("inline action carries the correct prompt identifier", () => {
        const html = renderGovernedMarkdownAnalysis("# Title", {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(
            html.includes('data-agent-prompt="prompts-00006-update-document"'),
            "prompt must be prompts-00006-update-document",
        );
    });

    test("inline action encodes document-stem in the input JSON", () => {
        const stem = "task-00042-implement-rfc-00023";
        const html = renderGovernedMarkdownAnalysis("# Title", { documentStem: stem }).html;
        assert.ok(html.includes(stem), "document-stem value must appear in the rendered HTML");
    });

    test("inline action appears on every heading level", () => {
        const source = ["# H1", "## H2", "### H3", "#### H4", "##### H5", "###### H6"].join("\n");
        const html = renderGovernedMarkdownAnalysis(source, {
            documentStem: "task-00042-foo",
        }).html;
        const count = (html.match(/vector-agent-inline-action/g) ?? []).length;
        assert.strictEqual(count, 6, "one inline action per heading level");
    });

    test("inline action is placed before the closing heading tag", () => {
        const html = renderGovernedMarkdownAnalysis("## My Section", {
            documentStem: "task-00042-foo",
        }).html;
        const actionPos = html.indexOf("vector-agent-inline-action");
        const closeTagPos = html.indexOf("</h2>");
        assert.ok(actionPos > -1, "inline action must be present");
        assert.ok(closeTagPos > -1, "closing h2 tag must be present");
        assert.ok(actionPos < closeTagPos, "action must appear before the closing heading tag");
    });

    test("multiple headings in a large document each receive an inline action", () => {
        const headings = Array.from({ length: 20 }, (_, i) => `## Section ${String(i + 1)}`);
        const html = renderGovernedMarkdownAnalysis(headings.join("\n\n"), {
            documentStem: "task-00042-foo",
        }).html;
        const count = (html.match(/vector-agent-inline-action/g) ?? []).length;
        assert.strictEqual(
            count,
            20,
            "every heading in a large document must get an inline action",
        );
    });

    test("renderGovernedMarkdown without stem produces no inline actions", () => {
        const html = renderGovernedMarkdown("# A\n## B\n### C");
        assert.ok(
            !html.includes("vector-agent-inline-action"),
            "legacy renderGovernedMarkdown must remain free of inline actions",
        );
    });

    test("inline action button displays the pencil glyph", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(html.includes("✏"), "button must contain the pencil glyph");
    });

    test("inline action carries an aria-label for accessibility", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(html.includes("aria-label="), "button must carry an aria-label");
    });

    test("inline action escapes special characters in document-stem", () => {
        const stem = 'task-00042-foo"bar';
        const html = renderInlineHeaderAction(stem);
        assert.ok(!html.includes('"bar'), "unescaped quote must not appear in the HTML attribute");
    });

    test("inline action includes document-header when heading text is provided", () => {
        const html = renderInlineHeaderAction("task-00042-foo", "My Section");
        assert.ok(
            html.includes("document-header"),
            "input JSON must contain document-header when heading text is given",
        );
        assert.ok(html.includes("My Section"), "document-header value must equal the heading text");
    });

    test("inline action omits document-header when heading text is absent", () => {
        const html = renderInlineHeaderAction("task-00042-foo");
        assert.ok(
            !html.includes("document-header"),
            "input JSON must not contain document-header when no heading text is given",
        );
    });

    test("rendered heading carries document-header matching the heading title", () => {
        const html = renderGovernedMarkdownAnalysis("## Implementation Plan", {
            documentStem: "task-00042-foo",
        }).html;
        assert.ok(
            html.includes("document-header"),
            "rendered heading must include document-header in the action input",
        );
        assert.ok(
            html.includes("Implementation Plan"),
            "document-header value must match the heading text",
        );
    });
});
