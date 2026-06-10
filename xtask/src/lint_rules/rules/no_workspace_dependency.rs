//! RULE-15: Workspace Dependency Enforcement.
//!
//! Ensures all dependencies in workspace-member crates are centrally managed
//! in the root `Cargo.toml`.
//!
//! # Violation
//! Any dependency entry in a child `Cargo.toml` that is NOT `{ workspace = true }`.
//!
//! # Exemption
//! The root `Cargo.toml` of the workspace is the source of truth and is exempt.

use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;

pub(crate) struct NoWorkspaceDependency;

impl Rule for NoWorkspaceDependency {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_toml(&self, path: &Path, content: &str, out: &mut Vec<RuleViolation>) {
        // Condition 1: Must be a `Cargo.toml` file (walker already ensures this, but let's be safe).
        if path.file_name().and_then(|n| n.to_str()) != Some("Cargo.toml") {
            return;
        }

        // Condition 2: Exempt the root `Cargo.toml`.
        // We assume the caller passes absolute paths and we can check if it's in the workspace root.
        // For the POC, we look for `[workspace]` in the content to identify the root.
        if content.contains("[workspace]") {
            return;
        }

        let mut in_dependencies = false;
        for (idx, line) in content.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Enter/exit dependency sections
            if line.starts_with('[') {
                let section = line.to_lowercase();
                in_dependencies = section.contains("dependencies");
                continue;
            }

            if in_dependencies {
                // We expect `name = { workspace = true }`.
                // If it contains `=` but not `workspace = true`, it's a violation.
                if line.contains('=') {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let value = parts[1].trim();
                        if !value.contains("workspace") || !value.contains("true") {
                            out.push(RuleViolation {
                                file: path.to_path_buf(),
                                line: Some((idx + 1) as u32),
                                column: None,
                                rule_id: "RULE-15",
                                message: format!(
                                    "Dependency `{}` must use `workspace = true` for central management",
                                    parts[0].trim()
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "no_workspace_dependency_test.rs"]
mod tests;
