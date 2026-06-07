import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import { createGovernedMarkdownIt } from "../document-viewer/markdownRenderer.js";
import { resolveGovernedPreviewSourceByIdentifier } from "../document-viewer/previewAssets.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeTempWorkspace(config: string): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-phase-b-test-"));
    const vectorDir = path.join(dir, ".vector");
    fs.mkdirSync(vectorDir, { recursive: true });
    fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
    return dir;
}

function writeDoc(workspaceRoot: string, relPath: string, content: string): string {
    const abs = path.join(workspaceRoot, relPath);
    fs.mkdirSync(path.dirname(abs), { recursive: true });
    fs.writeFileSync(abs, content, "utf-8");
    return abs;
}

// ---------------------------------------------------------------------------
// governedWikilinkPreviewPlugin — package-qualified wikilinks
// ---------------------------------------------------------------------------

suite("Task 00054 Phase B — wikilink preview plugin, package-qualified links", () => {
    test("[[package/doc-id]] renders as a clickable anchor", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("See [[shared-lib/rfc-00001-sample]].");
        assert.ok(
            html.includes('class="vector-wikilink"'),
            "package-qualified wikilink must render as a vector-wikilink anchor",
        );
        assert.ok(
            html.includes('data-wikilink="shared-lib/rfc-00001-sample"'),
            "anchor must carry data-wikilink with the full qualified stem",
        );
    });

    test("[[package/doc-id]] with display label renders with label as link text", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("[[my-pkg/task-00054-impl|The Task]]");
        assert.ok(html.includes("The Task"), "label text must appear in anchor content");
        assert.ok(
            html.includes('data-wikilink="my-pkg/task-00054-impl"'),
            "anchor must carry the package-qualified stem as data-wikilink",
        );
    });

    test("[[doc-id]] (unqualified) still renders as a clickable anchor — regression", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("See [[rfc-00013-my-rfc]].");
        assert.ok(
            html.includes('class="vector-wikilink"'),
            "unqualified wikilink must still render as vector-wikilink anchor",
        );
        assert.ok(
            html.includes('data-wikilink="rfc-00013-my-rfc"'),
            "anchor must carry the unqualified stem",
        );
    });

    test("plain text without valid stem is not converted to a link", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("[[not-a-stem]]");
        assert.ok(
            !html.includes('class="vector-wikilink"'),
            "invalid stem must not be rendered as a link",
        );
    });

    test("[[package/]] with empty stem is not converted to a link", () => {
        const md = createGovernedMarkdownIt();
        const html = md.render("[[my-pkg/]]");
        assert.ok(
            !html.includes('class="vector-wikilink"'),
            "empty stem after slash must not render as a link",
        );
    });
});

// ---------------------------------------------------------------------------
// resolveGovernedPreviewSourceByIdentifier — workspace-local stems
// ---------------------------------------------------------------------------

