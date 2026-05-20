export type LineClass = "heading" | "list-item" | "fenced-code" | null;

export interface InlineToken {
    text: string;
    type: "strong" | "em" | "code" | "plain";
}

/**
 * Classifies a markdown line for block-level visual styling.
 * Returns null for plain paragraph lines.
 */
export function classifyLine(line: string): LineClass {
    if (/^#{1,6}\s/.test(line)) return "heading";
    if (/^(?:[-*+]|\d+\.)\s/.test(line)) return "list-item";
    if (/^```/.test(line)) return "fenced-code";
    return null;
}

/**
 * Tokenizes a text string into inline markdown tokens.
 * Preserves raw syntax characters — token text equals the source text.
 */
export function tokenizeInline(text: string): InlineToken[] {
    const re = /(\*\*[^*\n]+\*\*|\*(?!\s)[^*\n]+(?<!\s)\*|`[^`\n]+`)/g;
    const tokens: InlineToken[] = [];
    let lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = re.exec(text)) !== null) {
        if (match.index > lastIndex) {
            tokens.push({ text: text.slice(lastIndex, match.index), type: "plain" });
        }
        const tok = match[0];
        const type: InlineToken["type"] = tok.startsWith("**")
            ? "strong"
            : tok.startsWith("*")
              ? "em"
              : "code";
        tokens.push({ text: tok, type });
        lastIndex = re.lastIndex;
    }
    if (lastIndex < text.length) {
        tokens.push({ text: text.slice(lastIndex), type: "plain" });
    }
    return tokens;
}

/**
 * Returns the concatenated raw text from an array of inline tokens.
 * The result always equals the original source string passed to tokenizeInline.
 */
export function joinTokenText(tokens: InlineToken[]): string {
    return tokens.map((t) => t.text).join("");
}
