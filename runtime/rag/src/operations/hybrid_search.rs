//! Plugin operation boundary for Phase 8 hybrid retrieval.

use crate::{
    Embedder, EmbeddingError, EmbeddingVector, FastembedBgeSmallEnV15Embedder, RagDefaults,
    lifecycle::{LanceDbStoreRequest, document_predicate, lancedb_store_dir, open_primary_table},
};
use arrow_array::{Array, ListArray, RecordBatch, StringArray, UInt32Array, cast::AsArray};
use futures::TryStreamExt;
use lancedb::{
    index::scalar::FullTextSearchQuery,
    query::{ExecutableQuery, QueryBase, Select},
};
use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

const HYBRID_SEARCH_RRF_K: usize = 60;
type SectionNeighbors = (Option<String>, Option<String>);

/// Input for the `hybrid_search` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct HybridSearchInput {
    /// Workspace root used to resolve the governed RAG store.
    pub root_dir: PathBuf,
    /// Governed RAG defaults that own retrieval settings.
    pub config: RagDefaults,
    /// User query text to execute against the retrieval store.
    pub query_text: String,
    /// Optional package filter applied before ranking and fusion.
    pub package_filter: Option<String>,
    /// Optional governed document stem filter applied before ranking and fusion.
    pub document_filter: Option<String>,
    /// Optional final result count override.
    pub result_limit: Option<usize>,
}

/// One machine-readable retrieval result.
///
/// # DTO(machine-readable retrieval payload consumed by CLI and future MCP callers)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct HybridSearchResult {
    /// Package identity, or `None` for workspace-local documents.
    pub package: Option<String>,
    /// Governed document stem in `<doc-type>-<code>-<slug>` form.
    pub document_stem: String,
    /// Heading path for the winning section identity.
    pub heading_path: Vec<String>,
    /// Stable chunk identifier for debugging and traceability.
    pub chunk_id: String,
    /// Zero-based chunk ordinal within the governed document.
    pub chunk_ordinal: usize,
    /// Retrieved chunk text.
    pub text: String,
    /// Token count emitted by chunking for this stored row.
    pub token_count: usize,
    /// Semantic rank position when the chunk appears in the vector branch.
    pub semantic_rank: Option<usize>,
    /// Lexical rank position when the chunk appears in the full-text branch.
    pub lexical_rank: Option<usize>,
    /// Reciprocal Rank Fusion score after branch merging.
    pub rrf_score: f32,
    /// Previous adjacent chunk identifier in the same section, when it exists.
    pub previous_chunk_id: Option<String>,
    /// Next adjacent chunk identifier in the same section, when it exists.
    pub next_chunk_id: Option<String>,
    /// Whether the row was added by adjacent chunk expansion.
    pub was_expanded: bool,
    /// Primary hit that introduced this row through adjacent chunk expansion.
    pub expanded_from_chunk_id: Option<String>,
}

/// Output for the `hybrid_search` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct HybridSearchOutput {
    /// Normalized query text used for retrieval.
    pub query_text: String,
    /// Optional package filter after normalization.
    pub package_filter: Option<String>,
    /// Optional governed document stem filter after normalization.
    pub document_filter: Option<String>,
    /// Final result limit after governed defaults are resolved.
    pub result_limit: usize,
    /// Machine-readable retrieval results.
    pub results: Vec<HybridSearchResult>,
}

#[derive(Debug, Clone, PartialEq)]
struct RankedCandidate {
    package: Option<String>,
    document_stem: String,
    heading_path: Vec<String>,
    chunk_id: String,
    chunk_ordinal: usize,
    text: String,
    token_count: usize,
    semantic_rank: Option<usize>,
    lexical_rank: Option<usize>,
}

struct FusionRequest<'a, E> {
    table: &'a lancedb::Table,
    query_text: &'a str,
    package_filter: Option<&'a str>,
    document_filter: Option<&'a str>,
    result_limit: usize,
    semantic_limit: usize,
    lexical_limit: usize,
    embedder: &'a E,
}

#[derive(Debug)]
struct LazyFastembedQueryEmbedder {
    inner: std::sync::Mutex<Option<Arc<FastembedBgeSmallEnV15Embedder>>>,
}

impl LazyFastembedQueryEmbedder {
    const fn new() -> Self {
        Self { inner: std::sync::Mutex::new(None) }
    }
}

