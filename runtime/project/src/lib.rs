//! Project bootstrap runtime operations for the vector system.
//!
//! This crate provides project-oriented plugin operations. It is
//! transport-agnostic: MCP, CLI, and future frontends depend on this crate,
//! not the other way around.
pub mod operation;
pub mod setup;

pub use operation::*;
pub use setup::*;
