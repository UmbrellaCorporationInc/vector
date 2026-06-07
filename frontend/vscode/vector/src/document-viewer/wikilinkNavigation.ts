import type MarkdownIt from "markdown-it";
import type Token from "markdown-it/lib/token.mjs";
import { escapeHtml } from "./previewHtml.js";
import { parseDocIdentifier } from "../docIdentifier.js";

/**
 * Governed file name stem pattern: <type>-<code>-<slug>
 * Example: rfc-00015-extension-owned-governed-document-preview
 */
const GOVERNED_STEM_RE = /^([a-z][a-z0-9-]*?)-(\d+)-(.+)$/;

/**
 * Wikilink pattern: [[stem]] or [[stem|label]].
 */
const WIKILINK_RE = /\[\[([^\]|#\n]+?)(?:\|([^\]#\n]+?))?\]\]/g;

/**
 * Parses a governed file name stem and extracts type, zero-padded code, and slug.
 * Returns null when the stem does not match the governed naming contract.
 */
export function parseGovernedStem(
    stem: string,
): { type: string; code: string; slug: string } | null {
    const match = stem.trim().match(GOVERNED_STEM_RE);
    if (!match) {
        return null;
    }
    return { type: match[1] ?? "", code: match[2] ?? "", slug: match[3] ?? "" };
}

/**
 * markdown-it plugin for the governed preview webview.
 *
 * Transforms [[stem]] tokens into anchor elements whose click dispatches a
 * postMessage to the extension. Only governed stems are converted; unrecognised
 * targets are left as plain text.
 */
export function governedWikilinkPreviewPlugin(md: MarkdownIt): void {
    md.core.ruler.push("governed-wikilink-preview", (state) => {
        for (const blockToken of state.tokens) {
            if (blockToken.type !== "inline" || !blockToken.children) {
                continue;
            }

            const newChildren: Token[] = [];

            for (const token of blockToken.children) {
                if (token.type !== "text") {
                    newChildren.push(token);
                    continue;
                }

                const text = token.content;
                let lastIndex = 0;
                WIKILINK_RE.lastIndex = 0;
                let match: RegExpExecArray | null;

                while ((match = WIKILINK_RE.exec(text)) !== null) {
                    const matchStart = match.index;
                    const matchEnd = matchStart + match[0].length;
                    const stem = (match[1] ?? "").trim();
                    const label = match[2]?.trim() ?? stem;

                    if (matchStart > lastIndex) {
                        const pre = new state.Token("text", "", 0);
                        pre.content = text.slice(lastIndex, matchStart);
                        newChildren.push(pre);
                    }

                    const isGoverned =
                        parseGovernedStem(stem) !== null || parseDocIdentifier(stem) !== null;
                    if (isGoverned) {
                        const htmlToken = new state.Token("html_inline", "", 0);
                        htmlToken.content = buildWikilinkAnchor(stem, label);
                        newChildren.push(htmlToken);
                    } else {
                        const fallback = new state.Token("text", "", 0);
                        fallback.content = match[0];
                        newChildren.push(fallback);
                    }

                    lastIndex = matchEnd;
                }

                if (lastIndex < text.length) {
                    const tail = new state.Token("text", "", 0);
                    tail.content = text.slice(lastIndex);
                    newChildren.push(tail);
                } else if (lastIndex === 0) {
                    newChildren.push(token);
                }
            }

            blockToken.children = newChildren;
        }
    });
}

/**
 * Builds the HTML anchor for a governed wikilink inside the preview webview.
 * The anchor carries a data-wikilink attribute used by the click dispatch script.
 */
function buildWikilinkAnchor(stem: string, label: string): string {
    return (
        `<a href="#" class="vector-wikilink" data-wikilink="${escapeHtml(stem)}">` +
        `${escapeHtml(label)}</a>`
    );
}

/**
 * The message type sent from the webview to the extension when a wikilink is clicked.
 */
export const WIKILINK_MESSAGE_TYPE = "vector.navigateWikilink" as const;

/**
 * The message type sent from the webview when a frontmatter document link is clicked.
 */
export const FM_LINK_MESSAGE_TYPE = "vector.navigateFmLink" as const;

/**
 * Shape of the postMessage payload sent from the webview click handler.
 */
export interface WikilinkMessage {
    type: typeof WIKILINK_MESSAGE_TYPE;
    stem: string;
}

/**
 * Returns true when the given webview message is a governed wikilink navigation request.
 */
export function isWikilinkMessage(msg: unknown): msg is WikilinkMessage {
    return (
        typeof msg === "object" &&
        msg !== null &&
        (msg as Record<string, unknown>)["type"] === WIKILINK_MESSAGE_TYPE &&
        typeof (msg as Record<string, unknown>)["stem"] === "string"
    );
}

/**
 * Shape of the postMessage payload sent when a frontmatter document link is clicked.
 */
export interface FmLinkMessage {
    type: typeof FM_LINK_MESSAGE_TYPE;
    stem: string;
}

/**
 * Returns true when the given webview message is a frontmatter document link navigation request.
 */
export function isFmLinkMessage(msg: unknown): msg is FmLinkMessage {
    return (
        typeof msg === "object" &&
        msg !== null &&
        (msg as Record<string, unknown>)["type"] === FM_LINK_MESSAGE_TYPE &&
        typeof (msg as Record<string, unknown>)["stem"] === "string"
    );
}

/**
 * Inline click-dispatch script injected into the preview HTML shell.
 * Handles clicks on both governed wikilink anchors and frontmatter document links,
 * posting the appropriate message type to the extension for each.
 */
export const WIKILINK_CLICK_SCRIPT = `(function () {
    const vscode = acquireVsCodeApi();
    document.addEventListener("click", function (event) {
        const target = event.target;
        if (!(target instanceof Element)) { return; }
        const wikilink = target.closest("a[data-wikilink]");
        if (wikilink instanceof HTMLAnchorElement) {
            event.preventDefault();
            const stem = wikilink.dataset.wikilink;
            if (stem) { vscode.postMessage({ type: "${WIKILINK_MESSAGE_TYPE}", stem: stem }); }
            return;
        }
        const fmlink = target.closest("a[data-fmlink]");
        if (fmlink instanceof HTMLAnchorElement) {
            event.preventDefault();
            const stem = fmlink.dataset.fmlink;
            if (stem) { vscode.postMessage({ type: "${FM_LINK_MESSAGE_TYPE}", stem: stem }); }
        }
    });
})();`;
