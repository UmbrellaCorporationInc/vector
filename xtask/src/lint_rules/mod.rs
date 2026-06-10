//! Project-local lint rules that `cargo quality-lint` cannot express via clippy.
//!
//! # Public API
//!
//! ```text
//! lint_rules::run(workspace: &Path, future: bool) -> Vec<RuleViolation>
//! ```
//!
//! Walks all `.rs` files under `workspace`, runs every active rule against the
//! parsed AST, and returns all violations. `future` activates rules that are
//! informational-only and do not yet affect the pass/fail decision.

mod rule;
mod rules;
mod runner;
mod walker;

pub use rule::RuleViolation;
pub use runner::run;
