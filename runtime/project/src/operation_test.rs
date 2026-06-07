#![allow(clippy::unwrap_used)]

use super::*;
use runtime_core::{RuntimeResult, channel::Sender, operation::FlowOperation};

// A mock cancelable sender to capture outputs during test.
struct MockSender {
    outputs: Vec<CreateProjectOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl Sender<CreateProjectOutput> for MockSender {
    async fn send(&mut self, value: CreateProjectOutput) -> RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<CreateProjectOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_create_project_op_contract() {
    let input = CreateProjectInput {
        target_dir: IoPath::new("test_project_dir"),
        project_name: "test_project".to_string(),
        force: false,
    };

    let mut sender = MockSender::new();
    let op = CreateProjectOp;

    op.run(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs.len(), 1);
    let output = &sender.outputs[0];
    assert!(output.message.contains("test_project"));
    assert!(output.message.contains("test_project_dir"));

    // Cleanup
    let _ = std::fs::remove_dir_all("test_project_dir");
}

#[tokio::test]
async fn test_create_project_provisions_files() {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("vector_test_{now}"));
    let target_dir = IoPath::new(&temp_dir);

    let input =
        CreateProjectInput { target_dir, project_name: "test_provision".to_string(), force: false };

    let mut sender = MockSender::new();
    let op = CreateProjectOp;

    op.run(input, &mut sender).await.unwrap();

    // Verify a subset of critical files
    assert!(temp_dir.join(".vector/document-types.yaml").exists());
    assert!(temp_dir.join(".vector/dashboards/project-status.yaml").exists());
    assert!(temp_dir.join(".agents/mcp_config.json").exists());
    assert!(temp_dir.join(".geminiignore").exists());
    assert!(temp_dir.join(".gemini/settings.json").exists());
    assert!(temp_dir.join(".gemini/antigravity-cli/settings.json").exists());
    assert!(temp_dir.join(".gemini/antigravity-cli/mcp_config.json").exists());
    assert!(temp_dir.join("CLAUDE.md").exists());
    assert!(temp_dir.join("opencode.json").exists());
    assert!(temp_dir.join("doc/ai-rule/active/ai-rule-00000-master-dispatcher.md").exists());
    assert!(temp_dir.join("doc/template/project/template-00002-spec.md").exists());
    assert!(temp_dir.join("doc/template/ai/template-00006-documentation.md").exists());
    assert!(temp_dir.join("doc/template/project/template-00004-doc-type-template.md").exists());
    assert!(temp_dir.join("doc/template/project/template-00005-doc-type-prompt.md").exists());
    assert!(temp_dir.join("doc/prompts/doc-type/prompts-00001-create-doc-type.md").exists());
    assert!(temp_dir.join("doc/prompts/authoring/prompts-00002-create-doc.md").exists());
    assert!(temp_dir.join("doc/prompts/authoring/prompts-00003-create-task.md").exists());
    assert!(temp_dir.join("doc/prompts/actions/prompts-00004-execute-task-phase.md").exists());
    assert!(
        temp_dir
            .join("doc/prompts/actions/prompts-00007-validate-fix-repository-governance-flow.md")
            .exists()
    );
    assert!(temp_dir.join("doc/prompts/form-actions/prompts-00005-create-document.md").exists());
    assert!(temp_dir.join("doc/prompts/form-actions/prompts-00006-update-document.md").exists());
    assert!(temp_dir.join("doc/prompts/quality-gate/prompts-00008-rust.md").exists());
    assert!(temp_dir.join("doc/prompts/quality-gate/prompts-00009-typescript.md").exists());
    assert!(temp_dir.join("doc/form/form-00001-create-document.md").exists());
    assert!(temp_dir.join("doc/template/project/template-00007-task.md").exists());
    assert!(temp_dir.join("doc/template/project/template-00008-rfc.md").exists());
    assert!(
        temp_dir
            .join("doc/template/project/template-00009-project-definition-template.md")
            .exists()
    );
    assert!(
        temp_dir
            .join("doc/template/project/template-00010-language-dependency-governance-template.md")
            .exists()
    );
    assert!(
        temp_dir
            .join("doc/template/project/template-00011-project-principles-template.md")
            .exists()
    );
    assert!(temp_dir.join("doc/template/project/template-00012-package-readme.md").exists());
    assert_eq!(
        std::fs::read_to_string(temp_dir.join("opencode.json")).unwrap(),
        include_str!("../assets/opencode.json")
    );

    let gitignore_path = temp_dir.join(".gitignore");
    assert!(gitignore_path.exists());
    let gitignore_content = std::fs::read_to_string(&gitignore_path).unwrap();
    assert!(
        gitignore_content.contains(".vector-database/"),
        "gitignore must exclude .vector-database/"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_create_project_skip_existing_policy() {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("vector_test_skip_{now}"));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a conflict
    let conflict_file = temp_dir.join("CLAUDE.md");
    std::fs::write(&conflict_file, "Original content").unwrap();

    let target_dir = IoPath::new(&temp_dir);
    let mut sender = MockSender::new();
    let op = CreateProjectOp;

    let input =
        CreateProjectInput { target_dir, project_name: "test_skip".to_string(), force: false };

    // Should succeed and report skipped file
    op.run(input, &mut sender).await.unwrap();

    assert_eq!(sender.outputs.len(), 1);
    let output = &sender.outputs[0];
    assert!(output.skipped_files.contains(&"CLAUDE.md".to_string()));
    assert_eq!(std::fs::read_to_string(&conflict_file).unwrap(), "Original content");

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_create_project_repeated_preserves_gitignore() {
    let now =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("vector_test_repeat_{now}"));
    let target_dir = IoPath::new(&temp_dir);

    let input = CreateProjectInput {
        target_dir: target_dir.clone(),
        project_name: "test_repeat".to_string(),
        force: false,
    };

    let mut sender = MockSender::new();
    let op = CreateProjectOp;

    // Run first time
    op.run(input.clone(), &mut sender).await.unwrap();

    let gitignore_path = temp_dir.join(".gitignore");
    assert!(gitignore_path.exists());
    let content_first = std::fs::read_to_string(&gitignore_path).unwrap();
    assert!(content_first.contains(".vector-database/"));

    // Run second time (repeated bootstrap/setup flow)
    let mut sender2 = MockSender::new();
    op.run(input, &mut sender2).await.unwrap();

    let content_second = std::fs::read_to_string(&gitignore_path).unwrap();
    assert!(content_second.contains(".vector-database/"));
    assert_eq!(
        content_first, content_second,
        "repeated run should not mutate gitignore if skipped"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_dir);
}