impl Embedder for LazyFastembedQueryEmbedder {
    fn model_id(&self) -> &str {
        crate::defaults::EMBEDDING_MODEL_IDENTIFIER
    }

    fn dimension(&self) -> usize {
        crate::defaults::EMBEDDING_DIMENSION
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<EmbeddingVector>, EmbeddingError> {
        let embedder = {
            let mut guard = self.inner.lock().map_err(|_| EmbeddingError::Backend {
                message: "lazy hybrid-search embedder lock was poisoned".to_owned(),
            })?;
            if guard.is_none() {
                *guard = Some(Arc::new(FastembedBgeSmallEnV15Embedder::try_new()?));
            }
            guard
                .as_ref()
                .ok_or_else(|| EmbeddingError::Backend {
                    message: "lazy hybrid-search embedder did not initialize".to_owned(),
                })?
                .clone()
        };
        embedder.embed_batch(inputs)
    }
}

async fn hybrid_search(
    input: HybridSearchInput,
    output: &mut impl PluginSender<HybridSearchOutput>,
) -> RuntimeResult<()> {
    let embedder = LazyFastembedQueryEmbedder::new();
    let result = run_hybrid_search(&input, &embedder).await?;
    output.send(result).await
}

async fn run_hybrid_search(
    input: &HybridSearchInput,
    embedder: &(impl Embedder + Sync),
) -> RuntimeResult<HybridSearchOutput> {
    let query_text = normalize_required(&input.query_text, "query_text")?;
    let package_filter = normalize_optional(input.package_filter.as_deref(), "package_filter")?;
    let document_filter = normalize_optional(input.document_filter.as_deref(), "document_filter")?;
    let result_limit = input.result_limit.unwrap_or_else(|| input.config.final_retrieval_limit());
    if result_limit == 0 {
        return Err(RuntimeError::operation("result_limit must be greater than zero".to_owned()));
    }

    let store_request = LanceDbStoreRequest {
        root_dir: input.root_dir.clone(),
        embedding_model: input.config.embedding_model_identifier().to_owned(),
        embedding_dimension: input.config.embedding_dimension(),
    };
    let database_dir = lancedb_store_dir(&input.root_dir);
    if !database_dir.exists() {
        return Err(RuntimeError::operation(format!(
            "RAG store is missing at '{}'; run 'vector-database rag init' or 'vector-database rag update-database' first",
            database_dir.display()
        )));
    }
    let store = crate::ensure_lancedb_store(&store_request)
        .await
        .map_err(|error| RuntimeError::operation(error.to_string()))?;
    let table = open_primary_table(&store.database_dir)
        .await
        .map_err(|error| RuntimeError::operation(error.to_string()))?;
    let row_count =
        table.count_rows(None).await.map_err(|error| RuntimeError::operation(error.to_string()))?;

    let results = if row_count == 0 {
        Vec::new()
    } else {
        execute_fusion(FusionRequest {
            table: &table,
            query_text: &query_text,
            package_filter: package_filter.as_deref(),
            document_filter: document_filter.as_deref(),
            result_limit,
            semantic_limit: input.config.semantic_retrieval_limit(),
            lexical_limit: input.config.lexical_retrieval_limit(),
            embedder,
        })
        .await?
    };

    Ok(HybridSearchOutput { query_text, package_filter, document_filter, result_limit, results })
}

async fn execute_fusion<E: Embedder + Sync>(
    request: FusionRequest<'_, E>,
) -> RuntimeResult<Vec<HybridSearchResult>> {
    let filter = candidate_filter(request.package_filter, request.document_filter);
    let semantic_candidates = execute_semantic_branch(
        request.table,
        request.query_text,
        filter.as_deref(),
        request.semantic_limit,
        request.embedder,
    )
    .await?;
    let lexical_candidates = execute_lexical_branch(
        request.table,
        request.query_text,
        filter.as_deref(),
        request.lexical_limit,
    )
    .await?;

    let mut fused: HashMap<String, RankedCandidate> = HashMap::new();
    for (rank, candidate) in semantic_candidates.into_iter().enumerate() {
        let rank = rank + 1;
        let key = candidate.chunk_id.clone();
        fused
            .entry(key)
            .and_modify(|existing| existing.semantic_rank = Some(rank))
            .or_insert(RankedCandidate { semantic_rank: Some(rank), ..candidate });
    }
    for (rank, candidate) in lexical_candidates.into_iter().enumerate() {
        let rank = rank + 1;
        let key = candidate.chunk_id.clone();
        fused
            .entry(key)
            .and_modify(|existing| existing.lexical_rank = Some(rank))
            .or_insert(RankedCandidate { lexical_rank: Some(rank), ..candidate });
    }

    let mut results = fused
        .into_values()
        .map(|candidate| HybridSearchResult {
            package: candidate.package,
            document_stem: candidate.document_stem,
            heading_path: candidate.heading_path,
            chunk_id: candidate.chunk_id,
            chunk_ordinal: candidate.chunk_ordinal,
            text: candidate.text,
            token_count: candidate.token_count,
            semantic_rank: candidate.semantic_rank,
            lexical_rank: candidate.lexical_rank,
            rrf_score: reciprocal_rank_fusion(candidate.semantic_rank, candidate.lexical_rank),
            previous_chunk_id: None,
            next_chunk_id: None,
            was_expanded: false,
            expanded_from_chunk_id: None,
        })
        .collect::<Vec<_>>();

    sort_search_results(&mut results);
    let results = deduplicate_sections(results);
    expand_adjacent_chunks(request.table, results, request.result_limit).await
}

async fn execute_semantic_branch(
    table: &lancedb::Table,
    query_text: &str,
    filter: Option<&str>,
    limit: usize,
    embedder: &(impl Embedder + Sync),
) -> RuntimeResult<Vec<RankedCandidate>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let vector = embedder
        .embed_batch(&[query_text])
        .map_err(|error| RuntimeError::operation(format!("query embedding failed: {error}")))?
        .into_iter()
        .next()
        .ok_or_else(|| RuntimeError::operation("query embedding returned no vectors".to_owned()))?;

