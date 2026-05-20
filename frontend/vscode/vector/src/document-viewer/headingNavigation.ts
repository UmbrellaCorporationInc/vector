import markdownIt from "markdown-it";
import type Token from "markdown-it/lib/token.mjs";

export interface HeadingEntry {
    level: number;
    text: string;
    id: string;
}

interface HeadingRenderEnv {
    vectorHeadingIds?: string[];
    vectorHeadingIndex?: number;
    vectorDocumentStem?: string;
}

/**
 * Extracts the heading outline used by the governed preview table of contents.
 */
export function extractHeadingOutline(source: string): HeadingEntry[] {
    const md = markdownIt({
        html: false,
        linkify: false,
        typographer: false,
    });
    const tokens = md.parse(source, {});
    return extractHeadingOutlineFromTokens(tokens);
}

export function extractHeadingOutlineFromTokens(tokens: readonly Token[]): HeadingEntry[] {
    const slugCounts = new Map<string, number>();
    const headings: HeadingEntry[] = [];

    for (let index = 0; index < tokens.length; index++) {
        const token = tokens[index];
        if (!token || token.type !== "heading_open") {
            continue;
        }

        const inline = tokens[index + 1];
        if (!inline || inline.type !== "inline") {
            continue;
        }

        const text = collectInlineText(inline).trim();
        if (text === "") {
            continue;
        }

        const level = Number.parseInt(token.tag.slice(1), 10);
        const id = allocateHeadingId(text, slugCounts);
        headings.push({ level, text, id });
    }

    return headings;
}

/**
 * Creates the renderer environment used to assign deterministic IDs to headings.
 * When documentStem is provided, the env also carries it for inline header action rendering.
 */
export function createHeadingRenderEnv(
    headings: readonly HeadingEntry[],
    documentStem?: string,
): HeadingRenderEnv {
    const env: HeadingRenderEnv = {
        vectorHeadingIds: headings.map((heading) => heading.id),
        vectorHeadingIndex: 0,
    };
    if (documentStem !== undefined) {
        env.vectorDocumentStem = documentStem;
    }
    return env;
}

/**
 * Computes the next heading ID for the current render pass.
 */
export function takeHeadingId(env: unknown, fallbackText: string): string {
    const headingEnv = env as HeadingRenderEnv;
    const ids = headingEnv.vectorHeadingIds ?? [];
    const index = headingEnv.vectorHeadingIndex ?? 0;
    const explicit = ids[index];
    headingEnv.vectorHeadingIndex = index + 1;
    return explicit ?? slugifyHeadingText(fallbackText);
}

export function slugifyHeadingText(text: string): string {
    const normalized = text
        .normalize("NFKD")
        .replace(/[\u0300-\u036f]/g, "")
        .toLowerCase();
    const collapsed = normalized.replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");
    return collapsed || "section";
}

function allocateHeadingId(text: string, slugCounts: Map<string, number>): string {
    const base = slugifyHeadingText(text);
    const count = slugCounts.get(base) ?? 0;
    slugCounts.set(base, count + 1);
    return count === 0 ? base : `${base}-${String(count + 1)}`;
}

function collectInlineText(token: Token): string {
    if (!token.children || token.children.length === 0) {
        return token.content;
    }

    return token.children.map((child) => collectInlineText(child)).join("");
}
