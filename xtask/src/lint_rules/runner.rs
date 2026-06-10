//! Lint rule execution runner.

use super::rule::RuleViolation;
use super::{rules, walker};
use std::path::Path;

/// Run all active lint rules across the workspace.
///
/// Returns a flat list of violations. An empty `Vec` means the workspace is
/// clean. When `future` is `false`, only standard rules are applied. When
/// `true`, future rules are also evaluated.
#[must_use]
pub fn run(workspace: &Path, future: bool) -> Vec<RuleViolation> {
    let active_rules = rules::all(future);
    let entries = walker::walk(workspace);

    let mut violations = Vec::new();
    for entry in &entries {
        for rule in &active_rules {
            if rule.is_active(future) {
                match entry {
                    walker::LintEntry::Rust(path, ast, raw) => {
                        rule.check_rust(path, ast, raw, &mut violations);
                    }
                    walker::LintEntry::Toml(path, content) => {
                        rule.check_toml(path, content, &mut violations);
                    }
                }
            }
        }
    }
    violations
}

#[cfg(test)]
#[path = "runner_test.rs"]
mod tests;
