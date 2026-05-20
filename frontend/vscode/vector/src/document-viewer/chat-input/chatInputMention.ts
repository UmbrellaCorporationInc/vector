import type { ChatInputMention } from "./chatInputTypes.js";
import type { FileSuggestion } from "./chatInputMessaging.js";

export interface MentionQuery {
    query: string;
    start: number;
    end: number;
}

export interface MentionRange {
    mention: ChatInputMention;
    from: number;
    to: number;
}

/**
 * Detects an active @mention query at the given cursor position.
 * Returns query details when the cursor immediately follows an @<word> token
 * that is either at the start of text or preceded by whitespace.
 */
export function detectMentionQuery(text: string, cursorPos: number): MentionQuery | null {
    if (cursorPos <= 0) {
        return null;
    }
    const before = text.slice(0, cursorPos);
    const match = /(?:^|[\s\n])@([^\s@]*)$/.exec(before);
    if (!match) {
        return null;
    }
    const query = match[1] ?? "";
    const atIndex = before.lastIndexOf("@");
    return { query, start: atIndex, end: cursorPos };
}

/**
 * Builds the plain-text token inserted into the editor for a file mention.
 */
export function createMentionToken(suggestion: FileSuggestion): string {
    return `@${suggestion.path}`;
}

/**
 * Replaces the @<query> token with @<path> and returns the updated text
 * and the new cursor position after the insertion.
 */
export function insertMentionText(
    text: string,
    mentionQuery: MentionQuery,
    suggestion: FileSuggestion,
): { text: string; cursorPos: number } {
    const token = createMentionToken(suggestion);
    const newText = text.slice(0, mentionQuery.start) + token + text.slice(mentionQuery.end);
    return { text: newText, cursorPos: mentionQuery.start + token.length };
}

/**
 * Builds a structured ChatInputMention from a FileSuggestion.
 */
export function buildMention(suggestion: FileSuggestion): ChatInputMention {
    return { type: "file", label: suggestion.label, path: suggestion.path };
}

/**
 * Finds every mention whose @<path> token still appears in the current text.
 */
export function findMentionRanges(text: string, mentions: ChatInputMention[]): MentionRange[] {
    const ranges: MentionRange[] = [];
    for (const mention of mentions) {
        const token = `@${mention.path}`;
        let searchFrom = 0;
        while (searchFrom < text.length) {
            const index = text.indexOf(token, searchFrom);
            if (index === -1) {
                break;
            }
            ranges.push({ mention, from: index, to: index + token.length });
            searchFrom = index + token.length;
        }
    }
    ranges.sort((left, right) => left.from - right.from || left.to - right.to);
    return ranges;
}

/**
 * Filters the mentions list to only include mentions whose paths still
 * appear in the text as @<path> tokens.
 */
export function reconcileMentions(text: string, mentions: ChatInputMention[]): ChatInputMention[] {
    const ranges = findMentionRanges(text, mentions);
    const seen = new Set<string>();
    const reconciled: ChatInputMention[] = [];
    for (const { mention } of ranges) {
        const key = `${mention.type}:${mention.path}`;
        if (seen.has(key)) {
            continue;
        }
        seen.add(key);
        reconciled.push(mention);
    }
    return reconciled;
}

/**
 * Finds a mention range touching the cursor for whole-token deletion behavior.
 */
export function findMentionRangeAtCursor(
    text: string,
    mentions: ChatInputMention[],
    cursorPos: number,
    direction: "backward" | "forward",
): MentionRange | null {
    const ranges = findMentionRanges(text, mentions);
    for (const range of ranges) {
        if (direction === "backward" && cursorPos === range.to) {
            return range;
        }
        if (direction === "forward" && cursorPos === range.from) {
            return range;
        }
    }
    return null;
}
