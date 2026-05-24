import { escapeHtml } from "../previewHtml.js";

const INLINE_ACTION_PROFILE = "create-doc";
const INLINE_ACTION_PROMPT = "prompts-00006-update-document";
const INLINE_ACTION_LABEL = "Update section";
const INLINE_ACTION_GLYPH = "✏"; // ✏ PENCIL

/**
 * Renders the pencil-style inline action button injected after each markdown heading.
 *
 * The element reuses the same data-agent-* contract consumed by preview.js on click,
 * so no new click-handling code is required. The document-stem input variable
 * identifies which governed document the heading belongs to, and document-header
 * carries the heading text so prompts can reference the specific section.
 */
export function renderInlineHeaderAction(documentStem: string, headingText?: string): string {
    const inputObj: Record<string, string> = { "document-stem": documentStem };
    if (headingText) {
        inputObj["document-header"] = headingText;
    }
    const input = escapeHtml(JSON.stringify(inputObj));
    return (
        `<button class="vector-agent-inline-action" type="button"` +
        ` data-agent-profile="${escapeHtml(INLINE_ACTION_PROFILE)}"` +
        ` data-agent-prompt="${escapeHtml(INLINE_ACTION_PROMPT)}"` +
        ` data-agent-label="${escapeHtml(INLINE_ACTION_LABEL)}"` +
        ` data-agent-input="${input}"` +
        ` data-agent-prompt-field="prompt-message"` +
        ` aria-label="${escapeHtml(INLINE_ACTION_LABEL)}"` +
        `>${INLINE_ACTION_GLYPH}</button>`
    );
}
