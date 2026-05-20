import type * as vscode from "vscode";
import { escapeHtml } from "../document-viewer/previewHtml.js";
import type { DashboardRow } from "../dashboardResolution.js";

export interface DashboardAssetUris {
    styleUri: string;
    scriptUri: string;
}

function renderSection(section: {
    title: string;
    rows: DashboardRow[];
    error?: string | undefined;
    layout?: "status" | "category" | "directory" | undefined;
}): string {
    let contentHtml: string;
    if (section.error) {
        contentHtml = `<div class="vector-dashboard-error"><strong>Configuration Error:</strong> ${escapeHtml(section.error)}</div>`;
    } else if (section.rows.length === 0) {
        contentHtml = `<div class="vector-dashboard-empty">No documents found matching this section criteria.</div>`;
    } else {
        const layout = section.layout ?? "status";
        const hasGroupingColumn = layout !== "directory";
        const groupLabel = layout === "category" ? "Category" : "Status";

        const rowsHtml = section.rows
            .map((row) => {
                const groupingTd = hasGroupingColumn
                    ? `<td style="width: 120px; font-family: var(--vscode-editor-font-family); font-size: 0.85rem; color: var(--vscode-descriptionForeground);">${escapeHtml(row.status || "-")}</td>`
                    : "";

                return `
            <tr>
                ${groupingTd}
                <td style="width: 80px; font-family: var(--vscode-editor-font-family); font-size: 0.85rem; color: var(--vscode-descriptionForeground);">${escapeHtml(row.code)}</td>
                <td><button class="vector-dashboard-slug" onclick="openDocument('${escapeHtml(row.stem)}')" type="button">${escapeHtml(row.slug)}</button></td>
            </tr>`;
            })
            .join("");

        const groupTh = hasGroupingColumn ? `<th>${groupLabel}</th>` : "";

        contentHtml = `
            <table>
                <thead>
                    <tr>
                        ${groupTh}
                        <th>Code</th>
                        <th>Document Slug</th>
                    </tr>
                </thead>
                <tbody>
                    ${rowsHtml}
                </tbody>
            </table>`;
    }

    return `
    <div class="vector-dashboard-section">
        <h2>${escapeHtml(section.title)}</h2>
        ${contentHtml}
    </div>`;
}

/**
 * Builds the full HTML for a dashboard view.
 */
export function buildDashboardHtml(
    webview: vscode.Webview,
    title: string,
    sections: Record<
        string,
        {
            title: string;
            rows: DashboardRow[];
            error?: string | undefined;
            layout?: "status" | "category" | "directory" | undefined;
        }
    >,
    assets: DashboardAssetUris,
): string {
    const sectionsHtml = Object.values(sections)
        .map((section) => renderSection(section))
        .join("\n");

    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src ${webview.cspSource} 'unsafe-inline'; script-src ${webview.cspSource} 'unsafe-inline';" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>${escapeHtml(title)}</title>
<link rel="stylesheet" href="${assets.styleUri}" />
<style>
    body { font-family: var(--vscode-font-family); color: var(--vscode-editor-foreground); padding: 1.5rem; line-height: 1.4; }
    h1 { font-size: 1.8rem; margin-top: 0; margin-bottom: 2rem; border-bottom: 2px solid var(--vscode-panel-border); padding-bottom: 0.5rem; }
    .vector-dashboard-section { margin-bottom: 3rem; }
    .vector-dashboard-section h2 { font-size: 1.2rem; margin-bottom: 1rem; color: var(--vscode-symbolIcon-propertyForeground); display: flex; align-items: center; }
    .vector-dashboard-section h2::after { content: ""; flex: 1; margin-left: 1rem; height: 1px; background: var(--vscode-panel-border); }
    .vector-dashboard-error { color: var(--vscode-errorForeground); font-size: 0.9rem; margin-bottom: 0.5rem; padding: 0.5rem; background: var(--vscode-inputValidation-errorBackground); border: 1px solid var(--vscode-inputValidation-errorBorder); border-radius: 2px; }
    table { width: 100%; border-collapse: collapse; border: 1px solid var(--vscode-panel-border); background: var(--vscode-editor-background); box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
    th { text-align: left; padding: 0.8rem; border-bottom: 2px solid var(--vscode-panel-border); font-size: 0.75rem; text-transform: uppercase; color: var(--vscode-descriptionForeground); font-weight: bold; background: var(--vscode-sideBar-background); }
    td { padding: 0.8rem; border-bottom: 1px solid var(--vscode-panel-border); font-size: 0.95rem; }
    tr:nth-child(even) { background-color: var(--vscode-list-hoverBackground); }
    tr:hover { background-color: var(--vscode-list-activeSelectionBackground); color: var(--vscode-list-activeSelectionForeground); }
    tr:hover .vector-dashboard-slug { color: inherit; }
    .vector-dashboard-slug {
        background: none;
        border: none;
        padding: 0;
        margin: 0;
        color: var(--vscode-textLink-foreground);
        cursor: pointer;
        font-family: inherit;
        font-size: inherit;
        font-weight: 500;
        text-decoration: none;
        text-align: left;
    }
    .vector-dashboard-slug:hover { text-decoration: underline; color: var(--vscode-textLink-activeForeground); }
    .vector-dashboard-empty { color: var(--vscode-descriptionForeground); font-style: italic; font-size: 0.9rem; padding: 1rem; border: 1px dashed var(--vscode-panel-border); background: var(--vscode-sideBar-background); opacity: 0.7; }
</style>
</head>
<body>
    <h1>${escapeHtml(title)}</h1>
    ${sectionsHtml}
    <script>
        const vscode = acquireVsCodeApi();
        function openDocument(stem) {
            vscode.postMessage({
                type: 'vector.openDocument',
                stem: stem
            });
        }
    </script>
</body>
</html>`;
}
