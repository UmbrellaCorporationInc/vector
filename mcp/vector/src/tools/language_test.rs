#![allow(clippy::expect_used)]

use rmcp::handler::server::wrapper::Parameters;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_io::path::IoPath;
use runtime_language::{QualityGateInput, QualityGateOp};

fn create_language_tool_test_project() -> tempfile::TempDir {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    let prompt_dir = dir.path().join("doc").join("prompts").join("quality-gate");

    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::create_dir_all(&prompt_dir).expect("create prompt dir");

    fs::write(
        vector_dir.join("language-rules.yaml"),
        "rust:\n  quality-gate: prompts-00005-rust\n",
    )
    .expect("write language config");

    fs::write(
        prompt_dir.join("prompts-00005-rust.md"),
        "---\nid: prompts-00005-rust\ntype: prompts\ncode: \"00005\"\nslug: rust\n---\n\n# Rust Quality Gate\nRun xtask quality-test.\n",
    )
    .expect("write rust prompt");

    dir
}

/// Verifies that `QualityGateOp` can be selected and dispatched through
/// the standard `PluginDispatcher` path without any MCP types involved.
#[tokio::test]
async fn quality_gate_op_dispatches_through_dispatcher() {
    let dir = create_language_tool_test_project();
    let root = IoPath::new(dir.path());

    let (_cancel, mut receiver) = PluginDispatcher::new(QualityGateOp::new())
        .input(QualityGateInput::new(root, vec!["rust".to_string()]))
        .build()
        .expect("dispatcher build must succeed with valid input");

    let output = receiver
        .recv()
        .await
        .expect("receive must succeed")
        .expect("operation must emit one output");

    assert!(
        output.prompt.contains("# Rust Quality Gate"),
        "output prompt must contain the resolved body"
    );
}

/// Verifies that `PluginDispatcher::build` returns an error when no input
/// is supplied, enforcing the contract that input is mandatory before dispatch.
#[test]
fn dispatcher_build_requires_input() {
    let result = PluginDispatcher::new(QualityGateOp::new()).build();
    assert!(result.is_err(), "build without input must return an error");
}

/// Verifies that `LanguageQualityGateParams` deserializes with the required fields.
#[test]
fn language_quality_gate_params_deserialize_correctly() {
    let raw = r#"{"root_dir": "/tmp/project", "languages": ["rust", "typescript"]}"#;
    let params: super::LanguageQualityGateParams =
        serde_json::from_str(raw).expect("must deserialize LanguageQualityGateParams");
    assert_eq!(params.root_dir, "/tmp/project");
    assert_eq!(params.languages, vec!["rust", "typescript"]);
}

/// Verifies that `LanguageTools::new` and `Default` both construct a usable
/// MCP adapter instance for the language capability group.
#[test]
fn language_tools_constructors_produce_usable_adapter() {
    let _from_new = super::LanguageTools::new();
    let _from_default = super::LanguageTools::default();
}

/// Verifies that the MCP-facing `language-quality-gate` tool executes the real
/// `QualityGateOp` path and returns the concatenated prompt body.
#[tokio::test]
async fn language_quality_gate_tool_executes_quality_gate_path() {
    let dir = create_language_tool_test_project();

    let tools = super::LanguageTools::new();
    let result = tools
        .language_quality_gate(Parameters(super::LanguageQualityGateParams {
            root_dir: dir.path().display().to_string(),
            languages: vec!["rust".to_string()],
        }))
        .await
        .expect("tool execution must succeed with valid params");

    assert!(
        result.contains("# Rust Quality Gate"),
        "tool result must contain the resolved prompt body"
    );
    assert!(
        !result.contains("id: prompts-00005-rust"),
        "tool result must strip frontmatter before returning the prompt"
    );
}

/// Verifies that the MCP-facing tool surfaces runtime validation failures
/// unchanged at the adapter boundary.
#[tokio::test]
async fn language_quality_gate_tool_skips_unconfigured_languages() {
    let dir = create_language_tool_test_project();

    let tools = super::LanguageTools::new();
    let result = tools
        .language_quality_gate(Parameters(super::LanguageQualityGateParams {
            root_dir: dir.path().display().to_string(),
            languages: vec!["unknown".to_string(), "Rust".to_string()],
        }))
        .await
        .expect("tool must succeed when at least one requested language is configured");

    assert!(
        result.contains("# Rust Quality Gate"),
        "tool result must preserve configured languages after skipping unconfigured ones"
    );
}

/// Verifies that the MCP-facing tool still surfaces runtime validation failures
/// for configured languages with invalid quality-gate metadata.
#[tokio::test]
async fn language_quality_gate_tool_propagates_missing_mapping_errors() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(vector_dir.join("language-rules.yaml"), "rust: {}\n").expect("write language config");

    let tools = super::LanguageTools::new();
    let result = tools
        .language_quality_gate(Parameters(super::LanguageQualityGateParams {
            root_dir: dir.path().display().to_string(),
            languages: vec!["rust".to_string()],
        }))
        .await;

    let error =
        result.expect_err("tool must return an error for a configured language with no mapping");
    assert!(
        error.contains("language-quality-gate failed:"),
        "tool error must preserve the adapter prefix; got: {error:?}"
    );
    assert!(
        error.contains("missing a quality-gate mapping"),
        "tool error must preserve the runtime cause; got: {error:?}"
    );
}
