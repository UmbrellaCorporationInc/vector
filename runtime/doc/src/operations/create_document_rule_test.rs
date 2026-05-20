#![allow(clippy::unwrap_used)]

use runtime_core::FlowOperation;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

use crate::operations::create_document_rule::{
    CreateDocumentRuleInput, CreateDocumentRuleOp, CreateDocumentRuleOutput,
};

struct MockSender {
    output: Option<CreateDocumentRuleOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { output: None }
    }
}

impl runtime_core::Sender<CreateDocumentRuleOutput> for MockSender {
    async fn send(&mut self, value: CreateDocumentRuleOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<CreateDocumentRuleOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

fn create_test_project() -> (TempDir, IoPath) {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = IoPath::new(temp_dir.path());

    let vector_dir = temp_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();

    let config = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    description: Request for Comments
    tags: [governance, tech]
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    statuses: [draft]
  task:
    layout: category
    code-width: 5
    prompt: prompts-00002-create-task
  template:
    layout: category
    code-width: 5
    prompt: prompts-00003-create-template
";
    fs::write(vector_dir.join("document-types.yaml"), config).unwrap();

    let doc_dir = temp_dir.path().join("doc");
    let template_dir = doc_dir.join("template").join("ai");
    fs::create_dir_all(&template_dir).unwrap();

    let template_content = "---
id: rule-00002
title: Documentation Rule
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
---

# Supported Types

#{types}
";
    fs::write(template_dir.join("template-00002-documentation.md"), template_content).unwrap();

    (temp_dir, root)
}

#[tokio::test]
async fn test_create_document_rule_success() {
    let (_temp_dir, root) = create_test_project();
    let output_path =
        root.join("doc").join("ai-rule").join("active").join("ai-rule-00002-documentation.md");

    let input = CreateDocumentRuleInput {
        root_dir: root.clone(),
        output_path: output_path.clone(),
        template_stem: "template-00002-documentation".to_string(),
    };

    let mut sender = MockSender::new();
    CreateDocumentRuleOp.run(input, &mut sender).await.unwrap();

    assert!(output_path.as_path().exists());
    let content = fs::read_to_string(output_path.as_path()).unwrap();

    // Check types expansion (sorted order: rfc, task)
    assert!(content.contains("**document type:** rfc"));
    assert!(content.contains("**tags:** governance, tech"));
    assert!(content.contains("**description:** Request for Comments"));

    assert!(content.contains("**document type:** task"));
    assert!(content.contains("**tags:** -"));
    assert!(content.contains("**description:** -"));

    // Check date replacement
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert!(content.contains(&format!("created: {today}")));
    assert!(content.contains(&format!("updated: {today}")));
    assert!(!content.contains("<YYYY-MM-DD>"));
}

#[tokio::test]
async fn test_create_document_rule_preserves_created_date() {
    let (_temp_dir, root) = create_test_project();
    let output_path =
        root.join("doc").join("ai-rule").join("active").join("ai-rule-00002-documentation.md");

    // Create existing file with a specific created date
    fs::create_dir_all(output_path.as_path().parent().unwrap()).unwrap();
    fs::write(output_path.as_path(), "---\ncreated: 2025-01-01\nupdated: 2025-01-01\n---\n")
        .unwrap();

    let input = CreateDocumentRuleInput {
        root_dir: root.clone(),
        output_path: output_path.clone(),
        template_stem: "template-00002-documentation".to_string(),
    };

    let mut sender = MockSender::new();
    CreateDocumentRuleOp.run(input, &mut sender).await.unwrap();

    let content = fs::read_to_string(output_path.as_path()).unwrap();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    assert!(content.contains("created: 2025-01-01"), "Should preserve existing created date");
    assert!(content.contains(&format!("updated: {today}")), "Should update updated date");
}

#[tokio::test]
async fn test_create_document_rule_missing_placeholder() {
    let (temp_dir, root) = create_test_project();
    let template_dir = temp_dir.path().join("doc").join("template").join("ai");
    fs::write(template_dir.join("template-00003-noplaceholder.md"), "# No placeholder here")
        .unwrap();

    let output_path = root.join("rule.md");
    let input = CreateDocumentRuleInput {
        root_dir: root.clone(),
        output_path: output_path.clone(),
        template_stem: "template-00003-noplaceholder".to_string(),
    };

    let mut sender = MockSender::new();
    CreateDocumentRuleOp.run(input, &mut sender).await.unwrap();

    let content = fs::read_to_string(output_path.as_path()).unwrap();
    assert!(content.contains("# No placeholder here"));
}
