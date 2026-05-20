import type MarkdownIt from "markdown-it";
import type Token from "markdown-it/lib/token.mjs";
import { escapeHtml } from "./previewHtml.js";

const CALLOUT_MARKER_RE = /^\[!([A-Za-z][A-Za-z0-9_ -]*)\](?:[ \t]+([^\n]*))?(?:\n|$)/;

/**
 * Transforms blockquotes that start with a governed callout marker into
 * dedicated callout containers with a title row.
 */
export function governedCalloutPlugin(md: MarkdownIt): void {
    md.core.ruler.push("governed-callout-preview", (state) => {
        for (let index = 0; index < state.tokens.length; index += 1) {
            const openToken = state.tokens[index];
            if (openToken?.type !== "blockquote_open") {
                continue;
            }

            const closeIndex = findMatchingBlockquoteClose(state.tokens, index);
            if (closeIndex === -1) {
                continue;
            }

            const firstParagraphOpen = state.tokens[index + 1];
            const firstInline = state.tokens[index + 2];
            const firstParagraphClose = state.tokens[index + 3];

            if (
                firstParagraphOpen?.type !== "paragraph_open" ||
                firstInline?.type !== "inline" ||
                firstParagraphClose?.type !== "paragraph_close"
            ) {
                continue;
            }

            const match = firstInline.content.match(CALLOUT_MARKER_RE);
            if (!match) {
                continue;
            }

            const calloutType = (match[1] ?? "").toLowerCase().replace(/ /g, "-");
            const calloutLabel = (match[1] ?? "").toUpperCase();
            const calloutTitle = match[2]?.trim() ?? "";
            const remainder = firstInline.content.slice(match[0].length);

            openToken.tag = "div";
            openToken.attrJoin("class", `vector-callout vector-callout--${calloutType}`);
            openToken.attrSet("data-callout-type", calloutType);

            const closeToken = state.tokens[closeIndex];
            if (!closeToken) {
                continue;
            }
            closeToken.tag = "div";

            const titleToken = new state.Token("html_block", "", 0);
            titleToken.content = buildCalloutTitleHtml(calloutLabel, calloutTitle);
            state.tokens.splice(index + 1, 0, titleToken);

            if (remainder.length === 0) {
                state.tokens.splice(index + 2, 3);
                index = closeIndex - 2;
                continue;
            }

            firstInline.content = remainder;
            firstInline.children = [];
            state.md.inline.parse(remainder, state.md, state.env, firstInline.children);
            index = closeIndex + 1;
        }
    });
}

function findMatchingBlockquoteClose(tokens: Token[], openIndex: number): number {
    let depth = 0;
    for (let index = openIndex; index < tokens.length; index += 1) {
        const token = tokens[index];
        if (token?.type === "blockquote_open") {
            depth += 1;
        } else if (token?.type === "blockquote_close") {
            depth -= 1;
            if (depth === 0) {
                return index;
            }
        }
    }
    return -1;
}

function buildCalloutTitleHtml(label: string, title: string): string {
    const titleHtml = title
        ? ` <span class="vector-callout-heading">${escapeHtml(title)}</span>`
        : "";
    return (
        `<div class="vector-callout-title">` +
        `<span class="vector-callout-label">${escapeHtml(label)}</span>${titleHtml}</div>\n`
    );
}
