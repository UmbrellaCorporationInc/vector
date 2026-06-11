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
