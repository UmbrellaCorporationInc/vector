export const FILE_SUGGESTIONS_REQUEST_TYPE = "vector.chatInput.requestSuggestions" as const;
export const FILE_SUGGESTIONS_RESULT_TYPE = "vector.chatInput.suggestionsResult" as const;
export const RENDER_FORM_BLOCK_REQUEST_TYPE = "vector.renderFormBlock" as const;
export const RENDER_FORM_BLOCK_RESULT_TYPE = "vector.renderFormBlockResult" as const;

export interface FileSuggestion {
    label: string;
    path: string;
}

export interface FileSuggestionsRequest {
    type: typeof FILE_SUGGESTIONS_REQUEST_TYPE;
    requestId: string;
    query: string;
}

export interface FileSuggestionsResult {
    type: typeof FILE_SUGGESTIONS_RESULT_TYPE;
    requestId: string;
    suggestions: FileSuggestion[];
}

export function isFileSuggestionsRequest(msg: unknown): msg is FileSuggestionsRequest {
    return (
        isRecord(msg) &&
        msg["type"] === FILE_SUGGESTIONS_REQUEST_TYPE &&
        typeof msg["requestId"] === "string" &&
        typeof msg["query"] === "string"
    );
}

export function isFileSuggestionsResult(msg: unknown): msg is FileSuggestionsResult {
    return (
        isRecord(msg) &&
        msg["type"] === FILE_SUGGESTIONS_RESULT_TYPE &&
        typeof msg["requestId"] === "string" &&
        Array.isArray(msg["suggestions"])
    );
}

export interface RenderFormBlockRequest {
    type: typeof RENDER_FORM_BLOCK_REQUEST_TYPE;
    requestId: string;
    content: string;
}

export interface RenderFormBlockResult {
    type: typeof RENDER_FORM_BLOCK_RESULT_TYPE;
    requestId: string;
    html: string;
}

export function isRenderFormBlockRequest(msg: unknown): msg is RenderFormBlockRequest {
    return (
        isRecord(msg) &&
        msg["type"] === RENDER_FORM_BLOCK_REQUEST_TYPE &&
        typeof msg["requestId"] === "string" &&
        typeof msg["content"] === "string"
    );
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
}
