#![allow(clippy::panic, clippy::unwrap_used)]

use super::*;

#[test]
fn test_embed_markdown_chunks_accepts_empty_batches() {
    let model = DeterministicEmbedder::new("test-model", 2);

    let records = embed_markdown_chunks(&model, &[]).unwrap();

    assert!(records.is_empty());
}

#[test]
fn test_embed_markdown_chunks_adds_model_metadata_for_single_chunk() {
    let model = DeterministicEmbedder::new("test-model", 2);
    let chunks = vec![chunk("chunk-0000", 0, "alpha beta")];

    let records = embed_markdown_chunks(&model, &chunks).unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].chunk.text, "alpha beta");
    assert_eq!(records[0].embedding_model, "test-model");
    assert_eq!(records[0].embedding_dimension, 2);
    assert_eq!(records[0].embedding, vec![10.0, 2.0]);
}

#[test]
fn test_embed_markdown_chunks_preserves_multiple_input_chunks_in_order() {
    let model = DeterministicEmbedder::new("test-model", 2);
    let chunks = vec![
        chunk("chunk-0000", 0, "first chunk"),
        chunk("chunk-0001", 1, "second chunk with more words"),
    ];

    let records = embed_markdown_chunks(&model, &chunks).unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].chunk.chunk_id, "chunk-0000");
    assert_eq!(records[0].embedding, vec![11.0, 2.0]);
    assert_eq!(records[1].chunk.chunk_id, "chunk-0001");
    assert_eq!(records[1].embedding, vec![28.0, 5.0]);
}

#[test]
fn test_embed_markdown_chunks_rejects_dimension_mismatch() {
    let embedder = MismatchedDimensionEmbedder::new();
    let chunks = vec![chunk("chunk-0000", 0, "alpha beta")];

    let error = embed_markdown_chunks(&embedder, &chunks).unwrap_err();

    assert_eq!(
        error,
        EmbeddingError::DimensionMismatch {
            chunk_index: 0,
            expected_dimension: 2,
            actual_dimension: 1,
        }
    );
}

#[test]
fn test_embed_markdown_chunks_rejects_batch_length_mismatch() {
    let embedder = ShortBatchEmbedder::new();
    let chunks =
        vec![chunk("chunk-0000", 0, "first chunk"), chunk("chunk-0001", 1, "second chunk")];

    let error = embed_markdown_chunks(&embedder, &chunks).unwrap_err();

    assert_eq!(error, EmbeddingError::BatchLengthMismatch { input_count: 2, output_count: 1 });
}

#[test]
fn test_fastembed_bge_small_en_v15_contract_matches_rag_defaults() {
    let model_info = fastembed_model_info().unwrap();

    assert_eq!(model_info.model_code, EMBEDDING_MODEL_CODE);
    assert_eq!(model_info.dim, EMBEDDING_DIMENSION);
    validate_fastembed_model_contract().unwrap();
}

#[test]
fn test_fastembed_bge_small_en_v15_embedder_exposes_model_contract() {
    let embedder = FastembedBgeSmallEnV15Embedder::from_runtime(FakeFastembedRuntime::new(384));

    assert_eq!(embedder.model_id(), EMBEDDING_MODEL_IDENTIFIER);
    assert_eq!(embedder.model_code(), EMBEDDING_MODEL_CODE);
    assert_eq!(embedder.dimension(), EMBEDDING_DIMENSION);
}

#[test]
fn test_fastembed_bge_small_en_v15_embedder_delegates_batch_embedding() {
    let embedder = FastembedBgeSmallEnV15Embedder::from_runtime(FakeFastembedRuntime::new(384));

    let embeddings = embedder.embed_batch(&["alpha", "beta"]).unwrap();

    assert_eq!(embeddings.len(), 2);
    assert_eq!(embeddings[0].len(), EMBEDDING_DIMENSION);
    assert_eq!(embeddings[0][0].to_bits(), 5.0_f32.to_bits());
    assert_eq!(embeddings[1][0].to_bits(), 4.0_f32.to_bits());
}

#[test]
fn test_fastembed_bge_small_en_v15_embedder_rejects_invalid_runtime_dimensions() {
    let embedder = FastembedBgeSmallEnV15Embedder::from_runtime(FakeFastembedRuntime::new(383));

    let error = embedder.embed_batch(&["alpha"]).unwrap_err();

    assert_eq!(
        error,
        EmbeddingError::DimensionMismatch {
            chunk_index: 0,
            expected_dimension: EMBEDDING_DIMENSION,
            actual_dimension: 383,
        }
    );
}

#[derive(Debug, Clone)]
struct DeterministicEmbedder {
    model_id: String,
    dimension: usize,
}

impl DeterministicEmbedder {
    fn new(model_id: &str, dimension: usize) -> Self {
        Self { model_id: model_id.to_owned(), dimension }
    }
}

impl Embedder for DeterministicEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let byte_len = u16::try_from(input.len()).unwrap();
                let word_count = u16::try_from(input.split_whitespace().count()).unwrap();
                vec![f32::from(byte_len), f32::from(word_count)]
            })
            .collect())
    }
}

#[derive(Debug, Clone)]
struct MismatchedDimensionEmbedder {
    model_id: String,
}

impl MismatchedDimensionEmbedder {
    fn new() -> Self {
        Self { model_id: "bad-model".to_owned() }
    }
}

impl Embedder for MismatchedDimensionEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        2
    }

    fn embed_batch(&self, _inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(vec![vec![1.0]])
    }
}

#[derive(Debug, Clone)]
struct ShortBatchEmbedder {
    model_id: String,
}

impl ShortBatchEmbedder {
    fn new() -> Self {
        Self { model_id: "short-batch-model".to_owned() }
    }
}

impl Embedder for ShortBatchEmbedder {
    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        2
    }

    fn embed_batch(&self, _inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(vec![vec![1.0, 2.0]])
    }
}

#[derive(Debug, Clone)]
struct FakeFastembedRuntime {
    dimension: usize,
}

impl FakeFastembedRuntime {
    fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

impl FastembedRuntime for FakeFastembedRuntime {
    fn embed(&mut self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let mut embedding = vec![0.0; self.dimension];
                if let Some(first_dimension) = embedding.first_mut() {
                    *first_dimension = f32::from(u16::try_from(input.len()).unwrap());
                }
                embedding
            })
            .collect())
    }
}

fn chunk(chunk_id: &str, chunk_ordinal: usize, text: &str) -> MarkdownChunkRecord {
    MarkdownChunkRecord {
        chunk_id: chunk_id.to_owned(),
        package: None,
        document_stem: "spec-00011-rag-plan-implementation".to_owned(),
        document_hash: "document-hash".to_owned(),
        chunk_hash: format!("chunk-hash-{chunk_ordinal}"),
        chunk_ordinal,
        heading_path: vec!["Title".to_owned()],
        text: text.to_owned(),
        token_count: text.split_whitespace().count(),
        previous_chunk_id: None,
        next_chunk_id: None,
    }
}
