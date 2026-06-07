import * as assert from "assert";
import { parseDocIdentifier } from "../docIdentifier.js";

suite("Task 00054 Phase A — parseDocIdentifier", () => {
    // ── workspace-local (unqualified) forms ─────────────────────────────────

    test("parses unqualified single-segment doc type", () => {
        const result = parseDocIdentifier("rfc-00013-my-rfc");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, null);
        assert.strictEqual(result.docType, "rfc");
        assert.strictEqual(result.code, "00013");
        assert.strictEqual(result.slug, "my-rfc");
    });

    test("parses unqualified multi-segment slug", () => {
        const result = parseDocIdentifier("task-00054-implement-rfc-00030-vscode");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, null);
        assert.strictEqual(result.docType, "task");
        assert.strictEqual(result.code, "00054");
        assert.strictEqual(result.slug, "implement-rfc-00030-vscode");
    });

    test("parses unqualified multi-segment doc type (ai-rule)", () => {
        const result = parseDocIdentifier("ai-rule-00000-master-dispatcher");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, null);
        assert.strictEqual(result.docType, "ai-rule");
        assert.strictEqual(result.code, "00000");
        assert.strictEqual(result.slug, "master-dispatcher");
    });

    test("preserves leading zeros in code", () => {
        const result = parseDocIdentifier("spec-00001-api-contract");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.code, "00001");
    });

    // ── package-qualified forms ──────────────────────────────────────────────

    test("parses package-qualified single-segment doc type", () => {
        const result = parseDocIdentifier("my-pkg/rfc-00013-my-rfc");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, "my-pkg");
        assert.strictEqual(result.docType, "rfc");
        assert.strictEqual(result.code, "00013");
        assert.strictEqual(result.slug, "my-rfc");
    });

    test("parses package-qualified multi-segment doc type", () => {
        const result = parseDocIdentifier("shared-lib/ai-rule-00001-some-rule");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, "shared-lib");
        assert.strictEqual(result.docType, "ai-rule");
        assert.strictEqual(result.code, "00001");
        assert.strictEqual(result.slug, "some-rule");
    });

    test("parses package name that itself contains a hyphen", () => {
        const result = parseDocIdentifier("vector-lib/spec-00001-api");
        assert.ok(result !== null, "must parse successfully");
        assert.strictEqual(result.package, "vector-lib");
        assert.strictEqual(result.docType, "spec");
        assert.strictEqual(result.code, "00001");
        assert.strictEqual(result.slug, "api");
    });

    // ── failure cases ────────────────────────────────────────────────────────

    test("returns null for empty string", () => {
        assert.strictEqual(parseDocIdentifier(""), null);
    });

    test("returns null when code segment is missing", () => {
        assert.strictEqual(parseDocIdentifier("rfc-my-rfc"), null);
    });

    test("returns null when slug is missing", () => {
        assert.strictEqual(parseDocIdentifier("rfc-00013"), null);
    });

    test("returns null when code is not numeric", () => {
        assert.strictEqual(parseDocIdentifier("rfc-invalid-my-rfc"), null);
    });

    test("returns null when code is the first segment", () => {
        assert.strictEqual(parseDocIdentifier("00013-rfc-my-rfc"), null);
    });

    test("returns null for leading slash (empty package)", () => {
        assert.strictEqual(parseDocIdentifier("/rfc-00013-my-rfc"), null);
    });

    test("returns null for trailing slash (empty stem)", () => {
        assert.strictEqual(parseDocIdentifier("my-pkg/"), null);
    });

    test("returns null for slash only", () => {
        assert.strictEqual(parseDocIdentifier("/"), null);
    });

    test("returns null for plain word with no separators", () => {
        assert.strictEqual(parseDocIdentifier("invalid"), null);
    });
});
