/**
 * Parsed representation of a governed document identifier.
 *
 * Supports two forms:
 * - Unqualified: `{doc_type}-{code}-{slug}` (e.g., `rfc-00013-my-rfc`)
 * - Package-qualified: `{package}/{doc_type}-{code}-{slug}` (e.g., `my-pkg/rfc-00013-my-rfc`)
 */
export interface DocIdentifier {
    /** Synchronized package name, or null for workspace-local lookup. */
    package: string | null;
    /** Governed document type, e.g., "rfc", "task", "ai-rule". */
    docType: string;
    /** Zero-padded numeric code string, e.g., "00013". */
    code: string;
    /** Kebab-case slug, e.g., "my-rfc". */
    slug: string;
}

/**
 * Parses a governed document identifier string into its components.
 *
 * Accepts:
 * - `{doc_type}-{code}-{slug}` — workspace-local lookup
 * - `{package}/{doc_type}-{code}-{slug}` — package-qualified lookup
 *
 * The code component is identified as the first hyphen-separated segment consisting
 * entirely of ASCII digits. Everything before it forms the `docType`; everything
 * after forms the `slug`. This correctly handles multi-segment document type names
 * such as `ai-rule`.
 *
 * Returns null when the identifier cannot be parsed.
 */
export function parseDocIdentifier(identifier: string): DocIdentifier | null {
    if (!identifier) {
        return null;
    }

    let pkg: string | null = null;
    let stem = identifier;

    const slashPos = identifier.indexOf("/");
    if (slashPos !== -1) {
        const pkgPart = identifier.slice(0, slashPos);
        const stemPart = identifier.slice(slashPos + 1);
        if (!pkgPart || !stemPart) {
            return null;
        }
        pkg = pkgPart;
        stem = stemPart;
    }

    const parts = stem.split("-");
    if (parts.length < 3) {
        return null;
    }

    const codeIdx = parts.findIndex((p) => p.length > 0 && /^\d+$/.test(p));
    if (codeIdx <= 0 || codeIdx >= parts.length - 1) {
        return null;
    }

    const docType = parts.slice(0, codeIdx).join("-");
    const code = parts[codeIdx] ?? "";
    const slug = parts.slice(codeIdx + 1).join("-");

    if (!docType || !code || !slug) {
        return null;
    }

    return { package: pkg, docType, code, slug };
}
