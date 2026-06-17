use super::{
    HybridSearchResult, RankedCandidate, RuntimeError, RuntimeResult, candidate_projection,
};
use crate::lifecycle::document_predicate;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::collections::{HashMap, HashSet};

type SectionNeighbors = (Option<String>, Option<String>);

pub(super) fn sort_search_results(results: &mut [HybridSearchResult]) {
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

pub(super) fn deduplicate_sections(results: Vec<HybridSearchResult>) -> Vec<HybridSearchResult> {
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

pub(super) async fn expand_adjacent_chunks(
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
    let mut rows = super::collect_candidates(stream).await?;
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

#[cfg(test)]
#[path = "hybrid_search_support_test.rs"]
mod tests;
