#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn unique_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let root = std::env::temp_dir().join(format!("vector-rag-cli-{label}-{nanos}"));
    std::fs::create_dir_all(root.join(".vector")).expect("failed to create .vector root");
    root
}

#[test]
fn parse_args_routes_rag_init() {
    assert_eq!(parse_args(&args(&["vector-rag", "rag", "init"])), CliAction::RagInit);
}

#[test]
fn parse_args_routes_rag_search_with_passthrough_args() {
    assert_eq!(
        parse_args(&args(&[
            "vector-rag",
            "rag",
            "search",
            "hybrid retrieval",
            "--package",
            "shared-docs",
            "--limit",
            "3",
            "--json",
        ])),
        CliAction::RagSearch(args(&[
            "hybrid retrieval",
            "--package",
            "shared-docs",
            "--limit",
            "3",
            "--json",
        ]))
    );
}

#[test]
fn parse_args_routes_rag_update_database() {
    assert_eq!(
        parse_args(&args(&["vector-rag", "rag", "update-database"])),
        CliAction::RagUpdateDatabase(Vec::new())
    );
}

#[test]
fn parse_args_routes_rag_update_database_with_json_flag() {
    assert_eq!(
        parse_args(&args(&["vector-rag", "rag", "update-database", "--json"])),
        CliAction::RagUpdateDatabase(args(&["--json"]))
    );
}

#[test]
fn parse_args_reports_unknown_rag_subcommand() {
    assert_eq!(
        parse_args(&args(&["vector-rag", "rag", "rebuild"])),
        CliAction::Unknown("rag rebuild".to_owned())
    );
}

#[test]
fn parse_search_args_preserves_query_filters_limit_and_json() {
    let parsed = rag_search::parse_args(&args(&[
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
fn parse_search_args_rejects_missing_query() {
    let error =
        rag_search::parse_args(&args(&["--limit", "5"])).expect_err("missing query must fail");

    assert_eq!(error, "missing search query");
}

#[tokio::test]
async fn rag_dispatch_runs_init_against_workspace_root() {
    let root = unique_root("init");

    handle_rag_command(&root, CliAction::RagInit)
        .await
        .expect("rag init dispatch should create the local store");

    let store_dir = root.join(".vector-database").join("rag").join("lancedb");
    assert!(store_dir.exists(), "expected local LanceDB directory at {store_dir:?}");
}

#[tokio::test]
async fn rag_dispatch_rejects_search_without_query_before_runtime_execution() {
    let root = unique_root("missing-query");
    let error = handle_rag_command(&root, CliAction::RagSearch(Vec::new()))
        .await
        .expect_err("missing search query should fail before runtime dispatch");

    assert_eq!(error, "missing search query");
}

#[test]
fn parse_update_database_args_accepts_json_only() {
    let parsed =
        parse_update_database_args(&args(&["--json"])).expect("update-database args should parse");

    assert!(parsed.json_output());
}

#[test]
fn parse_update_database_args_rejects_unknown_option() {
    let error = parse_update_database_args(&args(&["--stream"]))
        .expect_err("unknown update-database option must fail");

    assert_eq!(error, "unknown rag update-database option '--stream'; only --json is supported");
}
