#![allow(clippy::expect_used)]

use rmcp::handler::server::wrapper::Parameters;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_io::path::IoPath;
use runtime_project::{ProjectSetupInput, ProjectSetupOp};

/// Verifies that `ProjectSetupOp` can be selected and dispatched through
/// the standard `PluginDispatcher` path without any MCP types involved.
///
/// This test covers the bridge shape: select op → hand off input → consume receiver.
/// It does not test MCP protocol encoding or tool registration.
#[tokio::test]
async fn project_setup_op_dispatches_through_dispatcher() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = IoPath::new(dir.path());

    let input = ProjectSetupInput::new(target, "test-project".to_string(), false);

    let (_cancel, mut receiver) = PluginDispatcher::new(ProjectSetupOp::default())
        .input(input)
        .build()
        .expect("dispatcher build must succeed with a valid input");

    let mut received_count = 0usize;
    while let Ok(Some(output)) = receiver.recv().await {
        assert!(
            !output.project.message.is_empty(),
            "project setup output must carry a non-empty message"
        );
        received_count += 1;
    }

    assert_eq!(received_count, 1, "ProjectSetupOp must emit exactly one output value");
}

/// Verifies that `PluginDispatcher::build` returns an error when no input
/// is supplied, enforcing the contract that input is mandatory before dispatch.
#[test]
fn dispatcher_build_requires_input() {
    let result = PluginDispatcher::new(ProjectSetupOp::default()).build();
    assert!(result.is_err(), "build without input must return an error");
}

/// Verifies that `CreateProjectParams` deserializes correctly and that
/// the force field defaults to false — boundary shape test without
/// invoking any MCP protocol machinery.
#[test]
fn create_project_params_defaults_force_to_false() {
    let raw = r#"{"target_dir": "/tmp/x", "project_name": "p"}"#;
    let params: super::CreateProjectParams =
        serde_json::from_str(raw).expect("must deserialize without force field");
    assert!(!params.force, "force must default to false when absent from JSON");
}

/// Verifies that `ProjectTools::new` and `Default` both construct a usable
/// MCP adapter instance for the project capability group.
#[test]
fn project_tools_constructors_produce_usable_adapter() {
    let _from_new = super::ProjectTools::new();
    let _from_default = super::ProjectTools::default();
}

/// Verifies that the MCP-facing `create_project` tool executes the real
/// `ProjectSetupOp` path and returns the aggregated project message.
///
/// This covers the adapter boundary directly: MCP params -> runtime input ->
/// dispatcher execution -> receiver consumption -> MCP-facing string result.
#[tokio::test]
async fn create_project_tool_executes_project_setup_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target_dir = dir.path().join("project");

    let tools = super::ProjectTools::new();
    let result = tools
        .create_project(Parameters(super::CreateProjectParams {
            target_dir: target_dir.display().to_string(),
            project_name: "vector-project".to_string(),
            force: false,
        }))
        .await
        .expect("tool execution must succeed with valid params");

    assert!(
        result.contains("vector-project"),
        "tool result must include the project name from runtime output"
    );

    let rule_path = target_dir
        .join("doc")
        .join("ai-rule")
        .join("active")
        .join("ai-rule-00003-documentation.md");
    assert!(rule_path.exists(), "tool path must leave the documentation rule generated");

    let rule_content = std::fs::read_to_string(&rule_path).expect("rule file must be readable");
    assert!(
        !rule_content.contains("#{types}"),
        "documentation rule generated through the tool must resolve template placeholders"
    );
    assert!(
        rule_content.contains("**document type:**"),
        "documentation rule generated through the tool must include resolved document type entries"
    );
}

/// Verifies that the tool can target an existing directory and still complete
/// the runtime setup path when files need to be skipped.
#[tokio::test]
async fn create_project_tool_succeeds_for_existing_target_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target_dir = dir.path().join("existing-project");
    std::fs::create_dir_all(&target_dir).expect("existing target dir");

    let tools = super::ProjectTools::default();
    let result = tools
        .create_project(Parameters(super::CreateProjectParams {
            target_dir: target_dir.display().to_string(),
            project_name: "existing-project".to_string(),
            force: false,
        }))
        .await
        .expect("tool execution must succeed for an existing target dir");

    assert!(
        result.contains("existing-project"),
        "tool result must still report the selected project name"
    );

    assert!(
        target_dir.join(".vector").join("document-types.yaml").exists(),
        "bootstrap assets must still exist after the tool completes"
    );
}

/// Verifies that `update_project` leaves existing files untouched while
/// provisioning any missing assets into an already-initialised project.
#[tokio::test]
async fn update_project_does_not_overwrite_existing_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target_dir = dir.path().join("update-project");

    // Bootstrap a partial project manually
    std::fs::create_dir_all(&target_dir).expect("create target dir");
    let existing_file = target_dir.join("CLAUDE.md");
    std::fs::write(&existing_file, "original content").expect("write existing file");

    let tools = super::ProjectTools::new();
    let result = tools
        .update_project(Parameters(super::UpdateProjectParams {
            target_dir: target_dir.display().to_string(),
            project_name: "update-project".to_string(),
        }))
        .await
        .expect("update_project must succeed for an existing project");

    assert!(result.contains("update-project"), "tool result must include the project name");

    // Existing file must remain unchanged
    assert_eq!(
        std::fs::read_to_string(&existing_file).expect("read existing file"),
        "original content",
        "update_project must not overwrite existing files"
    );
}

/// Verifies that `update_project` creates missing assets in an existing
/// project directory.
#[tokio::test]
async fn update_project_creates_missing_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target_dir = dir.path().join("update-project-missing");

    // Create only the root directory, nothing else
    std::fs::create_dir_all(&target_dir).expect("create target dir");

    let tools = super::ProjectTools::new();
    tools
        .update_project(Parameters(super::UpdateProjectParams {
            target_dir: target_dir.display().to_string(),
            project_name: "update-project-missing".to_string(),
        }))
        .await
        .expect("update_project must succeed");

    // Missing bootstrap assets must now exist
    assert!(
        target_dir.join(".vector").join("document-types.yaml").exists(),
        "update_project must create missing bootstrap assets"
    );
    assert!(target_dir.join("AGENTS.md").exists(), "update_project must create missing AGENTS.md");
}
