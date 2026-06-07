//! Centralized governed document identifier parsing.

/// Parsed representation of a governed document identifier.
///
/// Supports two forms:
/// - Unqualified: `{doc_type}-{code}-{slug}` (e.g., `rfc-00013-my-rfc`)
/// - Package-qualified: `{package}/{doc_type}-{code}-{slug}` (e.g., `my-pkg/rfc-00013-my-rfc`)
///
/// # DTO(identifier carries parsed components for downstream lookup and routing)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocIdentifier {
    /// Synchronized package name, or `None` for workspace-local lookup.
    pub package: Option<String>,
    /// Governed document type (e.g., `"rfc"`, `"task"`, `"ai-rule"`).
    pub doc_type: String,
    /// Numeric code of the document.
    pub code: u32,
    /// Kebab-case slug of the document.
    pub slug: String,
}

/// Parses a governed document identifier string into its components.
///
/// Accepts:
/// - `{doc_type}-{code}-{slug}` — workspace-local lookup
/// - `{package}/{doc_type}-{code}-{slug}` — package-qualified lookup
///
/// The code component is identified as the first hyphen-separated segment consisting
/// entirely of ASCII digits.  Everything before it forms the `doc_type`; everything
/// after forms the `slug`.  This strategy correctly handles multi-segment document
/// type names such as `ai-rule`.
///
/// Returns `None` when the identifier cannot be parsed.
#[must_use]
pub fn parse_doc_identifier(identifier: &str) -> Option<DocIdentifier> {
    if identifier.is_empty() {
        return None;
    }

    let (package, stem) = if let Some(slash_pos) = identifier.find('/') {
        let pkg = &identifier[..slash_pos];
        let rest = &identifier[slash_pos + 1..];
        if pkg.is_empty() || rest.is_empty() {
            return None;
        }
        (Some(pkg.to_string()), rest)
    } else {
        (None, identifier)
    };

    let parts: Vec<&str> = stem.split('-').collect();
    if parts.len() < 3 {
        return None;
    }

    let code_idx =
        parts.iter().position(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))?;

    if code_idx == 0 || code_idx >= parts.len() - 1 {
        return None;
    }

    let doc_type = parts[..code_idx].join("-");
    let code = parts[code_idx].parse::<u32>().ok()?;
    let slug = parts[code_idx + 1..].join("-");

    if doc_type.is_empty() || slug.is_empty() {
        return None;
    }

    Some(DocIdentifier { package, doc_type, code, slug })
}

#[cfg(test)]
#[path = "identifier_test.rs"]
mod tests;