    let mut query = table
        .vector_search(vector.as_slice())
        .map_err(|error| RuntimeError::operation(error.to_string()))?
        .select(candidate_projection())
        .limit(limit);
    if let Some(filter) = filter {
        query = query.only_if(filter);
    }

    let stream =
        query.execute().await.map_err(|error| RuntimeError::operation(error.to_string()))?;
    collect_candidates(stream).await
}

async fn execute_lexical_branch(
    table: &lancedb::Table,
    query_text: &str,
    filter: Option<&str>,
    limit: usize,
) -> RuntimeResult<Vec<RankedCandidate>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut query = table
        .query()
        .full_text_search(FullTextSearchQuery::new(query_text.to_owned()))
        .select(candidate_projection())
        .limit(limit);
    if let Some(filter) = filter {
        query = query.only_if(filter);
    }

    let stream =
        query.execute().await.map_err(|error| RuntimeError::operation(error.to_string()))?;
    collect_candidates(stream).await
}

async fn collect_candidates(
    stream: lancedb::arrow::SendableRecordBatchStream,
) -> RuntimeResult<Vec<RankedCandidate>> {
    let batches: Vec<RecordBatch> =
        stream.try_collect().await.map_err(|error| RuntimeError::operation(error.to_string()))?;
    let mut candidates = Vec::new();
    for batch in &batches {
        candidates.extend(parse_candidate_batch(batch)?);
    }
    Ok(candidates)
}

