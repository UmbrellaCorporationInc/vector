//! Local RAG defaults and Markdown discovery orchestration.

use runtime_markdown::{MarkdownDiscoveryRequest, PackageMarkdownRoot};
use std::path::PathBuf;

/// Workspace-local governed document corpus root.
pub const WORKSPACE_CORPUS_ROOT: &str = "doc";

/// Synchronized package storage root.
pub const PACKAGE_STORAGE_ROOT: &str = ".vector-database/packages";

/// Governed document folder inside every synchronized package.
pub const PACKAGE_DOCUMENT_DIR: &str = "doc";

/// Local RAG persistence root.
pub const RAG_STORAGE_ROOT: &str = ".vector-database/rag";

/// `LanceDB` storage path for the first local RAG implementation.
pub const LANCEDB_STORAGE_PATH: &str = ".vector-database/rag/lancedb";

/// Baseline embedding model identifier.
pub const EMBEDDING_MODEL_IDENTIFIER: &str = "BGESmallENV15";

/// Baseline embedding model code.
pub const EMBEDDING_MODEL_CODE: &str = "Xenova/bge-small-en-v1.5";

/// Baseline embedding vector dimension.
pub const EMBEDDING_DIMENSION: usize = 384;

/// Target chunk token count.
pub const CHUNK_TOKEN_TARGET: usize = 350;

/// Maximum chunk token count.
pub const CHUNK_TOKEN_MAXIMUM: usize = 500;

/// Semantic retrieval candidate limit.
pub const SEMANTIC_RETRIEVAL_LIMIT: usize = 20;

/// Lexical retrieval candidate limit.
pub const LEXICAL_RETRIEVAL_LIMIT: usize = 20;

/// Final retrieval result limit.
pub const FINAL_RETRIEVAL_LIMIT: usize = 8;

/// Fixed Phase 1 RAG defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RagDefaults {
    workspace_corpus_root: &'static str,
    package_storage_root: &'static str,
    package_document_dir: &'static str,
    rag_storage_root: &'static str,
    lancedb_storage_path: &'static str,
    embedding_model_identifier: &'static str,
    embedding_model_code: &'static str,
    embedding_dimension: usize,
    chunk_token_target: usize,
    chunk_token_maximum: usize,
    semantic_retrieval_limit: usize,
    lexical_retrieval_limit: usize,
    final_retrieval_limit: usize,
}

impl Default for RagDefaults {
    fn default() -> Self {
        Self::phase_one()
    }
}

impl RagDefaults {
    /// Return the fixed Phase 1 local RAG defaults.
    #[must_use]
    pub const fn phase_one() -> Self {
        Self {
            workspace_corpus_root: WORKSPACE_CORPUS_ROOT,
            package_storage_root: PACKAGE_STORAGE_ROOT,
            package_document_dir: PACKAGE_DOCUMENT_DIR,
            rag_storage_root: RAG_STORAGE_ROOT,
            lancedb_storage_path: LANCEDB_STORAGE_PATH,
            embedding_model_identifier: EMBEDDING_MODEL_IDENTIFIER,
            embedding_model_code: EMBEDDING_MODEL_CODE,
            embedding_dimension: EMBEDDING_DIMENSION,
            chunk_token_target: CHUNK_TOKEN_TARGET,
            chunk_token_maximum: CHUNK_TOKEN_MAXIMUM,
            semantic_retrieval_limit: SEMANTIC_RETRIEVAL_LIMIT,
            lexical_retrieval_limit: LEXICAL_RETRIEVAL_LIMIT,
            final_retrieval_limit: FINAL_RETRIEVAL_LIMIT,
        }
    }

    /// Return the workspace-local corpus root.
    #[must_use]
    pub const fn workspace_corpus_root(&self) -> &'static str {
        self.workspace_corpus_root
    }

    /// Return the synchronized package storage root.
    #[must_use]
    pub const fn package_storage_root(&self) -> &'static str {
        self.package_storage_root
    }

    /// Return the document directory name inside a synchronized package.
    #[must_use]
    pub const fn package_document_dir(&self) -> &'static str {
        self.package_document_dir
    }

    /// Return the local RAG persistence root.
    #[must_use]
    pub const fn rag_storage_root(&self) -> &'static str {
        self.rag_storage_root
    }

    /// Return the `LanceDB` storage path.
    #[must_use]
    pub const fn lancedb_storage_path(&self) -> &'static str {
        self.lancedb_storage_path
    }

    /// Return the baseline embedding model identifier.
    #[must_use]
    pub const fn embedding_model_identifier(&self) -> &'static str {
        self.embedding_model_identifier
    }

    /// Return the baseline embedding model code.
    #[must_use]
    pub const fn embedding_model_code(&self) -> &'static str {
        self.embedding_model_code
    }

    /// Return the baseline embedding dimension.
    #[must_use]
    pub const fn embedding_dimension(&self) -> usize {
        self.embedding_dimension
    }

    /// Return the target chunk token count.
    #[must_use]
    pub const fn chunk_token_target(&self) -> usize {
        self.chunk_token_target
    }

    /// Return the maximum chunk token count.
    #[must_use]
    pub const fn chunk_token_maximum(&self) -> usize {
        self.chunk_token_maximum
    }

    /// Return the semantic retrieval candidate limit.
    #[must_use]
    pub const fn semantic_retrieval_limit(&self) -> usize {
        self.semantic_retrieval_limit
    }

    /// Return the lexical retrieval candidate limit.
    #[must_use]
    pub const fn lexical_retrieval_limit(&self) -> usize {
        self.lexical_retrieval_limit
    }

    /// Return the final retrieval result limit.
    #[must_use]
    pub const fn final_retrieval_limit(&self) -> usize {
        self.final_retrieval_limit
    }

    /// Translate RAG corpus defaults into an explicit Markdown discovery request.
    #[must_use]
    pub fn markdown_discovery_request(
        &self,
        package_names: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> MarkdownDiscoveryRequest {
        let mut package_doc_roots = package_names
            .into_iter()
            .map(|package_name| {
                let package_name = package_name.as_ref().to_owned();
                let doc_root = PathBuf::from(self.package_storage_root)
                    .join(package_name.as_str())
                    .join(self.package_document_dir);

                PackageMarkdownRoot::new(package_name, doc_root)
            })
            .collect::<Vec<_>>();

        package_doc_roots.sort_by(|left, right| left.package().cmp(right.package()));

        MarkdownDiscoveryRequest::new([self.workspace_corpus_root], package_doc_roots)
    }
}

#[cfg(test)]
#[path = "defaults_test.rs"]
mod tests;
