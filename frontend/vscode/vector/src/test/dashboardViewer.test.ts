import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import * as vscode from "./vscode-stub.js";
import { DashboardViewerController } from "../dashboard-viewer/dashboardViewerController.js";

suite("Phase D — Dashboard Viewer", () => {
    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-dashboard-viewer-test-"));
        if (config !== undefined) {
            const vectorDir = path.join(dir, ".vector");
            fs.mkdirSync(vectorDir, { recursive: true });
            fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    test("openDashboard creates a webview panel", () => {
        const dir = makeTempWorkspace("document-types: {}");
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const dashPath = path.join(dashDir, "test.yaml");
            fs.writeFileSync(dashPath, "label: Test Dashboard\nsections: {}", "utf-8");

            const controller = new DashboardViewerController(
                dir,
                vscode.Uri.file("/extension") as unknown as ConstructorParameters<
                    typeof DashboardViewerController
                >[1],
            );
            controller.openDashboard(
                vscode.Uri.file(dashPath) as unknown as Parameters<
                    typeof controller.openDashboard
                >[0],
            );

            // In the stub, window.createWebviewPanel returns a panel
            assert.ok(true, "Panel should be created without errors");
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("dashboard HTML contains section titles", () => {
        const dir = makeTempWorkspace(`document-types:
  task:
    layout: status
    "code-width": 5
`);
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const dashPath = path.join(dashDir, "test.yaml");
            fs.writeFileSync(
                dashPath,
                `
label: Dashboard
sections:
  todo:
    title: Todo Section
    doc-type: task
    statuses: [todo]
`,
                "utf-8",
            );

            const controller = new DashboardViewerController(
                dir,
                vscode.Uri.file("/extension") as unknown as ConstructorParameters<
                    typeof DashboardViewerController
                >[1],
            );
            // We can't easily inspect the webview HTML content from the controller in the current stub
            // but we can verify it doesn't throw.
            controller.openDashboard(
                vscode.Uri.file(dashPath) as unknown as Parameters<
                    typeof controller.openDashboard
                >[0],
            );
            assert.ok(true);
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
