import * as assert from "assert";
import { renderGovernedMarkdown } from "../document-viewer/markdownRenderer.js";

suite("Phase A — Callout Parser", () => {
    test("detects [!type] and replaces blockquote with callout div", () => {
        const html = renderGovernedMarkdown("> [!note]\n> body text");
        assert.ok(
            html.includes('class="vector-callout vector-callout--note"'),
            "should have callout class",
        );
        assert.ok(!html.includes("<blockquote>"), "should not fall back to blockquote");
    });

    test("type is lowercased for the CSS modifier class", () => {
        const html = renderGovernedMarkdown("> [!WARNING]\n> body");
        assert.ok(html.includes("vector-callout--warning"), "CSS class should be lowercase");
    });

    test("callout label is uppercased in the title bar", () => {
        const html = renderGovernedMarkdown("> [!note]\n> body");
        assert.ok(
            html.includes('<span class="vector-callout-label">NOTE</span>'),
            "label should be uppercase",
        );
    });

    test("extracts optional inline title after the type", () => {
        const html = renderGovernedMarkdown("> [!note] Prime Directive\n> body");
        assert.ok(
            html.includes('<span class="vector-callout-heading">Prime Directive</span>'),
            "should render inline title",
        );
    });

    test("body content is preserved inside the callout", () => {
        const html = renderGovernedMarkdown("> [!tip]\n> this is the body");
        assert.ok(html.includes("this is the body"), "body text should appear in output");
    });

    test("plain blockquote without [!type] passes through unchanged", () => {
        const html = renderGovernedMarkdown("> ordinary quote");
        assert.ok(html.includes("<blockquote>"), "should remain a blockquote");
        assert.ok(!html.includes("vector-callout"), "should not be treated as a callout");
    });

    test("type with spaces matches and produces a hyphenated CSS class", () => {
        const html = renderGovernedMarkdown("> [!Prime Directive]\n> body");
        assert.ok(
            html.includes("vector-callout--prime-directive"),
            "spaces in type should be converted to hyphens in CSS class",
        );
        assert.ok(!html.includes("<blockquote>"), "should not fall back to blockquote");
    });

    test("multi-word type label is uppercased with spaces preserved", () => {
        const html = renderGovernedMarkdown("> [!Prime Directive]\n> body");
        assert.ok(
            html.includes('<span class="vector-callout-label">PRIME DIRECTIVE</span>'),
            "label should uppercase the original type including spaces",
        );
    });

    test("data-callout-type attribute is set to the lowercased type", () => {
        const html = renderGovernedMarkdown("> [!WARNING]\n> body");
        assert.ok(
            html.includes('data-callout-type="warning"'),
            "data attribute should hold lowercase type",
        );
    });
});

suite("Phase B — Callout Renderer", () => {
    test("known types produce their type-specific CSS modifier class", () => {
        for (const type of [
            "bug",
            "example",
            "quote",
            "success",
            "warning",
            "failure",
            "abstract",
        ]) {
            const html = renderGovernedMarkdown(`> [!${type}]\n> body`);
            assert.ok(
                html.includes(`vector-callout--${type}`),
                `[!${type}] should produce vector-callout--${type}`,
            );
        }
    });

    test("unknown custom type renders as a callout, not a plain blockquote", () => {
        const html = renderGovernedMarkdown("> [!custom-type]\n> body");
        assert.ok(html.includes("vector-callout"), "should render as a callout");
        assert.ok(!html.includes("<blockquote>"), "should not fall back to blockquote");
    });

    test("callout body renders inline markdown bold", () => {
        const html = renderGovernedMarkdown("> [!note]\n> **bold text**");
        assert.ok(html.includes("<strong>bold text</strong>"), "should render bold inline");
    });

    test("callout body renders inline code spans", () => {
        const html = renderGovernedMarkdown("> [!note]\n> use `inline code` here");
        assert.ok(
            html.includes("vector-inline-code"),
            "should render inline code with governed class",
        );
    });

    test("multi-paragraph body is fully preserved inside the callout", () => {
        const html = renderGovernedMarkdown(
            "> [!note]\n>\n> first paragraph\n>\n> second paragraph",
        );
        assert.ok(html.includes("first paragraph"), "first paragraph should be in output");
        assert.ok(html.includes("second paragraph"), "second paragraph should be in output");
        assert.ok(!html.includes("<blockquote>"), "should not fall back to blockquote");
    });
});
