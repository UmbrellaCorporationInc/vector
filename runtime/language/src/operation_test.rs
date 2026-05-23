#![allow(clippy::unwrap_used, clippy::expect_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::{
    BestPracticesInput, BestPracticesOp, BestPracticesOutput, QualityGateInput, QualityGateOp,
    QualityGateOutput,
};

// ---------------------------------------------------------------------------
// Mock senders
// ---------------------------------------------------------------------------

struct QualityGateMockSender {
    output: Option<QualityGateOutput>,
}

impl QualityGateMockSender {
    fn new() -> Self {
        Self { output: None }
    }
}

impl runtime_core::Sender<QualityGateOutput> for QualityGateMockSender {
    async fn send(&mut self, value: QualityGateOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<QualityGateOutput> for QualityGateMockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

struct BestPracticesMockSender {
    output: Option<BestPracticesOutput>,
}

impl BestPracticesMockSender {
    fn new() -> Self {
        Self { output: None }
    }
}

impl runtime_core::Sender<BestPracticesOutput> for BestPracticesMockSender {
    async fn send(&mut self, value: BestPracticesOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<BestPracticesOutput> for BestPracticesMockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_test_project() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    fs::create_dir_all(temp_dir.path().join(".vector")).unwrap();
    fs::create_dir_all(temp_dir.path().join("doc").join("prompts").join("quality-gate")).unwrap();
    fs::create_dir_all(temp_dir.path().join("doc").join("prompts").join("best-practices")).unwrap();

    (temp_dir, root)
}

fn write_language_rules(temp_dir: &TempDir, content: &str) {
    fs::write(temp_dir.path().join(".vector").join("language-rules.yaml"), content).unwrap();
}

fn write_prompt(temp_dir: &TempDir, relative_path: &str, content: &str) {
    let path = temp_dir.path().join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn governed_prompt(title: &str, body: &str) -> String {
    format!(
        "---\nid: prompts-00000-{title}\ntype: prompts\ncode: \"00000\"\nslug: {title}\ntitle: {title}\ndescription: test\ncategory: quality-gate\ncreated: 2026-05-11\nupdated: 2026-05-11\ntags: []\n---\n\n{body}"
    )
}

// ---------------------------------------------------------------------------
// QualityGate tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn loads_language_rules_and_returns_prompt_body() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00005-rust.md",
        &governed_prompt("rust", "# Rust Gate\n"),
    );

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Gate\n");
}

#[tokio::test]
async fn rejects_empty_languages_input() {
    let (_temp_dir, root) = create_test_project();
    let mut sender = QualityGateMockSender::new();

    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, Vec::new()), &mut sender)
        .await
        .expect_err("expected empty languages to fail");

    assert!(error.to_string().contains("languages input must not be empty"));
}

#[tokio::test]
async fn rejects_duplicate_languages_after_normalization() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");

    let mut sender = QualityGateMockSender::new();
    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["Rust".to_string(), "rust".to_string()]), &mut sender)
        .await
        .expect_err("expected duplicate language to fail");

    assert!(error.to_string().contains("duplicate language 'rust'"));
}

#[tokio::test]
async fn rejects_missing_language_rules_config() {
    let (_temp_dir, root) = create_test_project();
    let mut sender = QualityGateMockSender::new();

    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected missing config to fail");

    assert!(error.to_string().contains("failed to read .vector/language-rules.yaml"));
}

#[tokio::test]
async fn resolves_mixed_case_language_names() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00005-rust.md",
        &governed_prompt("rust", "# Rust Gate\n"),
    );

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["Rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Gate\n");
}

#[tokio::test]
async fn skips_quality_gate_when_field_is_none_sentinel() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: none\n");

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect("quality-gate: none must be treated as not configured");

    let output = sender.output.expect("operation must emit output");
    assert!(output.prompt.is_empty(), "prompt must be empty when field is the none sentinel");
}

#[tokio::test]
async fn skips_language_without_quality_gate_mapping() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust: {}\n");

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect("language with no quality-gate mapping must be skipped silently");

    let output = sender.output.expect("operation must emit output");
    assert!(output.prompt.is_empty(), "prompt must be empty when no mapping is configured");
}

