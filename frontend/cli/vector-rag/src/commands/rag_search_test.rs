#![allow(clippy::expect_used)]

use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn parse_args_preserves_query_filters_limit_and_json() {
    let parsed = parse_args(&args(&[
        "hybrid",
        "retrieval",
        "--package",
        "shared-docs",
        "--document",
        "spec-00011-rag-plan",
        "--limit",
        "5",
        "--json",
    ]))
    .expect("search args should parse");

    assert_eq!(parsed.query_text(), "hybrid retrieval");
    assert_eq!(parsed.package_filter(), Some("shared-docs"));
    assert_eq!(parsed.document_filter(), Some("spec-00011-rag-plan"));
    assert_eq!(parsed.result_limit(), Some(5));
    assert!(parsed.json_output());
}

#[test]
fn parse_args_rejects_unknown_flags() {
    let error = parse_args(&args(&["query", "--unknown"])).expect_err("unknown flags should fail");

    assert_eq!(error, "unknown rag search flag '--unknown'");
}
