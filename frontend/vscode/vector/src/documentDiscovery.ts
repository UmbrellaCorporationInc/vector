import * as fs from "fs";
import * as path from "path";
import * as yaml from "js-yaml";

/**
 * Raw shape loaded from `.vector/document-types.yaml`.
 */
export interface DocumentTypeConfig {
    layout: "status" | "category" | "directory";
    "code-width": number;
    "initial-status"?: string;
    statuses?: string[];
    template?: string;
    prompt?: string;
    description?: string;
    tags?: string[];
    "create-document-form"?: string;
}

/**
 * Global `doc-type` section of `.vector/document-types.yaml`.
 */
export interface DocTypeGlobalConfig {
    template?: string;
    "prompt-template"?: string;
    prompt?: string;
    "create-document-type-form"?: string;
    "prompt-validate-fix"?: string;
}

export interface DocumentTypesYaml {
    "doc-type"?: DocTypeGlobalConfig;
    "document-types": Record<string, DocumentTypeConfig>;
}

export interface GovernedDocumentGroup {
    kind: "status" | "category";
    value: string;
}

/**
 * A discovered governed document on disk.
 */
export interface GovernedDocument {
    type: string;
    code: string;
    slug: string;
    title: string;
    status?: string;
    category?: string;
    filePath: string;
}

/**
 * Resolve the first workspace root that contains `.vector/document-types.yaml`.
 */
export function findGovernedWorkspaceRoot(workspaceRoots: readonly string[]): string | null {
    for (const workspaceRoot of workspaceRoots) {
        const configPath = path.join(workspaceRoot, ".vector", "document-types.yaml");
        if (fs.existsSync(configPath)) {
            return workspaceRoot;
        }
    }

    return null;
}

/**
 * Reads `.vector/document-types.yaml` from the workspace root and returns the
 * parsed document-type map.  Returns `null` when the file is missing or
 * unreadable so callers can fail safely.
 */
export function loadDocumentTypes(workspaceRoot: string): DocumentTypesYaml | null {
    const configPath = path.join(workspaceRoot, ".vector", "document-types.yaml");
    if (!fs.existsSync(configPath)) {
        return null;
    }
    try {
        const raw = fs.readFileSync(configPath, "utf-8");

        return yaml.load(raw) as DocumentTypesYaml;
    } catch {
        return null;
    }
}

/**
 * Scans `doc/<type>/` for governed markdown files and extracts metadata from
 * frontmatter.  Results are sorted by numeric code ascending, then slug.
 */
export function scanGovernedDocuments(
    workspaceRoot: string,
    docType: string,
    config: DocumentTypeConfig,
): GovernedDocument[] {
    const docDir = path.join(workspaceRoot, "doc", docType);
    if (!fs.existsSync(docDir)) {
        return [];
    }

    const results: GovernedDocument[] = [];

    if (config.layout === "status" && config.statuses) {
        for (const status of config.statuses) {
            results.push(
                ...scanGovernedDocumentsInGroup(workspaceRoot, docType, config, {
                    kind: "status",
                    value: status,
                }),
            );
        }
    } else if (config.layout === "category") {
        for (const categoryEntry of fs.readdirSync(docDir, { withFileTypes: true })) {
            if (!categoryEntry.isDirectory()) {
                continue;
            }
            results.push(
                ...scanGovernedDocumentsInGroup(workspaceRoot, docType, config, {
                    kind: "category",
                    value: categoryEntry.name,
                }),
            );
        }
    } else if (config.layout === "directory") {
        for (const entry of fs.readdirSync(docDir)) {
            if (!entry.endsWith(".md")) {
                continue;
            }
            const filePath = path.join(docDir, entry);
            const parsed = parseGovernedFileName(entry);
            if (!parsed) {
                continue;
            }
            const frontmatter = readFrontmatter(filePath);
            results.push({
                type: docType,
                code: parsed.code,
                slug: parsed.slug,
                title: frontmatter.title ?? parsed.slug,
                filePath,
            });
        }
    }

    return sortGovernedDocuments(results);
}

/**
 * Scans only a single status or category group for governed markdown files.
 * Returns an empty list when the group is not compatible with the document type
 * contract or when the backing folder does not exist.
 */
export function scanGovernedDocumentsInGroup(
    workspaceRoot: string,
    docType: string,
    config: DocumentTypeConfig,
    group: GovernedDocumentGroup,
): GovernedDocument[] {
    const groupDir = resolveGroupDirectory(workspaceRoot, docType, config, group);
    if (!groupDir) {
        return [];
    }

    const results: GovernedDocument[] = [];
    for (const entry of fs.readdirSync(groupDir)) {
        if (!entry.endsWith(".md")) {
            continue;
        }
        const filePath = path.join(groupDir, entry);
        const parsed = parseGovernedFileName(entry);
        if (!parsed) {
            continue;
        }
        const frontmatter = readFrontmatter(filePath);
        results.push({
            type: docType,
            code: parsed.code,
            slug: parsed.slug,
            title: frontmatter.title ?? parsed.slug,
            ...(group.kind === "status" ? { status: group.value } : { category: group.value }),
            filePath,
        });
    }

    return sortGovernedDocuments(results);
}

/**
 * Parse a governed file name like `rfc-00014-sample.md` into type, code, slug.
 */
function parseGovernedFileName(
    fileName: string,
): { type: string; code: string; slug: string } | null {
    const base = path.basename(fileName, ".md");
    const match = base.match(/^([a-z][a-z0-9-]*?)-(\d+)-(.+)$/);
    if (!match) {
        return null;
    }
    return { type: match[1] ?? "", code: match[2] ?? "", slug: match[3] ?? "" };
}

/**
 * Extract `title` from YAML frontmatter if present.
 */
function readFrontmatter(filePath: string): { title?: string } {
    try {
        const content = fs.readFileSync(filePath, "utf-8");
        const delim = "---\n";
        if (!content.startsWith(delim)) {
            return {};
        }
        const end = content.indexOf(delim, delim.length);
        if (end === -1) {
            return {};
        }
        const front = content.slice(delim.length, end);

        const parsed = yaml.load(front) as Record<string, unknown> | undefined;
        if (parsed && typeof parsed.title === "string") {
            return { title: parsed.title };
        }
    } catch {
        // ignore
    }
    return {};
}

function resolveGroupDirectory(
    workspaceRoot: string,
    docType: string,
    config: DocumentTypeConfig,
    group: GovernedDocumentGroup,
): string | null {
    const docDir = path.join(workspaceRoot, "doc", docType);
    if (!fs.existsSync(docDir)) {
        return null;
    }

    if (config.layout === "status") {
        if (group.kind !== "status") {
            return null;
        }
        if (!(config.statuses ?? []).includes(group.value)) {
            return null;
        }
        const statusDir = path.join(docDir, group.value);
        return fs.existsSync(statusDir) ? statusDir : null;
    }

    if (group.kind !== "category") {
        return null;
    }

    const categoryDir = path.join(docDir, group.value);
    return fs.existsSync(categoryDir) ? categoryDir : null;
}

function sortGovernedDocuments(results: GovernedDocument[]): GovernedDocument[] {
    results.sort((a, b) => {
        const codeDiff = Number.parseInt(a.code, 10) - Number.parseInt(b.code, 10);
        if (codeDiff !== 0) {
            return codeDiff;
        }
        return a.slug.localeCompare(b.slug);
    });
    return results;
}
