import * as assert from "assert";
import {
    classifyLine,
    tokenizeInline,
    joinTokenText,
} from "../document-viewer/chat-input/chatInputMarkdown.js";

suite("Phase D — classifyLine: heading detection", () => {
    test("classifies h1 heading", () => {
        assert.strictEqual(classifyLine("# Title"), "heading");
    });

    test("classifies h2 heading", () => {
        assert.strictEqual(classifyLine("## Title"), "heading");
    });

    test("classifies h3 heading", () => {
        assert.strictEqual(classifyLine("### Title"), "heading");
    });

    test("classifies h6 heading", () => {
        assert.strictEqual(classifyLine("###### Title"), "heading");
    });

    test("returns null for plain text", () => {
        assert.strictEqual(classifyLine("plain text"), null);
    });

    test("returns null for hash without trailing space", () => {
        assert.strictEqual(classifyLine("#nospace"), null);
    });

    test("returns null for empty string", () => {
        assert.strictEqual(classifyLine(""), null);
    });
});

suite("Phase D — classifyLine: list item detection", () => {
    test("classifies unordered list with dash", () => {
        assert.strictEqual(classifyLine("- item"), "list-item");
    });

    test("classifies unordered list with asterisk", () => {
        assert.strictEqual(classifyLine("* item"), "list-item");
    });

    test("classifies unordered list with plus", () => {
        assert.strictEqual(classifyLine("+ item"), "list-item");
    });

    test("classifies ordered list item", () => {
        assert.strictEqual(classifyLine("1. item"), "list-item");
    });

    test("classifies multi-digit ordered list item", () => {
        assert.strictEqual(classifyLine("10. item"), "list-item");
    });
});

suite("Phase D — classifyLine: fenced code detection", () => {
    test("classifies fenced code block opener with language", () => {
        assert.strictEqual(classifyLine("```typescript"), "fenced-code");
    });

    test("classifies fenced code block closer", () => {
        assert.strictEqual(classifyLine("```"), "fenced-code");
    });
});

suite("Phase D — tokenizeInline: plain text", () => {
    test("returns single plain token for plain text", () => {
        const tokens = tokenizeInline("plain text");
        assert.strictEqual(tokens.length, 1);
        const [first] = tokens;
        assert.ok(first);
        assert.strictEqual(first.type, "plain");
        assert.strictEqual(first.text, "plain text");
    });

    test("returns empty array for empty string", () => {
        const tokens = tokenizeInline("");
        assert.strictEqual(tokens.length, 0);
    });
});

suite("Phase D — tokenizeInline: strong emphasis", () => {
    test("recognizes **strong** token", () => {
        const tokens = tokenizeInline("**bold**");
        assert.strictEqual(tokens.length, 1);
        const [first] = tokens;
        assert.ok(first);
        assert.strictEqual(first.type, "strong");
        assert.strictEqual(first.text, "**bold**");
    });

    test("splits surrounding text from strong token", () => {
        const tokens = tokenizeInline("review **this** file");
        assert.strictEqual(tokens.length, 3);
        const [a, b, c] = tokens;
        assert.ok(a);
        assert.ok(b);
        assert.ok(c);
        assert.strictEqual(a.type, "plain");
        assert.strictEqual(b.type, "strong");
        assert.strictEqual(c.type, "plain");
    });
});

suite("Phase D — tokenizeInline: emphasis", () => {
    test("recognizes *em* token", () => {
        const tokens = tokenizeInline("*italic*");
        assert.strictEqual(tokens.length, 1);
        const [first] = tokens;
        assert.ok(first);
        assert.strictEqual(first.type, "em");
        assert.strictEqual(first.text, "*italic*");
    });

    test("does not treat list marker * as italic opener", () => {
        const tokens = tokenizeInline("* item text");
        assert.ok(
            tokens.every((t) => t.type !== "em"),
            "list * must not be matched as italic",
        );
    });
});

suite("Phase D — tokenizeInline: inline code", () => {
    test("recognizes `code` token", () => {
        const tokens = tokenizeInline("`code`");
        assert.strictEqual(tokens.length, 1);
        const [first] = tokens;
        assert.ok(first);
        assert.strictEqual(first.type, "code");
        assert.strictEqual(first.text, "`code`");
    });
});

suite("Phase D — tokenizeInline: source preservation", () => {
    test("joinTokenText preserves original source for mixed inline tokens", () => {
        const source = "review **this** and `code` text";
        assert.strictEqual(joinTokenText(tokenizeInline(source)), source);
    });

    test("joinTokenText preserves source for heading line with inline tokens", () => {
        const source = "## Heading with **bold**";
        assert.strictEqual(joinTokenText(tokenizeInline(source)), source);
    });

    test("joinTokenText preserves source for plain text", () => {
        const source = "plain text no markdown";
        assert.strictEqual(joinTokenText(tokenizeInline(source)), source);
    });

    test("joinTokenText preserves source with multiple inline tokens", () => {
        const source = "**a** and *b* and `c`";
        assert.strictEqual(joinTokenText(tokenizeInline(source)), source);
    });

    test("joinTokenText preserves source when no patterns match", () => {
        const source = "** incomplete and * alone";
        assert.strictEqual(joinTokenText(tokenizeInline(source)), source);
    });

    test("joinTokenText preserves source for empty string", () => {
        assert.strictEqual(joinTokenText(tokenizeInline("")), "");
    });
});
