import { escapeHtml } from "../previewHtml.js";
import { parseFormBlock } from "./formParser.js";
import type { FormField } from "./formParser.js";

/**
 * Renders the body of a vector-form fence block into an HTML form section.
 * Returns an empty string when the block produces no recognisable fields.
 */
export function renderFormBlock(content: string): string {
    const fields = parseFormBlock(content);
    if (fields.length === 0) {
        return "";
    }
    const rows = fields.map(renderField).join("\n");
    return `<div class="vector-form">\n${rows}\n</div>\n`;
}

function renderField(field: FormField): string {
    const keyAttr = `data-form-key="${escapeHtml(field.key)}" data-form-type="${escapeHtml(field.type)}"`;
    const fieldId = buildFieldId(field.key);
    const labelId = `${fieldId}-label`;
    if (field.readOnly) {
        return (
            `<span class="vector-form-label vector-form-label--readonly" id="${labelId}">${escapeHtml(field.key)}</span>` +
            `<span class="vector-form-readonly-value" ${keyAttr} aria-labelledby="${labelId}">${escapeHtml(field.value ?? "")}</span>`
        );
    }
    const label = escapeHtml(field.label ?? field.key);
    const nameAttr = `name="${escapeHtml(field.key)}"`;
    if (field.type === "chat-input") {
        return (
            `<label class="vector-form-label" id="${labelId}" for="${fieldId}">${label}</label>` +
            `<div class="vector-chat-input-host" id="${fieldId}" ${keyAttr} ` +
            `data-chat-input-name="${escapeHtml(field.key)}" data-chat-input-label-id="${labelId}">` +
            `<div class="vector-chat-input-mount" ` +
            `role="textbox" aria-multiline="true" ` +
            `data-chat-input-editable="true"></div>` +
            `</div>`
        );
    }
    return (
        `<label class="vector-form-label" for="${fieldId}">${label}</label>` +
        `<input class="vector-form-input" id="${fieldId}" type="text" ${nameAttr} ${keyAttr} />`
    );
}

function buildFieldId(key: string): string {
    return `vector-form-field-${key}`;
}
