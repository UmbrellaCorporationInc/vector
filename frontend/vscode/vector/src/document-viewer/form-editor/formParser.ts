export type FormFieldType = "input" | "chat-input";

export interface FormField {
    key: string;
    type: FormFieldType;
    label: string | null;
    value: string | null;
    readOnly: boolean;
}

const FIELD_RE =
    /^([a-zA-Z][a-zA-Z0-9_-]*)\s*=\s*(input|chat-input)\(("(?:[^"\\]|\\.)*"|[^)]*)\)\s*$/;

/**
 * Parses the body of a vector-form fence block into a list of form fields.
 *
 * Grammar per line:
 *   key = input("Label")      → editable single-line field; label is quoted text
 *   key = chat-input("Label") → editable multi-line field; label is quoted text
 *   key = input(value)        → read-only field; value is unquoted (pre-substituted #{})
 *   key = chat-input(value)   → read-only multi-line field; value is unquoted
 *
 * Lines that do not match the grammar are silently ignored.
 */
export function parseFormBlock(content: string): FormField[] {
    const fields: FormField[] = [];
    for (const raw of content.split("\n")) {
        const line = raw.trim();
        if (!line) {
            continue;
        }
        const match = FIELD_RE.exec(line);
        if (!match) {
            continue;
        }
        const key = match[1] ?? "";
        const type = (match[2] ?? "input") as FormFieldType;
        const inner = match[3] ?? "";
        const isQuoted = inner.startsWith('"') && inner.endsWith('"') && inner.length >= 2;
        if (isQuoted) {
            const label = inner.slice(1, -1).replace(/\\"/g, '"');
            fields.push({ key, type, label, value: null, readOnly: false });
        } else {
            fields.push({ key, type, label: null, value: inner, readOnly: true });
        }
    }
    return fields;
}
