import markdownIt from "markdown-it";
import type { Options as MarkdownItOptions } from "markdown-it";
import Token from "markdown-it/lib/token.mjs";
import type Renderer from "markdown-it/lib/renderer.mjs";
import { governedCalloutPlugin } from "./calloutPlugin.js";
import {
    createHeadingRenderEnv,
    extractHeadingOutlineFromTokens,
    takeHeadingId,
} from "./headingNavigation.js";
import { escapeHtml } from "./previewHtml.js";
import { governedWikilinkPreviewPlugin } from "./wikilinkNavigation.js";
import { renderFormBlock } from "./form-editor/formRenderer.js";
import { renderOpenDocBlock } from "./document-actions/openDocRenderer.js";
import { renderAgentBlock } from "./document-actions/agentBlockRenderer.js";
import { renderInlineHeaderAction } from "./document-actions/inlineHeaderActionRenderer.js";

export type { default as MarkdownIt } from "markdown-it";
export interface GovernedMarkdownRender {
    html: string;
    headings: ReturnType<typeof extractHeadingOutlineFromTokens>;
}

/**
 * Creates a governed markdown-it instance with plugins and renderer rules applied.
 *
 * Parsing concerns (plugins) and presentation concerns (renderer rules) are kept
 * separate: plugins extend the token stream; renderer rules control HTML output.
 *
 */
export function createGovernedMarkdownIt(): ReturnType<typeof markdownIt> {
    const md = markdownIt({
        html: false,
        linkify: false,
        typographer: false,
    });

    md.use(governedWikilinkPreviewPlugin);
    md.use(governedCalloutPlugin);
    md.core.ruler.after("inline", "vector_task_list_markers", decorateTaskListMarkers);
    applyGovernedRendererRules(md);

    return md;
}

/**
 * Applies governed renderer rules to an existing markdown-it instance.
 * Exported separately so tests can verify rules in isolation.
 */
export function applyGovernedRendererRules(md: ReturnType<typeof markdownIt>): void {
    md.renderer.rules.heading_open = headingOpenRule;
    md.renderer.rules.heading_close = headingCloseRule;
    md.renderer.rules.code_inline = codeInlineRule;
    md.renderer.rules.fence = fenceRule;
    md.renderer.rules.table_open = tableOpenRule;
    md.renderer.rules.table_close = tableCloseRule;
}

function headingOpenRule(
    tokens: Token[],
    idx: number,
    options: unknown,
    env: unknown,
    self: Renderer,
): string {
    const token = tokens[idx];
    if (!token) {
        return "";
    }
    const inline = tokens[idx + 1];
    const headingText = inline?.type === "inline" ? inline.content : "";
    token.attrSet("id", takeHeadingId(env, headingText));
    return self.renderToken(tokens, idx, options as MarkdownItOptions);
}

function headingCloseRule(
    tokens: Token[],
    idx: number,
    options: unknown,
    env: unknown,
    self: Renderer,
): string {
    const token = tokens[idx];
    if (!token) {
        return "";
    }
    const headingEnv = env as { vectorDocumentStem?: string };
    const documentStem = headingEnv.vectorDocumentStem;
    if (!documentStem) {
        return self.renderToken(tokens, idx, options as MarkdownItOptions);
    }
    const inlineToken = tokens[idx - 1];
    const headingText = inlineToken?.type === "inline" ? inlineToken.content : undefined;
    return (
        renderInlineHeaderAction(documentStem, headingText) +
        self.renderToken(tokens, idx, options as MarkdownItOptions)
    );
}

/**
 * Renders inline code as a visually distinct token-like chip.
 */
function codeInlineRule(tokens: Token[], idx: number): string {
    const token = tokens[idx];
    if (!token) {
        return "";
    }
    const content = escapeHtml(token.content);
    return `<code class="vector-inline-code">${content}</code>`;
}

/**
 * Renders fenced code blocks as a dedicated code section.
 * Delegates vector-form blocks to the form-editor renderer.
 */
function fenceRule(tokens: Token[], idx: number): string {
    const token = tokens[idx];
    if (!token) {
        return "";
    }
    const info = token.info ? token.info.trim() : "";
    const lang = info ? (info.split(/\s+/)[0] ?? "") : "";

    if (lang === "vector-form") {
        return renderFormBlock(token.content);
    }

    if (lang === "vector-open-doc") {
        return renderOpenDocBlock(token.content);
    }

    if (lang === "vector-agent-button") {
        return renderAgentBlock(token.content, "button");
    }

    if (lang === "vector-agent-action") {
        return renderAgentBlock(token.content, "action");
    }

    if (lang === "vector-agent-inline-action") {
        return renderAgentBlock(token.content, "inline-action");
    }

    const escapedLang = lang ? escapeHtml(lang) : "";
    const code = escapeHtml(token.content);
    const langAttr = escapedLang ? ` data-lang="${escapedLang}"` : "";
    const codeClass = escapedLang ? ` class="language-${escapedLang}"` : "";
    return `<div class="vector-code-block"${langAttr}><pre><code${codeClass}>${code}</code></pre></div>\n`;
}

/**
 * Renders table opening with a scroll wrapper and governed class.
 */
function tableOpenRule(): string {
    return '<div class="vector-table-wrap"><table class="vector-table">';
}

/**
 * Closes the table scroll wrapper.
 */
function tableCloseRule(): string {
    return "</table></div>\n";
}

function decorateTaskListMarkers(state: { tokens: Token[] }): void {
    for (let idx = 0; idx < state.tokens.length; idx += 1) {
        const token = state.tokens[idx];
        if (
            !token ||
            token.type !== "inline" ||
            !token.children ||
            !isTaskListInline(state.tokens, idx)
        ) {
            continue;
        }

        const firstTextIndex = token.children.findIndex((child) => child.type === "text");
        if (firstTextIndex < 0) {
            continue;
        }

        const firstTextToken = token.children[firstTextIndex];
        if (!firstTextToken) {
            continue;
        }

        const match = firstTextToken.content.match(/^\[( |x|X)\]\s+/);
        if (!match) {
            continue;
        }

        const checked = match[1]?.toLowerCase() === "x";
        const markerToken = new Token("html_inline", "", 0);
        markerToken.content = checked
            ? '<span class="vector-task-marker vector-task-marker--checked">[<span class="vector-task-marker-x">x</span>]</span> '
            : '<span class="vector-task-marker vector-task-marker--unchecked">[ ]</span> ';

        firstTextToken.content = firstTextToken.content.slice(match[0].length);
        token.content = token.content.slice(match[0].length);

        const nextChildren = [...token.children];
        nextChildren.splice(firstTextIndex, 0, markerToken);
        if (firstTextToken.content.length === 0) {
            nextChildren.splice(firstTextIndex + 1, 1);
        }
        token.children = nextChildren;
    }
}

function isTaskListInline(tokens: Token[], idx: number): boolean {
    return tokens[idx - 1]?.type === "paragraph_open" && tokens[idx - 2]?.type === "list_item_open";
}

/**
 * Renders governed Markdown source to an HTML fragment using the governed pipeline.
 */
export function renderGovernedMarkdown(source: string): string {
    return renderGovernedMarkdownAnalysis(source).html;
}

export function renderGovernedMarkdownAnalysis(
    source: string,
    options?: { documentStem?: string },
): GovernedMarkdownRender {
    const md = createGovernedMarkdownIt();
    const tokens = md.parse(source, {});
    const headings = extractHeadingOutlineFromTokens(tokens);
    const html = md.renderer.render(
        tokens,
        md.options,
        createHeadingRenderEnv(headings, options?.documentStem),
    );
    return { html, headings };
}