fn parse_candidate_batch(batch: &RecordBatch) -> RuntimeResult<Vec<RankedCandidate>> {
    let package_col =
        batch.column_by_name("package").ok_or_else(|| missing_column_error("package"))?;
    let document_stem_col = batch
        .column_by_name("document_stem")
        .ok_or_else(|| missing_column_error("document_stem"))?;
    let heading_path_col =
        batch.column_by_name("heading_path").ok_or_else(|| missing_column_error("heading_path"))?;
    let chunk_id_col =
        batch.column_by_name("chunk_id").ok_or_else(|| missing_column_error("chunk_id"))?;
    let chunk_ordinal_col = batch
        .column_by_name("chunk_ordinal")
        .ok_or_else(|| missing_column_error("chunk_ordinal"))?;
    let text_col = batch.column_by_name("text").ok_or_else(|| missing_column_error("text"))?;
    let token_count_col =
        batch.column_by_name("token_count").ok_or_else(|| missing_column_error("token_count"))?;

    let packages = package_col.as_string::<i32>();
    let stems = document_stem_col.as_string::<i32>();
    let chunk_ids = chunk_id_col.as_string::<i32>();
    let chunk_ordinals =
        chunk_ordinal_col.as_any().downcast_ref::<UInt32Array>().ok_or_else(|| {
            RuntimeError::operation("chunk_ordinal column is not a UInt32Array".to_owned())
        })?;
    let texts = text_col.as_string::<i32>();
    let token_counts = token_count_col.as_any().downcast_ref::<UInt32Array>().ok_or_else(|| {
        RuntimeError::operation("token_count column is not a UInt32Array".to_owned())
    })?;
    let heading_paths = heading_path_col.as_any().downcast_ref::<ListArray>().ok_or_else(|| {
        RuntimeError::operation("heading_path column is not a ListArray".to_owned())
    })?;

    let mut candidates = Vec::with_capacity(batch.num_rows());
    for row in 0..batch.num_rows() {
        let package =
            if packages.is_null(row) { None } else { Some(packages.value(row).to_owned()) };
        candidates.push(RankedCandidate {
            package,
            document_stem: stems.value(row).to_owned(),
            heading_path: heading_segments(heading_paths, row)?,
            chunk_id: chunk_ids.value(row).to_owned(),
            chunk_ordinal: usize::try_from(chunk_ordinals.value(row)).unwrap_or(usize::MAX),
            text: texts.value(row).to_owned(),
            token_count: usize::try_from(token_counts.value(row)).unwrap_or(usize::MAX),
            semantic_rank: None,
            lexical_rank: None,
        });
    }
    Ok(candidates)
}

fn heading_segments(heading_paths: &ListArray, row: usize) -> RuntimeResult<Vec<String>> {
    let values = heading_paths.value(row);
    let strings = values.as_any().downcast_ref::<StringArray>().ok_or_else(|| {
        RuntimeError::operation("heading_path list values are not Utf8".to_owned())
    })?;
    let mut segments = Vec::with_capacity(strings.len());
    for index in 0..strings.len() {
        if strings.is_null(index) {
            return Err(RuntimeError::operation(
                "heading_path contained an unexpected null segment".to_owned(),
            ));
        }
        segments.push(strings.value(index).to_owned());
    }
    Ok(segments)
}

fn candidate_projection() -> Select {
    Select::columns(&[
        "package",
        "document_stem",
        "heading_path",
        "chunk_id",
        "chunk_ordinal",
        "text",
        "token_count",
    ])
}

