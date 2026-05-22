//! Release and version runtime owned by `mcp/vector`.
//!
//! This module is the single canonical owner of workspace version truth.
//! No other crate in the workspace may introduce a second source of truth
//! for version or release metadata.

/// Workspace version source of truth derived from the root `Cargo.toml`.
pub mod version;
