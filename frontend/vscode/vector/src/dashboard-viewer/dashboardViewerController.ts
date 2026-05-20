import * as path from "path";
import * as vscode from "vscode";
import { loadDashboard } from "../dashboardDiscovery.js";
import { resolveDashboardSection, type DashboardRow } from "../dashboardResolution.js";
import { loadDocumentTypes } from "../documentDiscovery.js";
import { buildDashboardHtml, type DashboardAssetUris } from "./dashboardHtml.js";

/**
 * Controller for the reusable dashboard viewer panel.
 */
export class DashboardViewerController implements vscode.Disposable {
    private panel: vscode.WebviewPanel | undefined;
    private currentDashboardPath: string | undefined;
    private readonly workspaceRoot: string;
    private readonly extensionUri: vscode.Uri;
    private readonly subscriptions: vscode.Disposable[] = [];

    constructor(workspaceRoot: string, extensionUri: vscode.Uri) {
        this.workspaceRoot = workspaceRoot;
        this.extensionUri = extensionUri;
    }

    /**
     * Opens or reveals the dashboard for the given YAML file URI.
     */
    openDashboard(uri: vscode.Uri): void {
        this.currentDashboardPath = uri.fsPath;
        this._render();
    }

    dispose(): void {
        this.panel?.dispose();
        for (const sub of this.subscriptions) {
            sub.dispose();
        }
    }

    /**
     * Re-renders the currently open dashboard.
     */
    refresh(): void {
        this._render();
    }

    private _render(): void {
        if (!this.currentDashboardPath) {
            return;
        }

        const docTypesYaml = loadDocumentTypes(this.workspaceRoot);
        const docTypes = docTypesYaml?.["document-types"] ?? {};

        const dashboard = loadDashboard(this.currentDashboardPath, docTypes);
        if (!dashboard) {
            void vscode.window.showErrorMessage(
                `Vector: cannot load dashboard: ${this.currentDashboardPath}`,
            );
            return;
        }

        const panel = this._getOrCreatePanel(dashboard.label);
        const assets = this._getDashboardAssets(panel.webview);
        if (!assets) {
            return;
        }

        const resolvedSections: Record<
            string,
            {
                title: string;
                rows: DashboardRow[];
                error?: string | undefined;
                layout?: "status" | "category" | "directory" | undefined;
            }
        > = {};
        for (const [key, section] of Object.entries(dashboard.sections)) {
            const config = docTypes[section["doc-type"]];
            resolvedSections[key] = {
                title: section.title,
                rows: resolveDashboardSection(this.workspaceRoot, section, docTypes),
                error: section.error,
                layout: config?.layout,
            };
        }

        panel.webview.html = buildDashboardHtml(
            panel.webview,
            dashboard.label,
            resolvedSections,
            assets,
        );
        panel.title = dashboard.label;
        panel.reveal(vscode.ViewColumn.Beside, true);
    }

    private _getOrCreatePanel(title: string): vscode.WebviewPanel {
        if (this.panel) {
            return this.panel;
        }

        const panel = vscode.window.createWebviewPanel(
            "vectorDashboardViewer",
            title,
            { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
            {
                enableScripts: true,
                localResourceRoots: [vscode.Uri.joinPath(this.extensionUri, "media")],
                retainContextWhenHidden: false,
            },
        );

        panel.onDidDispose(() => {
            this.panel = undefined;
            this.currentDashboardPath = undefined;
        });

        panel.webview.onDidReceiveMessage((msg: unknown) => {
            if (isOpenDocumentMessage(msg)) {
                void vscode.commands.executeCommand("vector.openStem", msg.stem);
            }
        });

        this.panel = panel;
        return panel;
    }

    private _getDashboardAssets(webview: vscode.Webview): DashboardAssetUris | null {
        const stylePath = path.join(this.extensionUri.fsPath, "media", "preview.css");
        const scriptPath = path.join(this.extensionUri.fsPath, "media", "preview.js");

        return {
            styleUri: webview.asWebviewUri(vscode.Uri.file(stylePath)).toString(),
            scriptUri: webview.asWebviewUri(vscode.Uri.file(scriptPath)).toString(),
        };
    }
}

interface OpenDocumentMessage {
    type: "vector.openDocument";
    stem: string;
}

function isOpenDocumentMessage(message: unknown): message is OpenDocumentMessage {
    if (typeof message !== "object" || message === null) {
        return false;
    }
    const m = message as Record<string, unknown>;
    return m.type === "vector.openDocument" && typeof m.stem === "string";
}
