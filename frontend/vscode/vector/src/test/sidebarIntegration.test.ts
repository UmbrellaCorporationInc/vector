import * as assert from "assert";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import * as vscode from "./vscode-stub.js";
import { PerTypeDocumentProvider } from "../governedDocumentProvider.js";

suite("Phase B — Sidebar Integration (Dashboards)", () => {
    function makeTempWorkspace(config?: string): string {
        const dir = fs.mkdtempSync(path.join(os.tmpdir(), "vector-dashboard-sidebar-test-"));
        if (config !== undefined) {
            const vectorDir = path.join(dir, ".vector");
            fs.mkdirSync(vectorDir, { recursive: true });
            fs.writeFileSync(path.join(vectorDir, "document-types.yaml"), config, "utf-8");
        }
        return dir;
    }

    test("getChildren returns dashboard roots when dashboards exist", () => {
        const dir = makeTempWorkspace("document-types: {}");
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            fs.writeFileSync(
                path.join(dashDir, "status.yaml"),
                "label: Status\nsections: {}",
                "utf-8",
            );

            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren();

            const dashboardNodes = children.filter((c) => c.kind === "dashboard");
            assert.strictEqual(dashboardNodes.length, 1, "Must find one dashboard node");
            if (dashboardNodes[0]?.kind === "dashboard") {
                assert.strictEqual(dashboardNodes[0].dashboard.label, "Status");
            } else {
                assert.fail("Node kind must be dashboard");
            }
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("getTreeItem for dashboard node uses dashboard icon and label", () => {
        const dashboard = {
            label: "Project Overview",
            sections: {},
            filePath: "/path/to/dash.yaml",
        };
        const node = { kind: "dashboard" as const, dashboard };
        const provider = new PerTypeDocumentProvider("/tmp");

        const item = provider.getTreeItem(node);

        assert.strictEqual(item.label, "Project Overview");
        assert.strictEqual(item.contextValue, "dashboard");
        assert.ok(item.iconPath instanceof vscode.ThemeIcon);
        assert.strictEqual(item.iconPath.id, "dashboard");
        assert.strictEqual(item.command?.command, "vector.openDashboard");
        assert.strictEqual(
            item.command.arguments?.[0]?.fsPath,
            vscode.Uri.file(dashboard.filePath).fsPath,
        );
    });

    test("getChildren returns both dashboards and doc type roots", () => {
        const config = `document-types:
  rfc:
    layout: status
    "code-width": 5
`;
        const dir = makeTempWorkspace(config);
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            fs.writeFileSync(path.join(dashDir, "d1.yaml"), "label: D1\nsections: {}", "utf-8");

            const provider = new PerTypeDocumentProvider(dir);
            const children = provider.getChildren();

            assert.strictEqual(children.length, 2, "Must find dashboard + rfc root");
            assert.ok(children.some((c) => c.kind === "dashboard" && c.dashboard.label === "D1"));
            assert.ok(children.some((c) => c.kind === "root" && c.docType === "rfc"));
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });

    test("refresh rebuilds dashboard items", () => {
        const dir = makeTempWorkspace("document-types: {}");
        try {
            const dashDir = path.join(dir, ".vector", "dashboards");
            fs.mkdirSync(dashDir, { recursive: true });
            const provider = new PerTypeDocumentProvider(dir);

            assert.strictEqual(provider.getChildren().length, 0);

            fs.writeFileSync(path.join(dashDir, "new.yaml"), "label: New\nsections: {}", "utf-8");
            provider.refresh();

            const children = provider.getChildren();
            assert.strictEqual(children.length, 1);
            if (children[0]?.kind === "dashboard") {
                assert.strictEqual(children[0].dashboard.label, "New");
            }
        } finally {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
});
