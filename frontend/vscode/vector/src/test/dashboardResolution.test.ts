import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { resolveDashboardSection } from "../dashboardResolution.js";
import type { DashboardSection } from "../dashboardDiscovery.js";
import type { DocumentTypeConfig } from "../documentDiscovery.js";

suite("Dashboard Section Resolution", () => {
    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-dashboard-resolution-test-"));
        if (config !== undefined) {
            const vectorDir = path.join(dir, ".vector");
            fs.mkdirSync(vectorDir, { recursive: true });
            fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    test("resolves status-based sections", () => {
        const dir = makeTempWorkspace();
        try {
            const rfcDir = path.join(dir, "doc", "rfc", "draft");
            fs.mkdirSync(rfcDir, { recursive: true });
            fs.writeFileSync(path.join(rfcDir, "rfc-00001-test.md"), "# Test", "utf-8");

            const section: DashboardSection = {
                title: "Draft RFCs",
                "doc-type": "rfc",
                statuses: ["draft"],
            };
            const docTypes: Record<string, DocumentTypeConfig> = {
                rfc: { layout: "status", "code-width": 5, statuses: ["draft"] },
            };

            const rows = resolveDashboardSection(dir, section, docTypes);
            assert.strictEqual(rows.length, 1);
            const r = rows[0];
            if (!r) {
                assert.fail("Row not found");
            }
            assert.strictEqual(r.status, "draft");
            assert.strictEqual(r.code, "1"); // rfc-00001 -> 1
            assert.strictEqual(r.slug, "test");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolves category-based sections", () => {
        const dir = makeTempWorkspace();
        try {
            const specDir = path.join(dir, "doc", "spec", "api");
            fs.mkdirSync(specDir, { recursive: true });
            fs.writeFileSync(path.join(specDir, "spec-00002-api.md"), "# API", "utf-8");

            const section: DashboardSection = {
                title: "API Specs",
                "doc-type": "spec",
                categories: ["api"],
            };
            const docTypes: Record<string, DocumentTypeConfig> = {
                spec: { layout: "category", "code-width": 5 },
            };

            const rows = resolveDashboardSection(dir, section, docTypes);
            assert.strictEqual(rows.length, 1);
            const r = rows[0];
            if (!r) {
                assert.fail("Row not found");
            }
            assert.strictEqual(r.status, "api"); // Category maps to status column
            assert.strictEqual(r.code, "2"); // spec-00002 -> 2
            assert.strictEqual(r.slug, "api");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("resolves directory-based sections", () => {
        const dir = makeTempWorkspace();
        try {
            const projDir = path.join(dir, "doc", "project");
            fs.mkdirSync(projDir, { recursive: true });
            fs.writeFileSync(
                path.join(projDir, "project-0003-principles.md"),
                "# Principles",
                "utf-8",
            );

            const section: DashboardSection = {
                title: "Principles",
                "doc-type": "project",
            };
            const docTypes: Record<string, DocumentTypeConfig> = {
                project: { layout: "directory", "code-width": 4 },
            };

            const rows = resolveDashboardSection(dir, section, docTypes);
            assert.strictEqual(rows.length, 1);
            const r = rows[0];
            if (!r) {
                assert.fail("Row not found");
            }
            assert.strictEqual(r.status, ""); // Empty for directory layout
            assert.strictEqual(r.code, "3"); // project-0003 -> 3
            assert.strictEqual(r.slug, "principles");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("returns empty array for section errors", () => {
        const section: DashboardSection = {
            title: "Bad",
            "doc-type": "rfc",
            error: "Some error",
        };
        const rows = resolveDashboardSection("/tmp", section, {});
        assert.strictEqual(rows.length, 0);
    });

    test("returns empty array for unknown doc_type", () => {
        const section: DashboardSection = {
            title: "Unknown",
            "doc-type": "unknown",
        };
        const rows = resolveDashboardSection("/tmp", section, {});
        assert.strictEqual(rows.length, 0);
    });
});
