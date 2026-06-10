//! Markdown runtime boundary.
//!
//! This crate owns Markdown discovery and extraction API contracts. RAG orchestration,
//! embeddings, retrieval storage, MCP transport, and governed document
//! authoring remain outside this crate.

pub mod discovery;
pub mod extraction;

pub use discovery::*;
pub use extraction::*;

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