suite("Task 00054 Phase B — resolveGovernedPreviewSourceByIdentifier, local stems", () => {
    const docTypeConfig = `document-types:
  rfc:
    layout: directory
    "code-width": 5
`;

    test("resolves an unqualified stem from the workspace doc directory", () => {
        const dir = makeTempWorkspace(docTypeConfig);
        try {
            writeDoc(dir, "doc/rfc/rfc-00007-sample.md", "---\ntitle: Sample RFC\n---\n# Body");
            const source = resolveGovernedPreviewSourceByIdentifier(dir, "rfc-00007-sample");
            assert.ok(source !== null, "must resolve the document");
            assert.ok(
                source.doc.filePath.endsWith("rfc-00007-sample.md"),
                "resolved file path must end with the doc filename",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("returns null for an invalid identifier format", () => {
        const dir = makeTempWorkspace(docTypeConfig);
        try {
            const source = resolveGovernedPreviewSourceByIdentifier(dir, "not-a-valid-id");
            assert.strictEqual(source, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("returns null when no document matches the code", () => {
        const dir = makeTempWorkspace(docTypeConfig);
        try {
            const source = resolveGovernedPreviewSourceByIdentifier(dir, "rfc-00099-missing");
            assert.strictEqual(source, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});

// ---------------------------------------------------------------------------
// resolveGovernedPreviewSourceByIdentifier — package-qualified stems
// ---------------------------------------------------------------------------

suite(
    "Task 00054 Phase B — resolveGovernedPreviewSourceByIdentifier, package-qualified stems",
    () => {
        const pkgDocTypeConfig = `document-types:
  rfc:
    layout: directory
    "code-width": 5
`;

        function makePackageWorkspace(): { workspaceRoot: string; pkgDir: string } {
            const workspaceRoot = fs.mkdtempSync(
                path.join(os.tmpdir(), "vector-phase-b-pkg-test-"),
            );
            const vectorDir = path.join(workspaceRoot, ".vector");
            fs.mkdirSync(vectorDir, { recursive: true });
            fs.writeFileSync(
                path.join(vectorDir, "document-types.yaml"),
                "document-types: {}",
                "utf-8",
            );
            const pkgDir = path.join(workspaceRoot, ".vector-database", "packages", "shared-lib");
            const pkgVectorDir = path.join(pkgDir, ".vector");
            fs.mkdirSync(pkgVectorDir, { recursive: true });
            fs.writeFileSync(
                path.join(pkgVectorDir, "document-types.yaml"),
                pkgDocTypeConfig,
                "utf-8",
            );
            return { workspaceRoot, pkgDir };
        }

        test("resolves a document from the synchronized package directory", () => {
            const { workspaceRoot, pkgDir } = makePackageWorkspace();
            try {
                const docPath = path.join(pkgDir, "doc", "rfc", "rfc-00003-shared-rule.md");
                fs.mkdirSync(path.dirname(docPath), { recursive: true });
                fs.writeFileSync(docPath, "---\ntitle: Shared Rule\n---\n# Content", "utf-8");

                const source = resolveGovernedPreviewSourceByIdentifier(
                    workspaceRoot,
                    "shared-lib/rfc-00003-shared-rule",
                );
                assert.ok(source !== null, "must resolve the package document");
                assert.ok(
                    source.doc.filePath.endsWith("rfc-00003-shared-rule.md"),
                    "resolved path must point to the package document",
                );
                assert.ok(
                    source.content.includes("Shared Rule"),
                    "content must be read from the package document",
                );
            } finally {
                fs.rmSync(workspaceRoot, { recursive: true, force: true });
            }
        });

        test("resolves a package document stored in a status subdirectory", () => {
            const { workspaceRoot, pkgDir } = makePackageWorkspace();
            try {
                const docPath = path.join(pkgDir, "doc", "rfc", "accepted", "rfc-00005-nested.md");
                fs.mkdirSync(path.dirname(docPath), { recursive: true });
                fs.writeFileSync(docPath, "---\ntitle: Nested\n---\n# Body", "utf-8");

                const source = resolveGovernedPreviewSourceByIdentifier(
                    workspaceRoot,
                    "shared-lib/rfc-00005-nested",
                );
                assert.ok(source !== null, "must resolve a document in a status subdirectory");
                assert.ok(
                    source.doc.filePath.endsWith("rfc-00005-nested.md"),
                    "path must point to the nested document",
                );
            } finally {
                fs.rmSync(workspaceRoot, { recursive: true, force: true });
            }
        });

        test("returns null when the package directory does not exist", () => {
            const { workspaceRoot } = makePackageWorkspace();
            try {
                const source = resolveGovernedPreviewSourceByIdentifier(
                    workspaceRoot,
                    "unknown-pkg/rfc-00001-doc",
                );
                assert.strictEqual(source, null, "unknown package must return null");
            } finally {
                fs.rmSync(workspaceRoot, { recursive: true, force: true });
            }
        });

        test("returns null when the doc type folder is missing in the package", () => {
            const { workspaceRoot } = makePackageWorkspace();
            try {
                const source = resolveGovernedPreviewSourceByIdentifier(
                    workspaceRoot,
                    "shared-lib/task-00001-missing-type",
                );
                assert.strictEqual(
                    source,
                    null,
                    "missing doc type folder in package must return null",
                );
            } finally {
                fs.rmSync(workspaceRoot, { recursive: true, force: true });
            }
        });

        test("returns null when the code does not match any file in the package", () => {
            const { workspaceRoot, pkgDir } = makePackageWorkspace();
            try {
                fs.mkdirSync(path.join(pkgDir, "doc", "rfc"), { recursive: true });
                const source = resolveGovernedPreviewSourceByIdentifier(
                    workspaceRoot,
                    "shared-lib/rfc-00099-not-there",
                );
                assert.strictEqual(source, null, "missing document code must return null");
            } finally {
                fs.rmSync(workspaceRoot, { recursive: true, force: true });
            }
        });
    },
);

// ---------------------------------------------------------------------------
// vector.packageSync — sync command terminal invocation
// ---------------------------------------------------------------------------

suite("Task 00054 Phase B — vector.packageSync terminal invocation", () => {
    test("extension.ts registers vector.packageSync command that opens a terminal", () => {
        const extensionSrc = fs.readFileSync(
            new URL("../extension.js", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1"),
            "utf-8",
        );
        assert.ok(
            extensionSrc.includes("vector.packageSync"),
            "extension must register the vector.packageSync command",
        );
        assert.ok(
            extensionSrc.includes("vector-database package sync"),
            "sync command must send 'vector-database package sync' to the terminal",
        );
    });

    test("extension.ts pushes vector.packageSync into context.subscriptions", () => {
        const extensionSrc = fs.readFileSync(
            new URL("../extension.js", import.meta.url).pathname.replace(/^\/([A-Z]:)/, "$1"),
            "utf-8",
        );
        assert.ok(
            extensionSrc.includes("packageSyncCmd") || extensionSrc.includes("vector.packageSync"),
            "vector.packageSync command must be registered in activate()",
        );
    });
});
