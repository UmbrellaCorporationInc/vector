import * as fs from "fs";
import * as path from "path";

export type ResolveResult = { ok: true; filePath: string } | { ok: false; reason: string };

function parseGovernedId(governedId: string): { docType: string } | null {
    const match = governedId.match(/^([a-z][a-z0-9-]*?)-(\d+)-(.+)$/);
    if (!match) {
        return null;
    }
    return { docType: match[1] ?? "" };
}

function walkForFile(dir: string, targetFileName: string, results: string[]): void {
    let entries: fs.Dirent[];
    try {
        entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
        return;
    }
    for (const entry of entries) {
        if (entry.isDirectory()) {
            walkForFile(path.join(dir, entry.name), targetFileName, results);
        } else if (entry.name === targetFileName) {
            results.push(path.join(dir, entry.name));
        }
    }
}

/**
 * Resolve a governed document identifier to exactly one source file path.
 *
 * Parses the doc-type prefix from the identifier, then searches doc/<type>/
 * recursively for <governedId>.md. Fails when the identifier is malformed,
 * no file is found, or multiple files match.
 */
export function resolveCreateFormSource(workspaceRoot: string, governedId: string): ResolveResult {
    const parsed = parseGovernedId(governedId);
    if (!parsed) {
        return { ok: false, reason: `Malformed governed identifier: "${governedId}"` };
    }

    const docTypeDir = path.join(workspaceRoot, "doc", parsed.docType);
    const targetFileName = `${governedId}.md`;
    const matches: string[] = [];
    walkForFile(docTypeDir, targetFileName, matches);

    if (matches.length === 0) {
        return {
            ok: false,
            reason: `Governed document "${governedId}" not found under doc/${parsed.docType}/`,
        };
    }

    if (matches.length > 1) {
        return {
            ok: false,
            reason: `Governed document "${governedId}" is ambiguous: ${String(matches.length)} matching files found`,
        };
    }

    const filePath = matches[0];
    if (filePath === undefined) {
        return { ok: false, reason: `Governed document "${governedId}" not found` };
    }

    return { ok: true, filePath };
}
