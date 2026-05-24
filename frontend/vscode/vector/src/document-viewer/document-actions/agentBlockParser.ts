import * as yaml from "js-yaml";

export interface AgentBlock {
    label: string;
    profile: string;
    prompt: string;
    promptField: string;
    input: Record<string, string>;
}

export interface AgentBlockParseError {
    error: string;
}

export type AgentBlockParseResult = AgentBlock | AgentBlockParseError;

export function isAgentBlockParseError(r: AgentBlockParseResult): r is AgentBlockParseError {
    return "error" in r;
}

/**
 * Parses the body of a vector-agent-button or vector-agent-action fence block.
 *
 * Expected YAML shape:
 *   label:   <string>   (required)
 *   profile: <string>   (required)
 *   prompt:  <string>   (required)
 *   input:              (optional key-value map)
 *     key: value
 */
export function parseAgentBlock(content: string): AgentBlockParseResult {
    let raw: unknown;
    try {
        raw = yaml.load(content);
    } catch {
        return { error: "vector-agent: YAML parse error" };
    }

    if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
        return { error: "vector-agent: block body must be a YAML mapping" };
    }

    const map = raw as Record<string, unknown>;

    if (typeof map.label !== "string" || !map.label.trim()) {
        return { error: "vector-agent: missing required field 'label'" };
    }

    if (typeof map.profile !== "string" || !map.profile.trim()) {
        return { error: "vector-agent: missing required field 'profile'" };
    }

    if (typeof map.prompt !== "string" || !map.prompt.trim()) {
        return { error: "vector-agent: missing required field 'prompt'" };
    }

    const input: Record<string, string> = {};
    if (map.input !== undefined && map.input !== null) {
        if (typeof map.input !== "object" || Array.isArray(map.input)) {
            return { error: "vector-agent: 'input' must be a key-value mapping" };
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

    let promptField = "prompt-message";
    if (map["prompt-field"] !== undefined) {
        if (typeof map["prompt-field"] !== "string" || !map["prompt-field"].trim()) {
            return { error: "vector-agent: 'prompt-field' must be a non-empty string" };
        }
        promptField = map["prompt-field"].trim();
    }

    return {
        label: map.label.trim(),
        profile: map.profile.trim(),
        prompt: map.prompt.trim(),
        promptField,
        input,
    };
}
