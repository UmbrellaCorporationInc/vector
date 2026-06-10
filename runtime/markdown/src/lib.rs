//! Markdown runtime boundary.
//!
//! This crate owns Markdown discovery API contracts. RAG orchestration,
//! embeddings, retrieval storage, MCP transport, and governed document
//! authoring remain outside this crate.

pub mod discovery;

pub use discovery::*;

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
