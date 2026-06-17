#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_fixture_root(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    std::env::temp_dir().join(format!("vector-index-workspace-test-{label}-{nanos}"))
}

#[tokio::test]
async fn index_workspace_op_initializes_store_and_delivers_index_result() {
    let root_dir = unique_fixture_root("basic");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let outputs = collect_outputs(&mut receiver).await;
    let summary = expect_summary(outputs.last().expect("expected final summary output"));

    assert!(matches!(
        outputs.first(),
        Some(IndexWorkspaceOutput::Progress(IndexWorkspaceProgress { label, .. }))
            if label == "initializing-store"
    ));
    assert_eq!(summary.skipped_count, 0);
    assert_eq!(summary.reindexed_count, 0);
    assert_eq!(summary.deleted_count, 0);
    assert!(summary.failures.is_empty());
    assert!(!summary.has_failures());
}

#[tokio::test]
async fn index_workspace_op_is_idempotent_on_repeated_runs() {
    let root_dir = unique_fixture_root("idempotent");
    let make_input = || IndexWorkspaceInput::new(root_dir.clone(), RagDefaults::phase_one());

    let (_cancel, mut rx1) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(make_input())
        .build()
        .expect("first dispatcher build failed");
    let first = collect_outputs(&mut rx1).await;

    let (_cancel, mut rx2) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(make_input())
        .build()
        .expect("second dispatcher build failed");
    let second = collect_outputs(&mut rx2).await;

    assert!(!expect_summary(first.last().expect("first summary")).has_failures());
    assert!(!expect_summary(second.last().expect("second summary")).has_failures());
}

#[tokio::test]
async fn index_workspace_op_receiver_is_none_after_single_output() {
    let root_dir = unique_fixture_root("single-output");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut rx) =
        PluginDispatcher::new(IndexWorkspaceOp::new()).input(input).build().unwrap();

    let outputs = collect_outputs(&mut rx).await;

    assert!(!outputs.is_empty(), "expected progress and summary outputs");
    assert!(matches!(outputs.last(), Some(IndexWorkspaceOutput::Summary(_))));
}

#[tokio::test]
async fn index_workspace_op_emits_document_progress_before_final_summary() {
    let root_dir = unique_fixture_root("document-progress");
    std::fs::create_dir_all(root_dir.join(".vector")).expect("failed to create .vector root");
    std::fs::create_dir_all(root_dir.join("doc")).expect("failed to create doc directory");
    std::fs::write(
        root_dir.join("doc").join("spec-00002-index-progress.md"),
        "# Index Progress\n\nThis file should produce incremental progress.\n",
    )
    .expect("failed to write governed document");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let outputs = collect_outputs(&mut receiver).await;

    assert!(
        outputs.iter().any(|output| matches!(
            output,
            IndexWorkspaceOutput::Progress(IndexWorkspaceProgress {
                label,
                document_stem: Some(document_stem),
                ..
            }) if label == "indexed" && document_stem == "spec-00002-index-progress"
        )),
        "expected indexed progress event"
    );
    assert!(matches!(outputs.last(), Some(IndexWorkspaceOutput::Summary(_))));
}

#[tokio::test]
async fn index_workspace_op_emits_failed_progress_for_document_errors() {
    let root_dir = unique_fixture_root("document-failure");
    std::fs::create_dir_all(root_dir.join(".vector")).expect("failed to create .vector root");
    std::fs::create_dir_all(root_dir.join("doc")).expect("failed to create doc directory");
    std::fs::write(
        root_dir.join("doc").join("spec-00003-bad-frontmatter.md"),
        "---\ntitle: [unterminated\n---\n# Broken\n",
    )
    .expect("failed to write malformed governed document");
    let input = IndexWorkspaceInput::new(root_dir, RagDefaults::phase_one());

    let (_cancel, mut receiver) = PluginDispatcher::new(IndexWorkspaceOp::new())
        .input(input)
        .build()
        .expect("dispatcher build failed");

    let outputs = collect_outputs(&mut receiver).await;

    assert!(
        outputs.iter().any(|output| matches!(
            output,
            IndexWorkspaceOutput::Progress(IndexWorkspaceProgress {
                label,
                document_stem: Some(document_stem),
                ..
            }) if label == "failed" && document_stem == "spec-00003-bad-frontmatter"
        )),
        "expected failed progress event"
    );
    let summary = expect_summary(outputs.last().expect("expected final summary output"));
    assert!(summary.has_failures(), "malformed document should record a failure");
}

async fn collect_outputs(
    receiver: &mut impl Receiver<IndexWorkspaceOutput>,
) -> Vec<IndexWorkspaceOutput> {
    let mut outputs = Vec::new();
    while let Some(output) = receiver.recv().await.expect("channel error") {
        outputs.push(output);
    }
    outputs
}

fn expect_summary(output: &IndexWorkspaceOutput) -> &IndexResult {
    let IndexWorkspaceOutput::Summary(result) = output else {
        unreachable!("expected summary output");
    };
    result
}
