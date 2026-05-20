import type * as vscode from "vscode";
import type { HeadingEntry } from "./headingNavigation.js";

export interface PreviewAssetUris {
    scriptUri: string;
    chatInputRuntimeUri: string;
    styleUri: string;
    highlightScriptUri: string;
    highlightStyleUri: string;
    codeMirrorImportMap: Readonly<Record<string, string>>;
}

export interface PreviewToolbarState {
    headings: readonly HeadingEntry[];
}

export function escapeHtml(s: string): string {
    return s
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;");
}

/**
 * Builds the HTML shell used by the governed preview WebviewPanel.
 * Accepts pre-rendered frontmatter and markdown fragments plus the local
 * webview-safe URIs for the preview stylesheet and interaction script.
 */
export function buildPreviewHtml(
    webview: vscode.Webview,
    title: string,
    bodyHtml: string,
    assets: PreviewAssetUris,
    frontmatterHtml?: string,
    toolbarState?: PreviewToolbarState,
): string {
    const fmSection = frontmatterHtml ? `${frontmatterHtml}\n` : "";
    const toolbarSection = buildToolbarHtml(toolbarState);
    const nonce = "vector-preview-runtime";
    const importMapJson = JSON.stringify({ imports: assets.codeMirrorImportMap });
    const styleSrc = `style-src ${webview.cspSource} 'unsafe-inline';`;
    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; ${styleSrc} script-src ${webview.cspSource} 'nonce-${nonce}';" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<meta name="vector-csp-nonce" content="${nonce}" />
<title>${escapeHtml(title)}</title>
<link rel="stylesheet" href="${assets.highlightStyleUri}" />
<link rel="stylesheet" href="${assets.styleUri}" />
</head>
<body>
${toolbarSection}
${fmSection}${bodyHtml}
<script src="${assets.highlightScriptUri}"></script>
<script type="importmap" nonce="${nonce}">${importMapJson}</script>
<script type="module" nonce="${nonce}" src="${assets.chatInputRuntimeUri}"></script>
<script src="${assets.scriptUri}"></script>
</body>
</html>`;
}

function buildToolbarHtml(toolbarState?: PreviewToolbarState): string {
    const headings = toolbarState?.headings ?? [];
    const tocItems =
        headings.length === 0
            ? '<div class="vector-toc-empty">No headings found.</div>'
            : headings
                  .map(
                      (heading) =>
                          `<button class="vector-toc-item" data-heading-id="${escapeHtml(heading.id)}" data-level="${String(heading.level)}" type="button">` +
                          escapeHtml(heading.text) +
                          `</button>`,
                  )
                  .join("\n");

    return `<aside class="vector-toc-panel" data-toc-panel hidden>
  <div class="vector-toc-header">Table of Contents</div>
  <div class="vector-toc-items">${tocItems}</div>
</aside>`;
}
