//! MCP facade for the vector system.
//!
//! This crate owns the MCP server bootstrap, transport setup, tool registration,
//! request decoding, response encoding, and MCP-specific error mapping.
//!
//! Reusable domain logic lives in `runtime-core` and other runtime crates.
//! `rmcp` types do not cross the boundary of this crate into runtime contracts.

/// MCP-local error types for the vector server.
pub mod error;

/// MCP server bootstrap and central handler composition.
pub mod server;

/// MCP tool groups organized by capability domain.
pub mod tools;
