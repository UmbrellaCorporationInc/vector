//! Embedding contracts for local RAG indexing.

use crate::MarkdownChunkRecord;

/// Dense vector emitted by an embedding model.
pub type EmbeddingVector = Vec<f32>;

/// Embedding backend boundary used by the RAG indexing pipeline.
pub trait Embedder {
    /// Return the stable model identifier associated with emitted vectors.
    fn model_id(&self) -> &str;

    /// Return the required vector dimension for this embedder.
    fn dimension(&self) -> usize;

    /// Embed a batch of chunk text inputs.
    ///
    /// # Errors
    /// Returns [`EmbeddingError`] when the backend cannot produce embeddings
    /// for the requested batch.
    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError>;
}

/// Markdown chunk record after embedding and metadata validation.
///
/// # DTO(embedding output consumed by downstream storage phases)
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EmbeddedMarkdownChunkRecord {
    /// Original chunk content and source metadata.
    pub chunk: MarkdownChunkRecord,
    /// Stable embedding model identifier used to produce `embedding`.
    pub embedding_model: String,
    /// Required embedding dimension for this model.
    pub embedding_dimension: usize,
    /// Dense embedding vector for `chunk.text`.
    pub embedding: EmbeddingVector,
}

/// Embedding failure before downstream storage writes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EmbeddingError {
    /// The embedder produced a different number of vectors than requested.
    BatchLengthMismatch {
        /// Number of chunk inputs requested.
        input_count: usize,
        /// Number of vectors returned by the embedder.
        output_count: usize,
    },
    /// An emitted vector does not match the embedder's declared dimension.
    DimensionMismatch {
        /// Index of the chunk with an invalid vector.
        chunk_index: usize,
        /// Expected vector length from [`Embedder::dimension`].
        expected_dimension: usize,
        /// Actual emitted vector length.
        actual_dimension: usize,
    },
    /// Backend-specific embedding failure.
    Backend {
        /// Human-readable backend failure message.
        message: String,
    },
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BatchLengthMismatch { input_count, output_count } => write!(
                formatter,
                "Embedder returned {output_count} vectors for {input_count} input chunks"
            ),
            Self::DimensionMismatch { chunk_index, expected_dimension, actual_dimension } => {
                write!(
                    formatter,
                    "Embedding vector at chunk index {chunk_index} has dimension {actual_dimension}; expected {expected_dimension}"
                )
            }
            Self::Backend { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for EmbeddingError {}

/// Embed Markdown chunks and validate vector shape before downstream writes.
///
/// # Errors
/// Returns [`EmbeddingError`] when the embedder fails, returns the wrong batch
/// size, or emits any vector whose length differs from
/// [`Embedder::dimension`].
pub fn embed_markdown_chunks(
    embedder: &impl Embedder,
    chunks: &[MarkdownChunkRecord],
) -> Result<Vec<EmbeddedMarkdownChunkRecord>, EmbeddingError> {
    let inputs = chunks.iter().map(|chunk| chunk.text.as_str()).collect::<Vec<_>>();
    let embeddings = embedder.embed_batch(&inputs)?;

    validate_embedding_batch_shape(chunks.len(), embedder.dimension(), &embeddings)?;

    Ok(chunks
        .iter()
        .cloned()
        .zip(embeddings)
        .map(|(chunk, embedding)| EmbeddedMarkdownChunkRecord {
            chunk,
            embedding_model: embedder.model_id().to_owned(),
            embedding_dimension: embedder.dimension(),
            embedding,
        })
        .collect())
}

fn validate_embedding_batch_shape(
    input_count: usize,
    expected_dimension: usize,
    embeddings: &[EmbeddingVector],
) -> Result<(), EmbeddingError> {
    if embeddings.len() != input_count {
        return Err(EmbeddingError::BatchLengthMismatch {
            input_count,
            output_count: embeddings.len(),
        });
    }

    for (chunk_index, embedding) in embeddings.iter().enumerate() {
        if embedding.len() != expected_dimension {
            return Err(EmbeddingError::DimensionMismatch {
                chunk_index,
                expected_dimension,
                actual_dimension: embedding.len(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "embedding_test.rs"]
mod tests;