#[tokio::test]
async fn rejects_unmapped_prompt_reference() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");

    let mut sender = QualityGateMockSender::new();
    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected missing prompt document to fail");

    assert!(error.to_string().contains("did not resolve to any governed prompts document"));
}

#[tokio::test]
async fn strips_frontmatter_from_prompt_output() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00005-rust.md",
        "---\nid: prompts-00005-rust\n---\n\n# Rust Gate\nUse xtask quality-test.\n",
    );

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Gate\nUse xtask quality-test.\n");
    assert!(!output.prompt.contains("id: prompts-00005-rust"));
}

#[tokio::test]
async fn concatenates_prompt_bodies_in_input_order() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(
        &temp_dir,
        "rust:\n  quality-gate: prompts-00005-rust\ntypescript:\n  quality-gate: prompts-00006-typescript\n",
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00005-rust.md",
        &governed_prompt("rust", "# Rust Gate\n"),
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00006-typescript.md",
        &governed_prompt("typescript", "# TypeScript Gate\n"),
    );

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(
            QualityGateInput::new(root, vec!["typescript".to_string(), "rust".to_string()]),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# TypeScript Gate\n\n\n\n# Rust Gate\n");
}

#[tokio::test]
async fn skips_unconfigured_languages_and_preserves_configured_order() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(
        &temp_dir,
        "rust:\n  quality-gate: prompts-00005-rust\ntypescript:\n  quality-gate: prompts-00006-typescript\n",
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00005-rust.md",
        &governed_prompt("rust", "# Rust Gate\n"),
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/quality-gate/prompts-00006-typescript.md",
        &governed_prompt("typescript", "# TypeScript Gate\n"),
    );

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(
            QualityGateInput::new(
                root,
                vec!["TypeScript".to_string(), "Python".to_string(), "Rust".to_string()],
            ),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# TypeScript Gate\n\n\n\n# Rust Gate\n");
}

#[tokio::test]
async fn returns_empty_prompt_when_all_requested_languages_are_unconfigured() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality-gate: prompts-00005-rust\n");

    let mut sender = QualityGateMockSender::new();
    QualityGateOp::new()
        .run(
            QualityGateInput::new(root, vec!["Python".to_string(), "Ruby".to_string()]),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "");
}

#[tokio::test]
async fn rejects_snake_case_language_rule_field_names() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  quality_gate: prompts-00005-rust\n");

    let mut sender = QualityGateMockSender::new();
    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected snake_case field name to fail");

    assert!(error.to_string().contains("quality_gate"));
}

#[tokio::test]
async fn rejects_snake_case_nested_language_rule_field_names_with_exact_path() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  gates:\n    quality_gate: prompts-00005-rust\n");

    let mut sender = QualityGateMockSender::new();
    let error = QualityGateOp::new()
        .run(QualityGateInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected nested snake_case field name to fail");

    assert!(error.to_string().contains(".vector/language-rules.yaml"));
    assert!(error.to_string().contains("gates.quality_gate"));
}

// ---------------------------------------------------------------------------
// BestPractices tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bp_loads_language_rules_and_returns_prompt_body() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        &governed_prompt("rust", "# Rust Best Practices\n"),
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Best Practices\n");
}

#[tokio::test]
async fn bp_rejects_empty_languages_input() {
    let (_temp_dir, root) = create_test_project();
    let mut sender = BestPracticesMockSender::new();

    let error = BestPracticesOp::new()
        .run(BestPracticesInput::new(root, Vec::new()), &mut sender)
        .await
        .expect_err("expected empty languages to fail");

    assert!(error.to_string().contains("languages input must not be empty"));
}

#[tokio::test]
async fn bp_rejects_duplicate_languages_after_normalization() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");

    let mut sender = BestPracticesMockSender::new();
    let error = BestPracticesOp::new()
        .run(
            BestPracticesInput::new(root, vec!["Rust".to_string(), "rust".to_string()]),
            &mut sender,
        )
        .await
        .expect_err("expected duplicate language to fail");

    assert!(error.to_string().contains("duplicate language 'rust'"));
}

