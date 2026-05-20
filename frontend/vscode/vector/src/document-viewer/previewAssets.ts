import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { resolveDocumentByCode } from "../governedDocumentProvider.js";
import { parseGovernedStem } from "./wikilinkNavigation.js";
import type { GovernedDocument } from "../documentDiscovery.js";
import type { PreviewAssetUris } from "./previewHtml.js";

/**
 * Resolved source for a governed preview render pass.
 */
export interface GovernedPreviewSource {
    doc: GovernedDocument;
    content: string;
}

/**
 * Reads the content of a governed document from disk.
 * Returns null when the file is unreadable.
 */
export function readGovernedDocumentContent(filePath: string): string | null {
    try {
        return fs.readFileSync(filePath, "utf-8");
    } catch {
        return null;
    }
}

/**
 * Resolves a governed document by its file stem using the governed lookup boundary.
 * Returns null when the stem is not a valid governed stem or when no document matches.
 */
export function resolveGovernedPreviewSource(
    workspaceRoot: string,
    stem: string,
): GovernedPreviewSource | null {
    const parsed = parseGovernedStem(stem);
    if (!parsed) {
        return null;
    }
    const doc = resolveDocumentByCode(workspaceRoot, parsed.type, parsed.code);
    if (!doc) {
        return null;
    }
    const content = readGovernedDocumentContent(doc.filePath);
    if (content === null) {
        return null;
    }
    return { doc, content };
}

/**
 * Resolves and validates all preview asset URIs for the given extension and webview.
 * Returns null when any required asset file is missing from the extension package.
 */
export function buildPreviewAssets(
    extensionUri: vscode.Uri,
    webview: vscode.Webview,
): PreviewAssetUris | null {
    const media = (name: string) => path.join(extensionUri.fsPath, "media", name);
    const node = (...segments: string[]) =>
        path.join(extensionUri.fsPath, "node_modules", ...segments);

    const stylePath = media("preview.css");
    const scriptPath = media("preview.js");
    const chatInputRuntimePath = media("chat-input-runtime.js");
    const hlStylePath = media("hljs-theme.css");
    const hlScriptPath = media("hljs.min.js");

    const codeMirrorImportPaths = {
        "@codemirror/state": node("@codemirror", "state", "dist", "index.js"),
        "@codemirror/view": node("@codemirror", "view", "dist", "index.js"),
        "@codemirror/autocomplete": node("@codemirror", "autocomplete", "dist", "index.js"),
        "@codemirror/commands": node("@codemirror", "commands", "dist", "index.js"),
        "@codemirror/language": node("@codemirror", "language", "dist", "index.js"),
        "@lezer/common": node("@lezer", "common", "dist", "index.js"),
        "@lezer/highlight": node("@lezer", "highlight", "dist", "index.js"),
        "@marijn/find-cluster-break": node("@marijn", "find-cluster-break", "src", "index.js"),
        crelt: node("crelt", "index.js"),
        "style-mod": node("style-mod", "src", "style-mod.js"),
        "w3c-keyname": node("w3c-keyname", "index.js"),
    } as const;

    if (
        !fs.existsSync(stylePath) ||
        !fs.existsSync(scriptPath) ||
        !fs.existsSync(chatInputRuntimePath) ||
        !fs.existsSync(hlStylePath) ||
        !fs.existsSync(hlScriptPath) ||
        Object.values(codeMirrorImportPaths).some((p) => !fs.existsSync(p))
    ) {
        void vscode.window.showErrorMessage(
            "Vector: preview resources are missing from the extension package.",
        );
        return null;
    }

    const toUri = (p: string) => webview.asWebviewUri(vscode.Uri.file(p)).toString();
    return {
        styleUri: toUri(stylePath),
        scriptUri: toUri(scriptPath),
        chatInputRuntimeUri: toUri(chatInputRuntimePath),
        highlightStyleUri: toUri(hlStylePath),
        highlightScriptUri: toUri(hlScriptPath),
        codeMirrorImportMap: Object.fromEntries(
            Object.entries(codeMirrorImportPaths).map(([key, p]) => [key, toUri(p)]),
        ),
    };
}