fn sort_search_results(results: &mut [HybridSearchResult]) {
    results.sort_by(|left, right| {
        right
            .rrf_score
            .total_cmp(&left.rrf_score)
            .then_with(|| {
                left.semantic_rank
                    .unwrap_or(usize::MAX)
                    .cmp(&right.semantic_rank.unwrap_or(usize::MAX))
            })
            .then_with(|| {
                left.lexical_rank
                    .unwrap_or(usize::MAX)
                    .cmp(&right.lexical_rank.unwrap_or(usize::MAX))
            })
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
}

fn deduplicate_sections(results: Vec<HybridSearchResult>) -> Vec<HybridSearchResult> {
    let mut seen_sections = HashSet::new();
    let mut deduplicated = Vec::new();
    for result in results {
        let section_key = section_identity_key(
            result.package.as_deref(),
            &result.document_stem,
            &result.heading_path,
        );
        if seen_sections.insert(section_key) {
            deduplicated.push(result);
        }
    }
    deduplicated
}

async fn expand_adjacent_chunks(
    table: &lancedb::Table,
    deduplicated_results: Vec<HybridSearchResult>,
    result_limit: usize,
) -> RuntimeResult<Vec<HybridSearchResult>> {
    let mut document_cache = HashMap::<String, Vec<RankedCandidate>>::new();
    let mut primaries = deduplicated_results.into_iter().take(result_limit).collect::<Vec<_>>();

    for primary in &mut primaries {
        let document_chunks = load_document_chunks(table, primary, &mut document_cache).await?;
        apply_neighbor_metadata(primary, &document_chunks);
    }

    if primaries.len() >= result_limit {
        return Ok(primaries);
    }

    let mut results = primaries.clone();
    let mut seen_chunk_ids =
        results.iter().map(|result| result.chunk_id.clone()).collect::<HashSet<_>>();

    for primary in &primaries {
        let document_chunks = load_document_chunks(table, primary, &mut document_cache).await?;
        for expanded in expand_section_neighbors(primary, &document_chunks) {
            if results.len() >= result_limit {
                return Ok(results);
            }
            if seen_chunk_ids.insert(expanded.chunk_id.clone()) {
                results.push(expanded);
            }
        }
    }

    Ok(results)
}

async fn load_document_chunks(
    table: &lancedb::Table,
    result: &HybridSearchResult,
    cache: &mut HashMap<String, Vec<RankedCandidate>>,
) -> RuntimeResult<Vec<RankedCandidate>> {
    let cache_key = document_identity_key(result.package.as_deref(), &result.document_stem);
    if let Some(cached) = cache.get(&cache_key) {
        return Ok(cached.clone());
    }

    let predicate = document_predicate(result.package.as_deref(), &result.document_stem, None);
    let stream = table
        .query()
        .only_if(predicate)
        .select(candidate_projection())
        .execute()
        .await
        .map_err(|error| RuntimeError::operation(error.to_string()))?;
    let mut rows = collect_candidates(stream).await?;
    rows.sort_by(|left, right| {
        left.chunk_ordinal
            .cmp(&right.chunk_ordinal)
            .then_with(|| left.chunk_id.cmp(&right.chunk_id))
    });
    cache.insert(cache_key, rows.clone());
    Ok(rows)
}

fn apply_neighbor_metadata(result: &mut HybridSearchResult, document_chunks: &[RankedCandidate]) {
    let neighbors = section_neighbors(document_chunks, result);
    result.previous_chunk_id = neighbors.0;
    result.next_chunk_id = neighbors.1;
}

fn expand_section_neighbors(
    primary: &HybridSearchResult,
    document_chunks: &[RankedCandidate],
) -> Vec<HybridSearchResult> {
    let Some(primary_index) =
        document_chunks.iter().position(|chunk| chunk.chunk_id == primary.chunk_id)
    else {
        return Vec::new();
    };

    let same_section = document_chunks
        .iter()
        .enumerate()
        .filter(|(_, chunk)| {
            chunk.package == primary.package
                && chunk.document_stem == primary.document_stem
                && chunk.heading_path == primary.heading_path
        })
        .collect::<Vec<_>>();

    let mut expanded = Vec::new();
    for offset in [-1_isize, 1_isize] {
        let Some(candidate_index) = primary_index.checked_add_signed(offset) else {
            continue;
        };
        let Some(candidate) = document_chunks.get(candidate_index) else {
            continue;
        };
        if candidate.package != primary.package
            || candidate.document_stem != primary.document_stem
            || candidate.heading_path != primary.heading_path
        {
            continue;
        }

        let previous_chunk_id = same_section
            .iter()
            .position(|(_, chunk)| chunk.chunk_id == candidate.chunk_id)
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| same_section.get(index))
            .map(|(_, chunk)| chunk.chunk_id.clone());
        let next_chunk_id = same_section
            .iter()
            .position(|(_, chunk)| chunk.chunk_id == candidate.chunk_id)
            .and_then(|index| same_section.get(index + 1))
            .map(|(_, chunk)| chunk.chunk_id.clone());

        expanded.push(HybridSearchResult {
            package: candidate.package.clone(),
            document_stem: candidate.document_stem.clone(),
            heading_path: candidate.heading_path.clone(),
            chunk_id: candidate.chunk_id.clone(),
            chunk_ordinal: candidate.chunk_ordinal,
            text: candidate.text.clone(),
            token_count: candidate.token_count,
            semantic_rank: None,
            lexical_rank: None,
            rrf_score: primary.rrf_score,
            previous_chunk_id,
            next_chunk_id,
            was_expanded: true,
            expanded_from_chunk_id: Some(primary.chunk_id.clone()),
        });
    }

    expanded
}

fn section_neighbors(
    document_chunks: &[RankedCandidate],
    result: &HybridSearchResult,
) -> SectionNeighbors {
    let section_chunks = document_chunks
        .iter()
        .filter(|chunk| {
            chunk.package == result.package
                && chunk.document_stem == result.document_stem
                && chunk.heading_path == result.heading_path
        })
        .collect::<Vec<_>>();
    let Some(index) = section_chunks.iter().position(|chunk| chunk.chunk_id == result.chunk_id)
    else {
        return (None, None);
    };
    let previous = index.checked_sub(1).and_then(|previous| section_chunks.get(previous));
    let next = section_chunks.get(index + 1);
    (previous.map(|chunk| chunk.chunk_id.clone()), next.map(|chunk| chunk.chunk_id.clone()))
}

fn document_identity_key(package: Option<&str>, document_stem: &str) -> String {
    format!("{}::{document_stem}", package.unwrap_or(""))
}

fn section_identity_key(
    package: Option<&str>,
    document_stem: &str,
    heading_path: &[String],
) -> String {
    format!("{}::{document_stem}::{}", package.unwrap_or(""), heading_path.join("\u{1f}"))
}

fn reciprocal_rank_fusion(semantic_rank: Option<usize>, lexical_rank: Option<usize>) -> f32 {
    rank_contribution(semantic_rank) + rank_contribution(lexical_rank)
}

fn rank_contribution(rank: Option<usize>) -> f32 {
    rank.map_or(0.0, |rank| {
        let denominator = u16::try_from(HYBRID_SEARCH_RRF_K + rank).unwrap_or(u16::MAX);
        1.0 / f32::from(denominator)
    })
}

fn candidate_filter(package_filter: Option<&str>, document_filter: Option<&str>) -> Option<String> {
    let mut predicates = Vec::new();
    if let Some(package_filter) = package_filter {
        predicates.push(format!("package = '{}'", sql_string_literal(package_filter)));
    }
    if let Some(document_filter) = document_filter {
        predicates.push(format!("document_stem = '{}'", sql_string_literal(document_filter)));
    }
    if predicates.is_empty() { None } else { Some(predicates.join(" AND ")) }
}

fn sql_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn missing_column_error(column: &str) -> RuntimeError {
    RuntimeError::operation(format!("candidate query result is missing '{column}'"))
}

fn normalize_required(value: &str, field_name: &str) -> RuntimeResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(RuntimeError::operation(format!("{field_name} must not be empty")));
    }
    Ok(trimmed.to_owned())
}