#[tokio::test]
async fn bp_rejects_missing_language_rules_config() {
    let (_temp_dir, root) = create_test_project();
    let mut sender = BestPracticesMockSender::new();

    let error = BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected missing config to fail");

    assert!(error.to_string().contains("failed to read .vector/language-rules.yaml"));
}

#[tokio::test]
async fn bp_resolves_mixed_case_language_names() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        &governed_prompt("rust", "# Rust Best Practices\n"),
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["Rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Best Practices\n");
}

#[tokio::test]
async fn bp_skips_best_practices_when_field_is_none_sentinel() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: none\n");

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect("best-practices: none must be treated as not configured");

    let output = sender.output.expect("operation must emit output");
    assert!(output.prompt.is_empty(), "prompt must be empty when field is the none sentinel");
}

#[tokio::test]
async fn bp_skips_language_without_best_practices_mapping() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust: {}\n");

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect("language with no best-practices mapping must be skipped silently");

    let output = sender.output.expect("operation must emit output");
    assert!(output.prompt.is_empty(), "prompt must be empty when no mapping is configured");
}

#[tokio::test]
async fn bp_rejects_unmapped_prompt_reference() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");

    let mut sender = BestPracticesMockSender::new();
    let error = BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .expect_err("expected missing prompt document to fail");

    assert!(error.to_string().contains("did not resolve to any governed prompts document"));
}

#[tokio::test]
async fn bp_strips_frontmatter_from_prompt_output() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        "---\nid: prompts-00010-rust\n---\n\n# Rust Best Practices\nFollow idiomatic Rust.\n",
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Best Practices\nFollow idiomatic Rust.\n");
    assert!(!output.prompt.contains("id: prompts-00010-rust"));
}

#[tokio::test]
async fn bp_concatenates_prompt_bodies_in_input_order() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(
        &temp_dir,
        "rust:\n  best-practices: prompts-00010-rust\ntypescript:\n  best-practices: prompts-00011-typescript\n",
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        &governed_prompt("rust", "# Rust Best Practices\n"),
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00011-typescript.md",
        &governed_prompt("typescript", "# TypeScript Best Practices\n"),
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(
            BestPracticesInput::new(root, vec!["typescript".to_string(), "rust".to_string()]),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# TypeScript Best Practices\n\n\n\n# Rust Best Practices\n");
}

#[tokio::test]
async fn bp_skips_unconfigured_languages_and_preserves_configured_order() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(
        &temp_dir,
        "rust:\n  best-practices: prompts-00010-rust\ntypescript:\n  best-practices: prompts-00011-typescript\n",
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        &governed_prompt("rust", "# Rust Best Practices\n"),
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00011-typescript.md",
        &governed_prompt("typescript", "# TypeScript Best Practices\n"),
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(
            BestPracticesInput::new(
                root,
                vec!["TypeScript".to_string(), "Python".to_string(), "Rust".to_string()],
            ),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# TypeScript Best Practices\n\n\n\n# Rust Best Practices\n");
}

#[tokio::test]
async fn bp_returns_empty_prompt_when_all_requested_languages_are_unconfigured() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(&temp_dir, "rust:\n  best-practices: prompts-00010-rust\n");

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(
            BestPracticesInput::new(root, vec!["Python".to_string(), "Ruby".to_string()]),
            &mut sender,
        )
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "");
}

#[tokio::test]
async fn bp_config_with_both_fields_resolves_best_practices_independently() {
    let (temp_dir, root) = create_test_project();
    write_language_rules(
        &temp_dir,
        "rust:\n  quality-gate: prompts-00005-rust\n  best-practices: prompts-00010-rust\n",
    );
    write_prompt(
        &temp_dir,
        "doc/prompts/best-practices/prompts-00010-rust.md",
        &governed_prompt("rust", "# Rust Best Practices\n"),
    );

    let mut sender = BestPracticesMockSender::new();
    BestPracticesOp::new()
        .run(BestPracticesInput::new(root, vec!["rust".to_string()]), &mut sender)
        .await
        .unwrap();

    let output = sender.output.expect("expected output");
    assert_eq!(output.prompt, "\n# Rust Best Practices\n");
}
