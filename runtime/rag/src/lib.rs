//! Retrieval-augmented generation runtime boundary.
//!
//! This crate owns local RAG defaults and orchestration. Markdown-specific
//! discovery contracts are delegated to `runtime-markdown`.

pub mod chunking;
pub mod defaults;
pub mod embedding;
pub mod lifecycle;
pub mod pipeline;
pub mod storage;

pub use chunking::*;
pub use defaults::*;
pub use embedding::*;
pub use lifecycle::*;
pub use pipeline::*;
pub use storage::*;
