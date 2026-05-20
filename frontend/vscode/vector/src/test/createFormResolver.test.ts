import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { resolveCreateFormSource } from "../createFormResolver.js";
import { loadDocumentTypes } from "../documentDiscovery.js";

function makeTempWorkspace(): string {
    return fs.mkdtempSync(path.join(os.tmpdir(), "vector-resolver-test-"));
}

function writeFile(filePath: string, content = ""): void {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, content, "utf-8");
}

suite("Phase A — Config contracts and create-form resolver", () => {
    suite("DocumentTypeConfig — create-document-form field", () => {
        test("loadDocumentTypes parses create-document-form on a doc type", () => {
            const dir = makeTempWorkspace();
            try {
                const vectorDir = path.join(dir, ".vector");
                fs.mkdirSync(vectorDir, { recursive: true });
                fs.writeFileSync(
                    path.join(vectorDir, "document-types.yaml"),
                    `document-types:
  rfc:
    layout: status
    code-width: 5
    create-document-form: form-00001-create-document
`,
                    "utf-8",
                );

                const config = loadDocumentTypes(dir);
                assert.ok(config !== null);
                assert.strictEqual(
                    config["document-types"]["rfc"]?.["create-document-form"],
                    "form-00001-create-document",
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("loadDocumentTypes treats missing create-document-form as undefined (non-fatal)", () => {
            const dir = makeTempWorkspace();
            try {
                const vectorDir = path.join(dir, ".vector");
                fs.mkdirSync(vectorDir, { recursive: true });
                fs.writeFileSync(
                    path.join(vectorDir, "document-types.yaml"),
                    `document-types:
  task:
    layout: status
    code-width: 5
`,
                    "utf-8",
                );

                const config = loadDocumentTypes(dir);
                assert.ok(config !== null);
                assert.strictEqual(
                    config["document-types"]["task"]?.["create-document-form"],
                    undefined,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });
    });

    suite("DocTypeGlobalConfig — create-document-type-form field", () => {
        test("loadDocumentTypes parses create-document-type-form from doc-type section", () => {
            const dir = makeTempWorkspace();
            try {
                const vectorDir = path.join(dir, ".vector");
                fs.mkdirSync(vectorDir, { recursive: true });
                fs.writeFileSync(
                    path.join(vectorDir, "document-types.yaml"),
                    `document-types: {}
doc-type:
  template: template-00004-doc-type-template
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
`,
                    "utf-8",
                );

                const config = loadDocumentTypes(dir);
                assert.ok(config !== null);
                assert.strictEqual(
                    config["doc-type"]?.["create-document-type-form"],
                    "form-00002-create-document-type",
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("loadDocumentTypes treats absent doc-type section as undefined (non-fatal)", () => {
            const dir = makeTempWorkspace();
            try {
                const vectorDir = path.join(dir, ".vector");
                fs.mkdirSync(vectorDir, { recursive: true });
                fs.writeFileSync(
                    path.join(vectorDir, "document-types.yaml"),
                    `document-types: {}
`,
                    "utf-8",
                );

                const config = loadDocumentTypes(dir);
                assert.ok(config !== null);
                assert.strictEqual(config["doc-type"], undefined);
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });
    });

    suite("resolveCreateFormSource", () => {
        test("resolves an existing governed document in a flat doc-type directory", () => {
            const dir = makeTempWorkspace();
            try {
                const filePath = path.join(dir, "doc", "form", "form-00001-create-document.md");
                writeFile(filePath, "---\ntitle: Create Document\n---\n");

                const result = resolveCreateFormSource(dir, "form-00001-create-document");
                assert.ok(result.ok, `Expected ok but got: ${!result.ok ? result.reason : ""}`);
                assert.strictEqual(result.filePath, filePath);
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("resolves an existing governed document nested in a subdirectory", () => {
            const dir = makeTempWorkspace();
            try {
                const filePath = path.join(
                    dir,
                    "doc",
                    "prompts",
                    "actions",
                    "prompts-00004-execute-task-phase.md",
                );
                writeFile(filePath, "---\ntitle: Execute Task Phase\n---\n");

                const result = resolveCreateFormSource(dir, "prompts-00004-execute-task-phase");
                assert.ok(result.ok, `Expected ok but got: ${!result.ok ? result.reason : ""}`);
                assert.strictEqual(result.filePath, filePath);
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("returns error when governed document is not found", () => {
            const dir = makeTempWorkspace();
            try {
                const result = resolveCreateFormSource(dir, "form-00099-does-not-exist");
                assert.ok(!result.ok);
                assert.ok(
                    result.reason.includes("form-00099-does-not-exist"),
                    `Reason must name the identifier: "${result.reason}"`,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("returns error when multiple files with the same name exist", () => {
            const dir = makeTempWorkspace();
            try {
                const file1 = path.join(
                    dir,
                    "doc",
                    "form",
                    "draft",
                    "form-00001-create-document.md",
                );
                const file2 = path.join(
                    dir,
                    "doc",
                    "form",
                    "published",
                    "form-00001-create-document.md",
                );
                writeFile(file1, "");
                writeFile(file2, "");

                const result = resolveCreateFormSource(dir, "form-00001-create-document");
                assert.ok(!result.ok);
                assert.ok(
                    result.reason.includes("ambiguous"),
                    `Reason must say ambiguous: "${result.reason}"`,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("returns error for a malformed governed identifier", () => {
            const dir = makeTempWorkspace();
            try {
                const result = resolveCreateFormSource(dir, "not-a-valid-id");
                assert.ok(!result.ok);
                assert.ok(
                    result.reason.toLowerCase().includes("malformed"),
                    `Reason must say malformed: "${result.reason}"`,
                );
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });

        test("returns error for an empty identifier", () => {
            const dir = makeTempWorkspace();
            try {
                const result = resolveCreateFormSource(dir, "");
                assert.ok(!result.ok);
            } finally {
                fs.rmSync(dir, { recursive: true, force: true });
            }
        });
    });
});
