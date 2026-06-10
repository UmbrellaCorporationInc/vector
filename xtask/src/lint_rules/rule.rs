//! Core types for the lint_rules subsystem: `RuleViolation` and the `Rule` trait.

use std::path::PathBuf;

/// A single violation reported by a lint rule.
#[derive(Debug)]
pub(crate) struct RuleViolation {
    /// Absolute path to the file where the violation was detected.
    pub(crate) file: PathBuf,
    /// 1-based line number of the offending node, if available.
    pub(crate) line: Option<u32>,
    /// 1-based column number of the offending node, if available.
    pub(crate) column: Option<u32>,
    /// Rule identifier (e.g. `"RULE-13A"`).
    pub(crate) rule_id: &'static str,
    /// Human-readable description of the violation.
    pub(crate) message: String,
}

/// A project-local lint rule operating on either parsed Rust source or TOML manifests.
pub(crate) trait Rule {
    /// Returns `true` when this rule is active in the current run mode.
    ///
    /// Standard rules always return `true`. Rules gated behind `--future` return `true`
    /// only when `future` is `true`.
    #[must_use]
    fn is_active(&self, future: bool) -> bool;

    /// Inspect a parsed Rust source file and append any violations to `out`.
    ///
    /// `raw` is the original source text of the file, available for rules that
    /// need line-level analysis without re-parsing (e.g. RULE-26).
    ///
    /// The default implementation does nothing. Most rules will override this.
    fn check_rust(
        &self,
        _path: &std::path::Path,
        _ast: &syn::File,
        _raw: &str,
        _out: &mut Vec<RuleViolation>,
    ) {
    }

    /// Inspect a TOML manifest file and append any violations to `out`.
    ///
    /// The default implementation does nothing. Only manifest rules will override this.
    fn check_toml(&self, _path: &std::path::Path, _content: &str, _out: &mut Vec<RuleViolation>) {}
}

#[cfg(test)]
#[path = "rule_test.rs"]
mod tests;
