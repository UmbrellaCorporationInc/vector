import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";
import {
    detectMentionQuery,
    insertMentionText,
    reconcileMentions,
} from "../document-viewer/chat-input/chatInputMention.js";

const extensionRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const mediaDir = path.join(extensionRoot, "media");
const sourceRoot = path.join(extensionRoot, "src");

suite("Task 00039 Phase C - bounded layout and runtime cleanup", () => {
    test("chat-input runtime uses CodeMirror measurement APIs for bounded auto-grow", () => {
        const runtimeSource = fs.readFileSync(
            path.join(mediaDir, "chat-input-runtime.js"),
            "utf-8",
        );
        assert.ok(runtimeSource.includes("requestMeasure({"));
        assert.ok(runtimeSource.includes("view.contentHeight"));
        assert.ok(runtimeSource.includes("scrollDOM.style.height"));
        assert.ok(
            runtimeSource.includes(
                'scrollDOM.style.overflowY = measurement.shouldScroll ? "auto" : "hidden"',
            ),
        );
        assert.ok(runtimeSource.includes("new ResizeObserver(function () {"));
        assert.ok(runtimeSource.includes("MIN_EDITOR_HEIGHT_PX = 64"));
        assert.ok(runtimeSource.includes("MAX_EDITOR_HEIGHT_PX = 320"));
    });

    test("webview runtime no longer carries legacy contenteditable rewrite helpers", () => {
        const runtimeSource = fs.readFileSync(
            path.join(mediaDir, "chat-input-runtime.js"),
            "utf-8",
        );
        const previewSource = fs.readFileSync(path.join(mediaDir, "preview.js"), "utf-8");
        for (const legacyMarker of [
            "contenteditable",
            "caret-marker",
            "trailing-break-anchor",
            "setCursorCharOffset",
            "renderMarkdownHtml",
            "applyMarkdownHighlight",
        ]) {
            assert.ok(
                !runtimeSource.includes(legacyMarker),
                `runtime must not include ${legacyMarker}`,
            );
            assert.ok(
                !previewSource.includes(legacyMarker),
                `preview must not include ${legacyMarker}`,
            );
        }
    });

    test("agent execution still merges only plain form values into prompt variables", () => {
        const providerSource = fs.readFileSync(
            path.join(sourceRoot, "document-viewer", "governedDocumentEditorProvider.ts"),
            "utf-8",
        );
        assert.ok(
            providerSource.includes(
                "const mergedVars: Record<string, string> = { ...msg.staticInput, ...msg.formValues };",
            ),
        );
        assert.ok(!providerSource.includes("...msg.chatInputMentions"));
    });
});

suite("Task 00039 Phase C - multiline mention regressions", () => {
    test("mention detection remains active after consecutive blank lines", () => {
        const text =
            "# Heading\n\n@frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts";
        const result = detectMentionQuery(text, text.length);
        assert.ok(result !== null);
        assert.strictEqual(
            result.query,
            "frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts",
        );
    });

    test("mention insertion preserves blank lines before the token", () => {
        const text = "Summary\n\n@form";
        const query = detectMentionQuery(text, text.length);
        assert.ok(query !== null);
        const insertion = insertMentionText(text, query, {
            label: "formRenderer.ts",
            path: "frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts",
        });
        assert.strictEqual(
            insertion.text,
            "Summary\n\n@frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts",
        );
        assert.strictEqual(insertion.cursorPos, insertion.text.length);
    });

    test("delete-all reconciliation clears structured mention metadata", () => {
        const mentions = [
            {
                type: "file" as const,
                label: "formRenderer.ts",
                path: "frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts",
            },
        ];
        assert.deepStrictEqual(reconcileMentions("", mentions), []);
        assert.deepStrictEqual(reconcileMentions("\n\n", mentions), []);
    });
});