fn normalize_optional(value: Option<&str>, field_name: &str) -> RuntimeResult<Option<String>> {
    value.map(|text| normalize_required(text, field_name)).transpose()
}

declare_plugin_operations! {
    /// Operation boundary for Phase 8 hybrid retrieval.
    HybridSearchOp => hybrid_search(HybridSearchInput, HybridSearchOutput)
}

impl HybridSearchInput {
    /// Construct a `HybridSearchInput` with explicit fields.
    #[must_use]
    pub const fn new(
        root_dir: PathBuf,
        config: RagDefaults,
        query_text: String,
        package_filter: Option<String>,
        document_filter: Option<String>,
        result_limit: Option<usize>,
    ) -> Self {
        Self { root_dir, config, query_text, package_filter, document_filter, result_limit }
    }
}

impl HybridSearchResult {
    /// Construct one machine-readable hybrid retrieval result.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        package: Option<String>,
        document_stem: String,
        heading_path: Vec<String>,
        chunk_id: String,
        chunk_ordinal: usize,
        text: String,
        token_count: usize,
        semantic_rank: Option<usize>,
        lexical_rank: Option<usize>,
        rrf_score: f32,
        previous_chunk_id: Option<String>,
        next_chunk_id: Option<String>,
        was_expanded: bool,
        expanded_from_chunk_id: Option<String>,
    ) -> Self {
        Self {
            package,
            document_stem,
            heading_path,
            chunk_id,
            chunk_ordinal,
            text,
            token_count,
            semantic_rank,
            lexical_rank,
            rrf_score,
            previous_chunk_id,
            next_chunk_id,
            was_expanded,
            expanded_from_chunk_id,
        }
    }
}

impl HybridSearchOutput {
    /// Construct the machine-readable hybrid retrieval response payload.
    #[must_use]
    pub const fn new(
        query_text: String,
        package_filter: Option<String>,
        document_filter: Option<String>,
        result_limit: usize,
        results: Vec<HybridSearchResult>,
    ) -> Self {
        Self { query_text, package_filter, document_filter, result_limit, results }
    }
}

impl HybridSearchOp {
    /// Construct a new `HybridSearchOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for HybridSearchOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "hybrid_search_test.rs"]
mod tests;
