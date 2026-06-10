//! RULE-26: File code line limit.
//!
//! A file must not exceed 300 code lines. A code line is any line that is
//! neither blank nor a line comment (`//`, `///`, `//!`).
//!
//! **Exemptions:**
//! - Files ending in `_test.rs` — test files grow with case count.
//! - Files named `main.rs` — dispatcher pattern; trivially short by design.
//! - Files named `lib.rs` or `mod.rs` — aggregator files contain only
//!   re-exports and are not subject to implementation complexity limits.
//!
//! **Activation:** `--future` until the workspace clears all violations.

use std::path::Path;

use crate::lint_rules::rule::{Rule, RuleViolation};

pub struct FileTooLong {
    pub(crate) limit: usize,
    pub(crate) is_future: bool,
}

impl Rule for FileTooLong {
    fn is_active(&self, future: bool) -> bool {
        self.is_future && future
    }

    fn check_rust(&self, path: &Path, _ast: &syn::File, raw: &str, out: &mut Vec<RuleViolation>) {
        if is_exempt(path) {
            return;
        }

        let code_lines = count_code_lines(raw);
        if code_lines > self.limit {
            out.push(RuleViolation {
                file: path.to_path_buf(),
                line: None,
                column: None,
                rule_id: "RULE-26",
                message: format!(
                    "`{}` has {code_lines} code lines excluding comments (limit: {}) — split at a domain boundary",
                    path.display(),
                    self.limit,
                ),
            });
        }
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Returns `true` for files that are exempt from the line-limit rule.
fn is_exempt(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.ends_with("_test.rs") || name == "main.rs" || name == "lib.rs" || name == "mod.rs"
}

/// Counts non-blank, non-comment lines in a Rust source string.
///
/// A line is excluded when its first non-whitespace content is `//`
/// (covers `//`, `///`, and `//!`) or when it contains only whitespace.
#[must_use]
pub fn count_code_lines(src: &str) -> usize {
    src.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

#[cfg(test)]
#[path = "file_too_long_test.rs"]
mod tests;
