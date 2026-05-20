import { escapeHtml } from "../previewHtml.js";
import { parseAgentBlock, isAgentBlockParseError } from "./agentBlockParser.js";

export type AgentBlockVariant = "button" | "action";

/**
 * Renders a vector-agent-button or vector-agent-action fence block as an HTML button.
 *
 * The element carries data attributes consumed by preview.js on click:
 *   data-agent-profile  — profile name to resolve from agents.yaml
 *   data-agent-prompt   — prompt document identifier to resolve
 *   data-agent-label    — label used for the terminal title
 *   data-agent-input    — JSON-encoded static input variables map
 */
export function renderAgentBlock(content: string, variant: AgentBlockVariant): string {
    const result = parseAgentBlock(content);

    if (isAgentBlockParseError(result)) {
        return `<span class="vector-agent-error">${escapeHtml(result.error)}</span>\n`;
    }

    const cssClass = variant === "button" ? "vector-agent-button" : "vector-agent-action";
    const inputJson = escapeHtml(JSON.stringify(result.input));

    return (
        `<button class="${cssClass}" type="button"` +
        ` data-agent-profile="${escapeHtml(result.profile)}"` +
        ` data-agent-prompt="${escapeHtml(result.prompt)}"` +
        ` data-agent-label="${escapeHtml(result.label)}"` +
        ` data-agent-input="${inputJson}">` +
        escapeHtml(result.label) +
        `</button>\n`
    );
}
