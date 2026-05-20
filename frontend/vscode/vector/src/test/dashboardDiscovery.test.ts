import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { scanDashboards, loadDashboard } from "../dashboardDiscovery.js";
import type { DocumentTypeConfig } from "../documentDiscovery.js";

suite("Dashboard Discovery", () => {
    function makeTempWorkspace(): string {
        return fs.mkdtempSync(path.join(os.tmpdir(), "vector-dashboard-test-"));
    }

    test("scanDashboards returns empty array when .vector/dashboards is missing", () => {
        const dir = makeTempWorkspace();
        try {
            const results = scanDashboards(dir);
            assert.deepStrictEqual(results, []);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("scanDashboards discovers valid dashboards", () => {
        const dir = makeTempWorkspace();
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });

            const yamlContent = `
label: Test Dashboard
sections:
  todo:
    title: Todo
    doc-type: task
    statuses: [todo]
`;
            fs.writeFileSync(path.join(dashDir, "test.yaml"), yamlContent, "utf-8");

            const results = scanDashboards(dir);
            assert.strictEqual(results.length, 1);
            const dash = results[0];
            if (!dash) {
                assert.fail("Dashboard not discovered");
            }
            assert.strictEqual(dash.label, "Test Dashboard");
            const todoSection = dash.sections["todo"];
            if (!todoSection) {
                assert.fail("Section 'todo' not found");
            }
            assert.strictEqual(todoSection.title, "Todo");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadDashboard returns null for missing label", () => {
        const dir = makeTempWorkspace();
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const filePath = path.join(dashDir, "invalid.yaml");
            fs.writeFileSync(filePath, "sections: {}", "utf-8");

            const result = loadDashboard(filePath);
            assert.strictEqual(result, null);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadDashboard captures section errors for missing doc_type", () => {
        const dir = makeTempWorkspace();
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const filePath = path.join(dashDir, "partial.yaml");
            const yamlContent = `
label: Partial
sections:
  bad:
    title: Bad Section
`;
            fs.writeFileSync(filePath, yamlContent, "utf-8");

            const result = loadDashboard(filePath);
            if (!result) {
                assert.fail("Dashboard not loaded");
            }
            const badSection = result.sections["bad"];
            if (!badSection) {
                assert.fail("Section 'bad' not found");
            }
            assert.strictEqual(badSection.error, "Missing doc-type");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadDashboard validates layout-aware filters", () => {
        const dir = makeTempWorkspace();
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const filePath = path.join(dashDir, "layout-errors.yaml");
            const yamlContent = `
label: Layout Errors
sections:
  wrong-status:
    doc-type: rfc
    categories: [api]
  wrong-category:
    doc-type: spec
    statuses: [todo]
  directory-with-filter:
    doc-type: project
    statuses: [done]
`;
            fs.writeFileSync(filePath, yamlContent, "utf-8");

            const docTypes: Record<string, DocumentTypeConfig> = {
                rfc: { layout: "status", "code-width": 5 },
                spec: { layout: "category", "code-width": 5 },
                project: { layout: "directory", "code-width": 5 },
            };

            const result = loadDashboard(filePath, docTypes);
            if (!result) {
                assert.fail("Dashboard not loaded");
            }
            const s1 = result.sections["wrong-status"];
            const s2 = result.sections["wrong-category"];
            const s3 = result.sections["directory-with-filter"];

            assert.strictEqual(
                s1?.error,
                "Section for 'status' layout must provide 'statuses' array",
            );
            assert.strictEqual(
                s2?.error,
                "Section for 'category' layout must provide 'categories' array",
            );
            assert.strictEqual(
                s3?.error,
                "Section for 'directory' layout should not provide filters",
            );
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("loadDashboard parses valid filters when docTypes are provided", () => {
        const dir = makeTempWorkspace();
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const filePath = path.join(dashDir, "valid.yaml");
            const yamlContent = `
label: Valid
sections:
  tasks:
    doc-type: task
    statuses: [todo]
  specs:
    doc-type: spec
    categories: [api]
`;
            fs.writeFileSync(filePath, yamlContent, "utf-8");

            const docTypes: Record<string, DocumentTypeConfig> = {
                task: { layout: "status", "code-width": 5 },
                spec: { layout: "category", "code-width": 5 },
            };

            const result = loadDashboard(filePath, docTypes);
            if (!result) {
                assert.fail("Dashboard not loaded");
            }
            const tasks = result.sections["tasks"];
            const specs = result.sections["specs"];

            assert.strictEqual(tasks?.error, undefined);
            assert.deepStrictEqual(tasks?.statuses, ["todo"]);
            assert.strictEqual(specs?.error, undefined);
            assert.deepStrictEqual(specs?.categories, ["api"]);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
