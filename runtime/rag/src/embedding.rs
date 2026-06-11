//! Embedding contracts for local RAG indexing.

use crate::MarkdownChunkRecord;
use crate::defaults::{EMBEDDING_DIMENSION, EMBEDDING_MODEL_CODE, EMBEDDING_MODEL_IDENTIFIER};
use std::sync::Mutex;

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

/// Fastembed-backed embedder for the baseline `BGESmallENV15` model.
pub struct FastembedBgeSmallEnV15Embedder {
    runtime: Mutex<Box<dyn FastembedRuntime>>,
}

impl std::fmt::Debug for FastembedBgeSmallEnV15Embedder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FastembedBgeSmallEnV15Embedder")
            .field("model_id", &self.model_id())
            .field("model_code", &self.model_code())
            .field("dimension", &self.dimension())
            .finish_non_exhaustive()
    }
}

impl FastembedBgeSmallEnV15Embedder {
    /// Initialize the local fastembed runtime for `BGESmallENV15`.
    ///
    /// # Errors
    /// Returns [`EmbeddingError`] when the fastembed model registry no longer
    /// matches Vector's model contract or when fastembed cannot initialize the
    /// runtime. Model download and ONNX runtime setup happen here, keeping
    /// indexing callers dependent only on the [`Embedder`] boundary.
    pub fn try_new() -> Result<Self, EmbeddingError> {
        validate_fastembed_model_contract()?;
        let runtime = FastembedTextEmbeddingRuntime::try_new()?;
        Ok(Self::from_runtime(runtime))
    }

    /// Return the exact fastembed model code required by Vector.
    #[must_use]
    pub const fn model_code(&self) -> &'static str {
        EMBEDDING_MODEL_CODE
    }

    fn from_runtime(runtime: impl FastembedRuntime + 'static) -> Self {
        Self { runtime: Mutex::new(Box::new(runtime)) }
    }
}

impl Embedder for FastembedBgeSmallEnV15Embedder {
    fn model_id(&self) -> &str {
        EMBEDDING_MODEL_IDENTIFIER
    }

    fn dimension(&self) -> usize {
        EMBEDDING_DIMENSION
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        let embeddings = {
            let mut runtime = self.runtime.lock().map_err(|_error| {
                backend_error("Fastembed embedding runtime lock was poisoned".to_owned())
            })?;
            runtime.embed(inputs)?
        };
        validate_embedding_batch_shape(inputs.len(), self.dimension(), &embeddings)?;
        Ok(embeddings)
    }
}

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

trait FastembedRuntime: Send {
    fn embed(&mut self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError>;
}

struct FastembedTextEmbeddingRuntime {
    model: fastembed::TextEmbedding,
}

impl FastembedTextEmbeddingRuntime {
    fn try_new() -> Result<Self, EmbeddingError> {
        let options =
            fastembed::InitOptions::new(fastembed_model()).with_show_download_progress(false);
        let model = fastembed::TextEmbedding::try_new(options).map_err(|error| {
            backend_error(format!("Fastembed runtime initialization failed: {error}"))
        })?;
        Ok(Self { model })
    }
}

impl FastembedRuntime for FastembedTextEmbeddingRuntime {
    fn embed(&mut self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        self.model
            .embed(inputs, None)
            .map_err(|error| backend_error(format!("Fastembed batch embedding failed: {error}")))
    }
}

fn validate_fastembed_model_contract() -> Result<(), EmbeddingError> {
    let model_info = fastembed_model_info()?;

    if model_info.model_code != EMBEDDING_MODEL_CODE {
        return Err(backend_error(format!(
            "Fastembed BGESmallENV15 model code changed to {}; expected {}",
            model_info.model_code, EMBEDDING_MODEL_CODE
        )));
    }

    if model_info.dim != EMBEDDING_DIMENSION {
        return Err(backend_error(format!(
            "Fastembed BGESmallENV15 embedding dimension changed to {}; expected {}",
            model_info.dim, EMBEDDING_DIMENSION
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FastembedModelInfo {
    model_code: String,
    dim: usize,
}

fn fastembed_model_info() -> Result<FastembedModelInfo, EmbeddingError> {
    let model = fastembed_model();
    let model_info = fastembed::TextEmbedding::get_model_info(&model).map_err(|error| {
        backend_error(format!("Fastembed model metadata lookup failed: {error}"))
    })?;

    Ok(FastembedModelInfo { model_code: model_info.model_code.clone(), dim: model_info.dim })
}

const fn fastembed_model() -> fastembed::EmbeddingModel {
    fastembed::EmbeddingModel::BGESmallENV15
}

const fn backend_error(message: String) -> EmbeddingError {
    EmbeddingError::Backend { message }
}

#[cfg(test)]
#[path = "embedding_test.rs"]
mod tests;
