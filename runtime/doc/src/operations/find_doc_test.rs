#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use runtime_io::path::IoPath;
use std::fs;
use tempfile::TempDir;

struct MockSender {
    outputs: Vec<FindDocOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::Sender<FindDocOutput> for MockSender {
    async fn send(&mut self, value: FindDocOutput) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<FindDocOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

fn write_runtime_doc_config(temp: &TempDir, doc_type: &str, body: &str) {
    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        format!("doc-type: {{template: t, prompt-template: pt, prompt: p}}\ndocument-types:\n  {doc_type}:\n{body}"),
    )
    .unwrap();
}

#[tokio::test]
async fn test_find_doc_success() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_runtime_doc_config(
        &temp,
        "rfc",
        "    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    );

    // Setup doc folder
    let rfc_dir = temp.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).unwrap();
    let target_file = rfc_dir.join("rfc-00013-my-rfc.md");
    fs::write(&target_file, "content").unwrap();

    let input = FindDocInput {
        root_dir: root,
        package: String::new(),
        doc_type: "rfc".to_string(),
        code: 13,
    };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_ok());
    assert_eq!(sender.outputs.len(), 1);

    let expected_path = dunce::canonicalize(target_file).unwrap().to_string_lossy().to_string();
    assert_eq!(sender.outputs[0].path, expected_path);
    assert_eq!(sender.outputs[0].package, "");
    assert_eq!(sender.outputs[0].content, "content");
}

#[tokio::test]
async fn test_find_doc_not_found() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_runtime_doc_config(
        &temp,
        "rfc",
        "    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    );

    let input = FindDocInput {
        root_dir: root,
        package: String::new(),
        doc_type: "rfc".to_string(),
        code: 99,
    };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_doc_invalid_type() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    let vector_dir = temp.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types: {}",
    )
    .unwrap();

    let input = FindDocInput {
        root_dir: root,
        package: String::new(),
        doc_type: "unknown".to_string(),
        code: 1,
    };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_doc_directory_based() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    write_runtime_doc_config(&temp, "research", "    layout: directory\n    code-width: 5");

    // Create a research document directly under doc/research/
    let research_dir = temp.path().join("doc").join("research");
    fs::create_dir_all(&research_dir).unwrap();
    let doc_path = research_dir.join("research-00001-study.md");
    fs::write(&doc_path, "# Study").unwrap();

    let input = FindDocInput {
        root_dir: root,
        package: String::new(),
        doc_type: "research".to_string(),
        code: 1,
    };

    let mut sender = MockSender::new();
    find_doc(input, &mut sender).await.unwrap();

    let output = sender.outputs.first().expect("Output should be sent");
    let expected_path = dunce::canonicalize(doc_path).unwrap().to_string_lossy().to_string();
    assert_eq!(output.path, expected_path);
    assert_eq!(output.package, "");
    assert_eq!(output.content, "# Study");
}

#[tokio::test]
async fn test_find_doc_with_package() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Create the package directory inside .vector-database/packages/my-package
    let pkg_dir = temp.path().join(".vector-database").join("packages").join("my-package");
    fs::create_dir_all(pkg_dir.join(".vector")).unwrap();
    fs::create_dir_all(pkg_dir.join("doc").join("rfc").join("draft")).unwrap();

    // Write the document types config inside the package
    fs::write(
        pkg_dir.join(".vector").join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    statuses: [draft]",
    )
    .unwrap();

    let target_file = pkg_dir.join("doc").join("rfc").join("draft").join("rfc-00013-my-rfc.md");
    fs::write(&target_file, "package doc content").unwrap();

    let input = FindDocInput {
        root_dir: root.clone(),
        package: "my-package".to_string(),
        doc_type: "rfc".to_string(),
        code: 13,
    };

    let mut sender = MockSender::new();
    find_doc(input, &mut sender).await.unwrap();

    let output = sender.outputs.first().expect("Output should be sent");
    let expected_path = dunce::canonicalize(target_file).unwrap().to_string_lossy().to_string();
    assert_eq!(output.path, expected_path);
    assert_eq!(output.package, "my-package");
    assert_eq!(output.content, "package doc content");
}

#[tokio::test]
async fn test_find_doc_with_package_not_synchronized() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Do not create the package directory at all — simulates an unsynchronized package.
    let input = FindDocInput {
        root_dir: root.clone(),
        package: "nonexistent-package".to_string(),
        doc_type: "rfc".to_string(),
        code: 13,
    };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("is not synchronized or does not exist"),
        "error must describe an unsynchronized package; got: {err_msg}"
    );
}

#[tokio::test]
async fn test_find_doc_with_package_invalid_contract() {
    let temp = TempDir::new().unwrap();
    let root = IoPath::new(temp.path());

    // Create a package directory that lacks "doc" or ".vector"
    let pkg_dir = temp.path().join(".vector-database").join("packages").join("bad-package");
    fs::create_dir_all(&pkg_dir).unwrap(); // Empty folder, violates contract

    let input = FindDocInput {
        root_dir: root.clone(),
        package: "bad-package".to_string(),
        doc_type: "rfc".to_string(),
        code: 13,
    };

    let mut sender = MockSender::new();
    let result = find_doc(input, &mut sender).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("does not satisfy the minimum repository contract"));
}
