import * as assert from "assert";
import {
    buildMention,
    createMentionToken,
    detectMentionQuery,
    findMentionRangeAtCursor,
    findMentionRanges,
    insertMentionText,
    reconcileMentions,
} from "../document-viewer/chat-input/chatInputMention.js";

suite("Phase C - detectMentionQuery", () => {
    test("returns null when cursor is at position 0", () => {
        assert.strictEqual(detectMentionQuery("@form", 0), null);
    });

    test("detects @ at the start of text", () => {
        const result = detectMentionQuery("@form", 5);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "form");
        assert.strictEqual(result.start, 0);
        assert.strictEqual(result.end, 5);
    });

    test("detects @ after a space", () => {
        const result = detectMentionQuery("hello @form", 11);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "form");
        assert.strictEqual(result.start, 6);
        assert.strictEqual(result.end, 11);
    });

    test("detects empty query when cursor is immediately after @", () => {
        const result = detectMentionQuery("@", 1);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "");
        assert.strictEqual(result.start, 0);
        assert.strictEqual(result.end, 1);
    });

    test("detects @ after a newline", () => {
        const result = detectMentionQuery("line1\n@form", 11);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "form");
    });

    test("returns null when no @ trigger is present", () => {
        assert.strictEqual(detectMentionQuery("no trigger here", 15), null);
    });

    test("returns null for email-style address without leading whitespace before @", () => {
        assert.strictEqual(detectMentionQuery("user@example.com", 16), null);
    });

    test("returns null when cursor is before the @ trigger", () => {
        assert.strictEqual(detectMentionQuery("hello @form", 3), null);
    });

    test("captures partial query as the user types", () => {
        const result = detectMentionQuery("review @formRe", 14);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "formRe");
    });

    test("detects empty query when cursor is right after @ preceded by space", () => {
        const result = detectMentionQuery("review @", 8);
        assert.ok(result !== null);
        assert.strictEqual(result.query, "");
        assert.strictEqual(result.start, 7);
    });
});

suite("Phase C - insertMentionText", () => {
    test("createMentionToken uses the stable workspace path form", () => {
        assert.strictEqual(
            createMentionToken({
                label: "formRenderer.ts",
                path: "frontend/vscode/vector/src/formRenderer.ts",
            }),
            "@frontend/vscode/vector/src/formRenderer.ts",
        );
    });

    test("replaces the @query token with @path at cursor", () => {
        const text = "@form";
        const q = detectMentionQuery(text, 5);
        assert.ok(q !== null);
        const { text: result, cursorPos } = insertMentionText(text, q, {
            label: "formRenderer.ts",
            path: "src/formRenderer.ts",
        });
        assert.strictEqual(result, "@src/formRenderer.ts");
        assert.strictEqual(cursorPos, "@src/formRenderer.ts".length);
    });

    test("replaces mid-text @query while preserving surrounding text", () => {
        const text = "please review @form more";
        const q = detectMentionQuery(text, 19);
        assert.ok(q !== null);
        const { text: result } = insertMentionText(text, q, {
            label: "formRenderer.ts",
            path: "src/formRenderer.ts",
        });
        assert.strictEqual(result, "please review @src/formRenderer.ts more");
    });

    test("handles empty query replacement", () => {
        const text = "review @";
        const q = detectMentionQuery(text, 8);
        assert.ok(q !== null);
        const { text: result, cursorPos } = insertMentionText(text, q, {
            label: "file.ts",
            path: "src/file.ts",
        });
        assert.strictEqual(result, "review @src/file.ts");
        assert.strictEqual(cursorPos, "review @src/file.ts".length);
    });

    test("cursor position is placed immediately after the inserted token", () => {
        const text = "@f";
        const q = detectMentionQuery(text, 2);
        assert.ok(q !== null);
        const { cursorPos } = insertMentionText(text, q, {
            label: "formRenderer.ts",
            path: "src/formRenderer.ts",
        });
        assert.strictEqual(cursorPos, "@src/formRenderer.ts".length);
    });
});

suite("Phase C - buildMention", () => {
    test("creates a file mention with type, label, and path", () => {
        const mention = buildMention({ label: "formRenderer.ts", path: "src/formRenderer.ts" });
        assert.strictEqual(mention.type, "file");
        assert.strictEqual(mention.label, "formRenderer.ts");
        assert.strictEqual(mention.path, "src/formRenderer.ts");
    });
});

suite("Phase C - reconcileMentions", () => {
    test("keeps mentions whose paths appear in the text", () => {
        const text = "please review @src/formRenderer.ts";
        const mentions = [
            { type: "file" as const, label: "formRenderer.ts", path: "src/formRenderer.ts" },
            { type: "file" as const, label: "other.ts", path: "src/other.ts" },
        ];
        const result = reconcileMentions(text, mentions);
        assert.strictEqual(result.length, 1);
        assert.strictEqual(result[0]?.label, "formRenderer.ts");
    });

    test("returns an empty array when no mentions appear in text", () => {
        const result = reconcileMentions("no mentions here", [
            { type: "file" as const, label: "file.ts", path: "src/file.ts" },
        ]);
        assert.deepStrictEqual(result, []);
    });

    test("keeps all mentions when all paths appear in text", () => {
        const text = "review @src/a.ts and @src/b.ts";
        const mentions = [
            { type: "file" as const, label: "a.ts", path: "src/a.ts" },
            { type: "file" as const, label: "b.ts", path: "src/b.ts" },
        ];
        const result = reconcileMentions(text, mentions);
        assert.strictEqual(result.length, 2);
    });

    test("returns empty array when mentions list is empty", () => {
        const result = reconcileMentions("some text @whatever", []);
        assert.deepStrictEqual(result, []);
    });

    test("distinguishes duplicate labels by path", () => {
        const text = "compare @src/a/formRenderer.ts with @src/b/formRenderer.ts";
        const mentions = [
            { type: "file" as const, label: "formRenderer.ts", path: "src/a/formRenderer.ts" },
            { type: "file" as const, label: "formRenderer.ts", path: "src/b/formRenderer.ts" },
        ];
        const ranges = findMentionRanges(text, mentions);
        assert.strictEqual(ranges.length, 2);
        assert.strictEqual(ranges[0]?.mention.path, "src/a/formRenderer.ts");
        assert.strictEqual(ranges[1]?.mention.path, "src/b/formRenderer.ts");
    });

    test("finds whole mention deletion boundaries for backspace and delete", () => {
        const text = "review @src/formRenderer.ts please";
        const mentions = [
            { type: "file" as const, label: "formRenderer.ts", path: "src/formRenderer.ts" },
        ];
        const backward = findMentionRangeAtCursor(text, mentions, 27, "backward");
        const forward = findMentionRangeAtCursor(text, mentions, 7, "forward");
        assert.ok(backward);
        assert.ok(forward);
        assert.strictEqual(backward.from, 7);
        assert.strictEqual(forward.to, 27);
    });
});
