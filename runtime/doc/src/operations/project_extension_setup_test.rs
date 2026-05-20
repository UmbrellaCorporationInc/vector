#![allow(clippy::unwrap_used)]

use runtime_core::{FlowOperation, RuntimeResult, cancel::CancelableSender, channel::Sender};
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use super::{ProjectExtensionSetupInput, ProjectExtensionSetupOp, ProjectExtensionSetupOutput};

struct MockSender {
    output: Option<ProjectExtensionSetupOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { output: None }
    }
}

impl Sender<ProjectExtensionSetupOutput> for MockSender {
    async fn send(&mut self, value: ProjectExtensionSetupOutput) -> RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl CancelableSender<ProjectExtensionSetupOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

fn create_extension_test_project() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  task:
    layout: category
    code-width: 5
    prompt: prompts-00001-create-task
  rfc:
    description: Request for Comments
    tags: [governance]
    layout: status
    code-width: 5
    prompt: prompts-00002-create-rfc
    statuses: [draft]
  template:
    layout: category
    code-width: 5
    prompt: prompts-00003-create-template
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    // locate_file_by_stem searches doc/{doc_type}/ — for stem "template-00006-*" it searches doc/template/
    let template_dir = temp_dir.path().join("doc").join("template").join("ai");
    fs::create_dir_all(&template_dir).unwrap();

    let template_content = "---
id: ai-rule-00003-documentation
title: Documentation Rule
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
---

# Supported Types

#{types}
";
    fs::write(template_dir.join("template-00006-documentation.md"), template_content).unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_project_extension_setup_generates_documentation_rule() {
    let (_temp_dir, root) = create_extension_test_project();

    let input = ProjectExtensionSetupInput { root_dir: root.clone() };
    let mut sender = MockSender::new();

    ProjectExtensionSetupOp.run(input, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    let rule_path = output.documentation_rule_path.as_path();

    assert!(rule_path.exists(), "documentation rule file must be generated");

    let content = fs::read_to_string(rule_path).unwrap();
    assert!(content.contains("**document type:** rfc"));
    assert!(content.contains("**document type:** task"));
    assert!(!content.contains("#{types}"), "placeholder must be replaced");
    assert!(!content.contains("<YYYY-MM-DD>"), "date placeholders must be replaced");
}

#[tokio::test]
async fn test_project_extension_setup_output_path_is_correct() {
    let (_temp_dir, root) = create_extension_test_project();

    let input = ProjectExtensionSetupInput { root_dir: root.clone() };
    let mut sender = MockSender::new();

    ProjectExtensionSetupOp.run(input, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    let path_str = output.documentation_rule_path.as_path().to_string_lossy();
    assert!(
        path_str.ends_with("ai-rule-00003-documentation.md"),
        "output path must point to the documentation rule file"
    );
}
