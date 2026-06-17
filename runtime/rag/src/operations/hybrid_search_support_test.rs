#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn result(
    chunk_id: &str,
    score: f32,
    semantic_rank: Option<usize>,
    lexical_rank: Option<usize>,
) -> HybridSearchResult {
    HybridSearchResult::new(
        None,
        "task-00001-example".to_owned(),
        vec!["Heading".to_owned()],
        chunk_id.to_owned(),
        0,
        "body".to_owned(),
        10,
        semantic_rank,
        lexical_rank,
        score,
        None,
        None,
        false,
        None,
    )
}

#[test]
fn sort_search_results_orders_by_score_then_ranks() {
    let mut results = vec![
        result("chunk-b", 0.5, Some(2), Some(2)),
        result("chunk-a", 0.6, Some(3), Some(3)),
        result("chunk-c", 0.5, Some(1), Some(2)),
    ];

    sort_search_results(&mut results);

    assert_eq!(results[0].chunk_id, "chunk-a");
    assert_eq!(results[1].chunk_id, "chunk-c");
    assert_eq!(results[2].chunk_id, "chunk-b");
}

#[test]
fn deduplicate_sections_keeps_first_result_per_section_identity() {
    let first = result("chunk-a", 0.7, Some(1), None);
    let duplicate = result("chunk-b", 0.6, Some(2), None);
    let mut other_section = result("chunk-c", 0.5, Some(3), None);
    other_section.heading_path = vec!["Other".to_owned()];

    let deduplicated = deduplicate_sections(vec![first.clone(), duplicate, other_section.clone()]);

    assert_eq!(deduplicated.len(), 2);
    assert_eq!(deduplicated[0].chunk_id, first.chunk_id);
    assert_eq!(deduplicated[1].chunk_id, other_section.chunk_id);
}
