import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { resolveDocumentByCode } from "../governedDocumentProvider.js";
import { parseGovernedStem } from "./wikilinkNavigation.js";
import { parseDocIdentifier } from "../docIdentifier.js";
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
 * Resolves a governed document by a stem that may be unqualified (`type-code-slug`) or
 * package-qualified (`package/type-code-slug`).
 *
 * - Unqualified stems are resolved within the active workspace using `resolveGovernedPreviewSource`.
 * - Package-qualified stems are resolved against `.vector-database/packages/<package>/`.
 *
 * Returns null when the identifier is invalid, the package is unknown, or no document matches.
 */
export function resolveGovernedPreviewSourceByIdentifier(
    workspaceRoot: string,
    stem: string,
): GovernedPreviewSource | null {
    const id = parseDocIdentifier(stem);
    if (!id) {
        return null;
    }
    if (id.package === null) {
        return resolveGovernedPreviewSource(workspaceRoot, stem);
    }
    return resolvePackageGovernedDocument(workspaceRoot, id.package, id.docType, id.code);
}

function resolvePackageGovernedDocument(
    workspaceRoot: string,
    pkgName: string,
    docType: string,
    code: string,
): GovernedPreviewSource | null {
    const pkgDir = path.join(workspaceRoot, ".vector-database", "packages", pkgName);
    if (!fs.existsSync(pkgDir)) {
        return null;
    }
    const docDir = path.join(pkgDir, "doc", docType);
    if (!fs.existsSync(docDir)) {
        return null;
    }
    const filePath = findFileByCode(docDir, docType, code);
    if (!filePath) {
        return null;
    }
    const content = readGovernedDocumentContent(filePath);
    if (content === null) {
        return null;
    }
    const fileStem = path.basename(filePath, ".md");
    const parsed = parseGovernedStem(fileStem);
    const doc: GovernedDocument = {
        type: parsed?.type ?? docType,
        code: parsed?.code ?? code,
        slug: parsed?.slug ?? fileStem,
        title: parsed ? `${parsed.type.toUpperCase()} ${parsed.code}` : fileStem,
        filePath,
    };
    return { doc, content };
}

function findFileByCode(docDir: string, docType: string, code: string): string | null {
    const codeNum = Number.parseInt(code, 10);
    if (Number.isNaN(codeNum)) {
        return null;
    }
    let entries: fs.Dirent[];
    try {
        entries = fs.readdirSync(docDir, { withFileTypes: true });
    } catch {
        return null;
    }
    for (const entry of entries) {
        const entryPath = path.join(docDir, entry.name);
        if (entry.isDirectory()) {
            const result = findFileByCode(entryPath, docType, code);
            if (result) {
                return result;
            }
        } else if (entry.isFile() && entry.name.endsWith(".md")) {
            const stem = path.basename(entry.name, ".md");
            const parsed = parseGovernedStem(stem);
            if (parsed && parsed.type === docType && Number.parseInt(parsed.code, 10) === codeNum) {
                return entryPath;
            }
        }
    }
    return null;
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
