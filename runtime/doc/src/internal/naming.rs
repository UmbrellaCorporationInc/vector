//! Naming conventions for governed documents.

use std::path::Path;

/// Formats a governed document code with zero-padding.
#[must_use]
pub fn format_code(code: u32, width: u8) -> String {
    let width = usize::from(width);
    format!("{code:0>width$}")
}

/// Parses the numeric code from a governed filename.
///
/// Expected pattern: `{doc_type}-{code}-{slug}.md`
#[must_use]
pub fn parse_code_from_filename(filename: &str, doc_type: &str) -> Option<u32> {
    let stem = Path::new(filename).file_stem().and_then(|s| s.to_str())?;

    let expected_prefix = format!("{doc_type}-");
    if !stem.starts_with(&expected_prefix) {
        return None;
    }

    let after_prefix = &stem[expected_prefix.len()..];

    let dash_idx = after_prefix.find('-')?;
    let code_str = &after_prefix[..dash_idx];

    code_str.parse::<u32>().ok()
}

/// Returns true if the filename starts with the expected document type prefix.
#[must_use]
pub fn is_governed_file(filename: &str, doc_type: &str) -> bool {
    let stem = Path::new(filename).file_stem().and_then(|s| s.to_str()).unwrap_or("");

    let expected_pattern = format!("{doc_type}-");
    stem.starts_with(&expected_pattern)
}

#[cfg(test)]
#[path = "naming_test.rs"]
mod tests;
