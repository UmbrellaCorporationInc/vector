import * as yaml from "js-yaml";

export interface OpenDocBlock {
    label: string;
    doc: string;
    input: Record<string, string>;
}

export interface OpenDocParseError {
    error: string;
}

export type OpenDocParseResult = OpenDocBlock | OpenDocParseError;

export function isOpenDocParseError(result: OpenDocParseResult): result is OpenDocParseError {
    return "error" in result;
}

/**
 * Parses the body of a vector-open-doc fence block (YAML format).
 *
 * Expected shape:
 *   label: <string>   (required)
 *   doc:   <string>   (required)
 *   input:            (optional key-value map)
 *     key: value
 */
export function parseOpenDocBlock(content: string): OpenDocParseResult {
    let raw: unknown;
    try {
        raw = yaml.load(content);
    } catch {
        return { error: "vector-open-doc: YAML parse error" };
    }

    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
        return { error: "vector-open-doc: block body must be a YAML mapping" };
    }

    const map = raw as Record<string, unknown>;

    if (typeof map.label !== "string" || !map.label.trim()) {
        return { error: "vector-open-doc: missing required field 'label'" };
    }

    if (typeof map.doc !== "string" || !map.doc.trim()) {
        return { error: "vector-open-doc: missing required field 'doc'" };
    }

    const input: Record<string, string> = {};
    if (map.input !== undefined && map.input !== null) {
        if (typeof map.input !== "object" || Array.isArray(map.input)) {
            return { error: "vector-open-doc: 'input' must be a key-value mapping" };
        }
        for (const [k, v] of Object.entries(map.input as Record<string, unknown>)) {
            if (typeof v === "string") {
                input[k] = v;
            } else if (typeof v === "number" || typeof v === "boolean") {
                input[k] = String(v);
            } else {
                input[k] = "";
            }
        }
    }

    return { label: map.label, doc: map.doc.trim(), input };
}
