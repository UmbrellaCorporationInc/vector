import * as assert from "assert";
import {
    FILE_SUGGESTIONS_REQUEST_TYPE,
    FILE_SUGGESTIONS_RESULT_TYPE,
    isFileSuggestionsRequest,
    isFileSuggestionsResult,
} from "../document-viewer/chat-input/chatInputMessaging.js";

suite("Phase A — chat-input module: messaging contracts", () => {
    suite("isFileSuggestionsRequest", () => {
        test("accepts a valid request message", () => {
            const msg = {
                type: FILE_SUGGESTIONS_REQUEST_TYPE,
                requestId: "req-1",
                query: "formRenderer",
            };
            assert.ok(isFileSuggestionsRequest(msg));
        });

        test("rejects null", () => {
            assert.ok(!isFileSuggestionsRequest(null));
        });

        test("rejects a non-object primitive", () => {
            assert.ok(!isFileSuggestionsRequest("string"));
            assert.ok(!isFileSuggestionsRequest(42));
        });

        test("rejects wrong message type", () => {
            const msg = { type: "vector.chatInput.other", requestId: "req-1", query: "x" };
            assert.ok(!isFileSuggestionsRequest(msg));
        });

        test("rejects missing requestId", () => {
            const msg = { type: FILE_SUGGESTIONS_REQUEST_TYPE, query: "x" };
            assert.ok(!isFileSuggestionsRequest(msg));
        });

        test("rejects missing query", () => {
            const msg = { type: FILE_SUGGESTIONS_REQUEST_TYPE, requestId: "req-1" };
            assert.ok(!isFileSuggestionsRequest(msg));
        });

        test("rejects numeric requestId", () => {
            const msg = { type: FILE_SUGGESTIONS_REQUEST_TYPE, requestId: 1, query: "x" };
            assert.ok(!isFileSuggestionsRequest(msg));
        });
    });

    suite("isFileSuggestionsResult", () => {
        test("accepts a valid result message with an empty suggestions array", () => {
            const msg = {
                type: FILE_SUGGESTIONS_RESULT_TYPE,
                requestId: "req-1",
                suggestions: [],
            };
            assert.ok(isFileSuggestionsResult(msg));
        });

        test("accepts a valid result message with suggestion items", () => {
            const msg = {
                type: FILE_SUGGESTIONS_RESULT_TYPE,
                requestId: "req-1",
                suggestions: [
                    {
                        label: "formRenderer.ts",
                        path: "src/document-viewer/form-editor/formRenderer.ts",
                    },
                ],
            };
            assert.ok(isFileSuggestionsResult(msg));
        });

        test("rejects null", () => {
            assert.ok(!isFileSuggestionsResult(null));
        });

        test("rejects wrong message type", () => {
            const msg = { type: "vector.chatInput.other", requestId: "req-1", suggestions: [] };
            assert.ok(!isFileSuggestionsResult(msg));
        });

        test("rejects missing suggestions field", () => {
            const msg = { type: FILE_SUGGESTIONS_RESULT_TYPE, requestId: "req-1" };
            assert.ok(!isFileSuggestionsResult(msg));
        });

        test("rejects suggestions that is not an array", () => {
            const msg = {
                type: FILE_SUGGESTIONS_RESULT_TYPE,
                requestId: "req-1",
                suggestions: "bad",
            };
            assert.ok(!isFileSuggestionsResult(msg));
        });

        test("rejects missing requestId", () => {
            const msg = { type: FILE_SUGGESTIONS_RESULT_TYPE, suggestions: [] };
            assert.ok(!isFileSuggestionsResult(msg));
        });
    });

    suite("message type constants", () => {
        test("request type constant is the expected string", () => {
            assert.strictEqual(
                FILE_SUGGESTIONS_REQUEST_TYPE,
                "vector.chatInput.requestSuggestions",
            );
        });

        test("result type constant is the expected string", () => {
            assert.strictEqual(FILE_SUGGESTIONS_RESULT_TYPE, "vector.chatInput.suggestionsResult");
        });
    });
});
