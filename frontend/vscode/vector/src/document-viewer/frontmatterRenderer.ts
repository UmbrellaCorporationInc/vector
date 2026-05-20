import { escapeHtml } from "./previewHtml.js";
import { parseGovernedStem } from "./wikilinkNavigation.js";

/**
 * YAML frontmatter block pattern: opening ---, content, closing ---.
 * Matches only when the document begins with the opening fence.
 */
const FRONTMATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/;

/**
 * Parsed frontmatter: a map of string keys to raw string values.
 * All values are kept as strings; callers decide how to interpret them.
 */
export type FrontmatterFields = Record<string, unknown>;

export interface FrontmatterStatusEditor {
    current: string;
    options: readonly string[];
}

/**
 * Result of splitting a governed document into its frontmatter and body.
 */
export interface FrontmatterSplit {
    fields: FrontmatterFields;
    body: string;
}

/**
 * Splits raw markdown content into a parsed frontmatter field map and the
 * remaining markdown body. Returns an empty fields map when no frontmatter
 * is present or when YAML parsing fails.
 */
export function splitFrontmatter(content: string): FrontmatterSplit {
    const match = content.match(FRONTMATTER_RE);
    if (!match) {
        return { fields: {}, body: content };
    }

    const yamlBlock = match[1] ?? "";
    const body = content.slice(match[0].length);

    try {
        const fields = parseSimpleYaml(yamlBlock);
        return { fields, body };
    } catch {
        // Malformed frontmatter: treat as no frontmatter, keep full body.
        return { fields: {}, body: content };
    }
}

/**
 * Minimal YAML parser covering the governed document frontmatter subset:
 * - Scalar string/number/boolean values
 * - Block sequence values (- item per line)
 * - Quoted strings (single or double)
 *
 * Does not handle nested mappings or multi-document YAML.
 */
function parseSimpleYaml(yaml: string): FrontmatterFields {
    const result: FrontmatterFields = {};
    const lines = yaml.split(/\r?\n/);
    let i = 0;

    while (i < lines.length) {
        const line = lines[i];
        if (!line) {
            i++;
            continue;
        }

        // Skip blank lines and comments.
        if (line.trim() === "" || line.trimStart().startsWith("#")) {
            i++;
            continue;
        }

        const colonIdx = line.indexOf(":");
        if (colonIdx === -1) {
            i++;
            continue;
        }

        const key = line.slice(0, colonIdx).trim();
        const rest = line.slice(colonIdx + 1).trim();

        if (rest === "") {
            // Possible block sequence on following lines.
            const items: string[] = [];
            i++;
            while (i < lines.length) {
                const seqLine = lines[i];
                if (!seqLine) {
                    break;
                }
                const seqMatch = seqLine.match(/^\s*-\s+(.*)/);
                if (!seqMatch) {
                    break;
                }
                items.push(unquote((seqMatch[1] ?? "").trim()));
                i++;
            }
            result[key] = items.length > 0 ? items : "";
        } else {
            result[key] = parseScalar(rest);
            i++;
        }
    }

    return result;
}

function unquote(s: string): string {
    if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) {
        return s.slice(1, -1);
    }
    return s;
}

function parseScalar(s: string): string | number | boolean | null {
    const u = unquote(s);
    if (u !== s) {
        return u;
    }
    if (s === "true") {
        return true;
    }
    if (s === "false") {
        return false;
    }
    if (s === "null" || s === "~") {
        return null;
    }
    const n = Number(s);
    if (!isNaN(n) && s !== "") {
        return n;
    }
    return s;
}

/**
 * Fields whose values are never rendered as document links even when they
 * match the governed stem pattern. These are identity fields, not references.
 */
const NO_LINK_FIELDS = new Set(["id", "slug"]);

/**
 * Renders a parsed frontmatter field map to an HTML properties panel.
 * Returns an empty string when fields is empty.
 */
export function renderFrontmatterPanel(
    fields: FrontmatterFields,
    statusEditor?: FrontmatterStatusEditor,
): string {
    const entries = Object.entries(fields);
    if (entries.length === 0) {
        return "";
    }

    const rows = entries.map(([key, value]) => renderRow(key, value, statusEditor)).join("\n");

    return `<div class="vector-frontmatter">\n${rows}\n</div>`;
}

function renderRow(key: string, value: unknown, statusEditor?: FrontmatterStatusEditor): string {
    const keyHtml = `<span class="vector-fm-key">${escapeHtml(key)}</span>`;
    const linkify = !NO_LINK_FIELDS.has(key);
    const valueHtml =
        key === "status" && statusEditor
            ? renderStatusEditor(statusEditor)
            : renderValue(value, linkify);
    return (
        `  <div class="vector-fm-row">` +
        `<div class="vector-fm-key-cell">${keyHtml}</div>` +
        `<div class="vector-fm-value-cell">${valueHtml}</div>` +
        `</div>`
    );
}

function renderStatusEditor(statusEditor: FrontmatterStatusEditor): string {
    const options = statusEditor.options
        .map((option) => {
            const selected = option === statusEditor.current ? " selected" : "";
            return `<option value="${escapeHtml(option)}"${selected}>${escapeHtml(option)}</option>`;
        })
        .join("");
    return (
        `<label class="vector-fm-status-editor">` +
        `<select class="vector-fm-status-select" data-status-select>` +
        options +
        `</select>` +
        `</label>`
    );
}

function renderValue(value: unknown, linkify: boolean): string {
    if (value === null || value === undefined) {
        return `<span class="vector-fm-empty">Empty</span>`;
    }
    if (Array.isArray(value)) {
        if (value.length === 0) {
            return `<span class="vector-fm-empty">Empty</span>`;
        }
        const chips = value.map((item) => renderScalarItem(String(item), linkify, true)).join(" ");
        return `<span class="vector-fm-chips">${chips}</span>`;
    }
    if (typeof value === "boolean") {
        return `<span class="vector-fm-chip">${escapeHtml(String(value))}</span>`;
    }
    if (typeof value === "string") {
        return renderScalarItem(value, linkify);
    }
    // For other types (objects, etc.), serialize to string
    return renderScalarItem(JSON.stringify(value), linkify);
}

function renderScalarItem(str: string, linkify: boolean, inArray = false): string {
    if (linkify && parseGovernedStem(str) !== null) {
        return buildFmLinkAnchor(str);
    }
    if (isDateLike(str)) {
        return `<span class="vector-fm-date">${escapeHtml(str)}</span>`;
    }
    if (inArray) {
        return `<span class="vector-fm-chip">${escapeHtml(str)}</span>`;
    }
    return `<span class="vector-fm-scalar">${escapeHtml(str)}</span>`;
}

/**
 * Builds the HTML anchor for a frontmatter document link.
 * Uses data-fmlink so the click script can distinguish it from wikilinks.
 */
export function buildFmLinkAnchor(stem: string): string {
    return (
        `<a href="#" class="vector-fm-link" data-fmlink="${escapeHtml(stem)}">` +
        `${escapeHtml(stem)}</a>`
    );
}

/**
 * Returns true for strings matching ISO-8601 date format (YYYY-MM-DD).
 */
function isDateLike(s: string): boolean {
    return /^\d{4}-\d{2}-\d{2}$/.test(s);
}
