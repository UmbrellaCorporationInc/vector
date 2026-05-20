import * as path from "path";
import * as vscode from "vscode";
import type { FileSuggestion } from "./chatInputMessaging.js";

const MAX_SUGGESTIONS = 10;

/**
 * Searches the workspace for files matching the query and returns bounded
 * suggestions. A blank query returns the first bounded workspace matches so
 * typing "@" can still open a useful suggestion surface. Search failures
 * still degrade to an empty array.
 */
export async function resolveFileSuggestions(
    workspaceRoot: string,
    query: string,
): Promise<FileSuggestion[]> {
    const trimmed = query.trim();
    const pattern = trimmed ? `**/*${trimmed.replace(/[*?[\]{}]/g, "\\$&")}*` : "**/*";
    try {
        const uris = await vscode.workspace.findFiles(pattern, undefined, MAX_SUGGESTIONS);
        return uris.map((uri) => {
            const rel = path.relative(workspaceRoot, uri.fsPath).replace(/\\/g, "/");
            const label = rel.split("/").pop() ?? rel;
            return { label, path: rel };
        });
    } catch {
        return [];
    }
}
