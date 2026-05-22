//! Workspace version source of truth.
//!
//! `[workspace.package].version` in the root `Cargo.toml` is the single
//! source of truth defined by SPEC 00008. Because `mcp-vector` inherits
//! `version.workspace = true`, `CARGO_PKG_VERSION` always reflects that
//! workspace version at compile time.

/// Returns the workspace version declared in the root `Cargo.toml`.
///
/// Resolved at compile time via `CARGO_PKG_VERSION`, which is injected by
/// Cargo from `[workspace.package].version`. All release and version surfaces
/// in this crate derive their value from this function.
#[must_use]
pub fn workspace_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
#[path = "version_test.rs"]
mod tests;
