import * as fs from "fs";
import * as path from "path";
import type { GovernedDocument } from "../documentDiscovery.js";

const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---(\r?\n|$)/;

export interface StatusChangeResult {
    filePath: string;
    content: string;
    changed: boolean;
}

/**
 * Updates a status-based governed document and atomically renames the file into
 * the matching status folder when the target location changes on disk.
 */
export function changeGovernedDocumentStatus(
    workspaceRoot: string,
    doc: GovernedDocument,
    nextStatus: string,
    allowedStatuses: readonly string[],
): StatusChangeResult {
    if (!allowedStatuses.includes(nextStatus)) {
        throw new Error(
            `Vector: invalid status '${nextStatus}' for governed document type '${doc.type}'.`,
        );
    }

    const content = fs.readFileSync(doc.filePath, "utf-8");
    const currentStatus = readFrontmatterScalar(content, "status");
    if (currentStatus === null) {
        throw new Error(`Vector: missing frontmatter status in ${doc.filePath}.`);
    }
    if (currentStatus === nextStatus) {
        return { filePath: doc.filePath, content, changed: false };
    }

    const updatedContent = replaceFrontmatterScalar(content, "status", nextStatus);
    if (updatedContent === null) {
        throw new Error(`Vector: unable to update frontmatter status in ${doc.filePath}.`);
    }

    const targetDir = path.join(workspaceRoot, "doc", doc.type, nextStatus);
    const targetPath = path.join(targetDir, path.basename(doc.filePath));
    if (targetPath === doc.filePath) {
        fs.writeFileSync(doc.filePath, updatedContent, "utf-8");
        return { filePath: doc.filePath, content: updatedContent, changed: true };
    }

    fs.mkdirSync(targetDir, { recursive: true });
    if (fs.existsSync(targetPath)) {
        throw new Error(`Vector: target document already exists: ${targetPath}`);
    }

    fs.writeFileSync(doc.filePath, updatedContent, "utf-8");
    try {
        fs.renameSync(doc.filePath, targetPath);
    } catch (error) {
        fs.writeFileSync(doc.filePath, content, "utf-8");
        throw error;
    }

    return { filePath: targetPath, content: updatedContent, changed: true };
}

export function readFrontmatterScalar(content: string, key: string): string | null {
    const match = content.match(FRONTMATTER_RE);
    if (!match) {
        return null;
    }

    const frontmatter = match[1] ?? "";
    const lineMatch = frontmatter.match(new RegExp(`^${escapeRegExp(key)}:\\s*(.+)$`, "m"));
    if (!lineMatch) {
        return null;
    }

    return (lineMatch[1] ?? "").trim();
}

export function replaceFrontmatterScalar(
    content: string,
    key: string,
    value: string,
): string | null {
    const match = content.match(FRONTMATTER_RE);
    if (!match) {
        return null;
    }

    const fullMatch = match[0];
    const frontmatter = match[1] ?? "";
    if (!new RegExp(`^${escapeRegExp(key)}:\\s*.+$`, "m").test(frontmatter)) {
        return null;
    }

    const nextFrontmatter = frontmatter.replace(
        new RegExp(`^(${escapeRegExp(key)}:\\s*).+$`, "m"),
        `$1${value}`,
    );

    return content.replace(fullMatch, `---\n${nextFrontmatter}\n---${match[2] ?? "\n"}`);
}

function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
