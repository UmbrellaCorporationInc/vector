import { escapeHtml } from "../previewHtml.js";
import { parseOpenDocBlock, isOpenDocParseError } from "./openDocParser.js";

/**
 * Renders the body of a vector-open-doc fence block as a navigable anchor.
 *
 * The anchor carries:
 *   data-open-doc       — the document identifier to resolve
 *   data-open-doc-input — JSON-encoded input variables map
 *
 * When YAML is malformed or required fields are absent, an inline error
 * message is rendered instead.
 */
export function renderOpenDocBlock(content: string): string {
    const result = parseOpenDocBlock(content);

    if (isOpenDocParseError(result)) {
        return `<span class="vector-open-doc-error">${escapeHtml(result.error)}</span>\n`;
    }

    const inputJson = escapeHtml(JSON.stringify(result.input));
    return (
        `<a class="vector-open-doc" href="#"` +
        ` data-open-doc="${escapeHtml(result.doc)}"` +
        ` data-open-doc-input="${inputJson}">` +
        escapeHtml(result.label) +
        `</a>\n`
    );
}
