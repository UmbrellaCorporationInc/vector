#![allow(clippy::expect_used)]

use rmcp::handler::server::wrapper::Parameters;
use runtime_channel::PluginDispatcher;
use runtime_core::channel::Receiver;
use runtime_doc::operations::{ValidateInput, ValidateOp};
use runtime_io::path::IoPath;

/// Verifies that `ValidateOp` can be dispatched through the standard
/// `PluginDispatcher` path without any MCP types involved.
///
/// Covers the bridge shape: select op → supply input → consume receiver.
/// Does not test MCP protocol encoding or tool registration.
#[tokio::test]
async fn validate_op_dispatches_through_dispatcher_for_missing_config() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = IoPath::new(dir.path());

    let input = ValidateInput::new(root, false);

    let (_cancel, mut receiver) = PluginDispatcher::new(ValidateOp::new())
        .input(input)
        .build()
        .expect("dispatcher build must succeed with a valid input");

    let mut received_count = 0usize;
    while let Ok(Some(output)) = receiver.recv().await {
        assert!(!output.valid, "output must be invalid when document-types.yaml is absent");
        assert!(
            output.errors.iter().any(|e| e.path.contains("document-types.yaml")),
            "errors must reference the missing config file"
        );
        received_count += 1;
    }

    assert_eq!(received_count, 1, "ValidateOp must emit exactly one output value");
}

/// Verifies that `PluginDispatcher::build` returns an error when no input
/// is supplied, enforcing the contract that input is mandatory before dispatch.
#[test]
fn dispatcher_build_requires_input() {
    let result = PluginDispatcher::new(ValidateOp::new()).build();
    assert!(result.is_err(), "build without input must return an error");
}

/// Verifies that `ValidateParams` deserializes correctly and that the
/// `fix` field defaults to `false` — boundary shape test without invoking
/// any MCP protocol machinery.
#[test]
fn validate_params_defaults_fix_to_false() {
    let raw = r#"{"root_dir": "/tmp/project"}"#;
    let params: super::ValidateParams =
        serde_json::from_str(raw).expect("must deserialize without fix field");
    assert!(!params.fix, "fix must default to false when absent from JSON");
}

/// Verifies that `DocumentTools::new` and `Default` both construct a usable
/// MCP adapter instance for the document capability group.
#[test]
fn document_tools_constructors_produce_usable_adapter() {
    let _from_new = super::DocumentTools::new();
    let _from_default = super::DocumentTools::default();
}

/// Verifies that the MCP-facing `validate` tool executes the real `ValidateOp`
/// path and returns a non-empty message for a project without a config file.
///
/// Covers the adapter boundary directly: MCP params → runtime input →
/// dispatcher execution → receiver consumption → MCP-facing string result.
#[tokio::test]
async fn validate_tool_reports_missing_config_as_error_message() {
    let dir = tempfile::tempdir().expect("temp dir");

    let tools = super::DocumentTools::new();
    let result = tools
        .validate(Parameters(super::ValidateParams {
            root_dir: dir.path().display().to_string(),
            fix: false,
        }))
        .await
        .expect("tool execution must succeed even when validation itself fails");

    assert!(
        result.contains("ERROR"),
        "tool result must contain an ERROR line when document-types.yaml is absent"
    );
    assert!(
        result.contains("document-types.yaml"),
        "tool result must reference the missing config file"
    );
}

/// Verifies that `FindDocParams` deserializes correctly from JSON input and that `package` defaults to empty.
#[test]
fn find_doc_params_deserializes_correctly() {
    let raw = r#"{"root_dir": "/tmp/project", "doc_type": "rfc", "code": 13}"#;
    let params: super::FindDocParams =
        serde_json::from_str(raw).expect("must deserialize FindDocParams");
    assert_eq!(params.root_dir, "/tmp/project");
    assert_eq!(params.doc_type, "rfc");
    assert_eq!(params.code, 13);
    assert_eq!(params.package, "", "package must default to empty string when absent from JSON");
}

/// Verifies that `FindDocParams` deserializes correctly when `package` is explicitly provided.
#[test]
fn find_doc_params_accepts_optional_package_field() {
    let raw = r#"{"package": "my-pkg", "root_dir": "/tmp/project", "doc_type": "rfc", "code": 13}"#;
    let params: super::FindDocParams =
        serde_json::from_str(raw).expect("must deserialize FindDocParams with package");
    assert_eq!(params.package, "my-pkg", "package must deserialize when explicitly provided");
    assert_eq!(params.root_dir, "/tmp/project");
    assert_eq!(params.doc_type, "rfc");
    assert_eq!(params.code, 13);
}

const MINIMAL_CONFIG: &str = "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    create-document-form: form-00001\n    statuses: [draft]";

fn create_doc_tool_test_project() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::fs;
    let dir = tempfile::tempdir().expect("temp dir");

    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config =
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    create-document-form: form-00001
    initial-status: draft
    statuses:
      - draft
    prompt: prompts-00001-create-rfc
  prompts:
    template: template-00003-prompts
    layout: category
    code-width: 5
    prompt: prompts-00002-create-prompts
    create-document-form: form-00001
";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let prompt_dir = dir.path().join("doc").join("prompts").join("authoring");
    let template_dir = dir.path().join("doc").join("template").join("project");
    fs::create_dir_all(&prompt_dir).expect("create prompt dir");
    fs::create_dir_all(&template_dir).expect("create template dir");
    fs::create_dir_all(dir.path().join("doc").join("rfc").join("draft")).expect("create rfc dir");

    let prompt_content = "---
id: prompts-00001-create-rfc
type: prompts
code: \"00001\"
slug: create-rfc
title: Create RFC Prompt
description: Prompt for creating RFC documents.
category: authoring
created: 2026-01-01
updated: 2026-01-01
tags: []
---

Type: #{doc-type}
Code: #{code}
Slug: #{slug}
Path: #{file-path}
";
    fs::write(prompt_dir.join("prompts-00001-create-rfc.md"), prompt_content)
        .expect("write prompt");

    let template_content = "---
id: rfc-00001-sample
type: rfc
code: \"00001\"
slug: sample
title: Sample
description: Sample RFC template.
created: 2026-01-01
updated: 2026-01-01
tags: []
related: []
---

# <Title>
";
    fs::write(template_dir.join("template-00001-rfc.md"), template_content)
        .expect("write template");

    let root = dir.path().to_path_buf();
    (dir, root)
}

fn create_doc_tool_test_project_without_prompt() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::fs;
    let dir = tempfile::tempdir().expect("temp dir");

    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config =
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}
document-types:
  spec:
    template: template-00002-spec
    layout: category
    code-width: 5
    create-document-form: form-00001
  prompts:
    template: template-00003-prompts
    layout: category
    code-width: 5
    prompt: prompts-00002-create-doc
    create-document-form: form-00001
";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let prompt_dir = dir.path().join("doc").join("prompts").join("authoring");
    let template_dir = dir.path().join("doc").join("template").join("project");
    fs::create_dir_all(&prompt_dir).expect("create prompt dir");
    fs::create_dir_all(&template_dir).expect("create template dir");
    fs::create_dir_all(dir.path().join("doc").join("spec").join("notes")).expect("create spec dir");

    let prompt_content = "---
id: prompts-00002-create-doc
type: prompts
code: \"00002\"
slug: create-doc
title: Create Document
description: Default prompt for creating documents.
category: authoring
created: 2026-01-01
updated: 2026-01-01
tags: []
---

Type: #{doc-type}
Code: #{code}
Slug: #{slug}
Path: #{file-path}
";
    fs::write(prompt_dir.join("prompts-00002-create-doc.md"), prompt_content)
        .expect("write prompt");

    let template_content = "---
id: spec-00001-sample
type: spec
code: \"00001\"
slug: sample
title: Sample
description: Sample spec template.
created: 2026-01-01
updated: 2026-01-01
tags: []
related: []
---

# <Title>
";
    fs::write(template_dir.join("template-00002-spec.md"), template_content)
        .expect("write template");

    let root = dir.path().to_path_buf();
    (dir, root)
}

/// Verifies that the `find_doc` tool returns path, empty package, and document content when the document exists.
#[tokio::test]
async fn find_doc_tool_returns_absolute_path_for_existing_document() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(vector_dir.join("document-types.yaml"), MINIMAL_CONFIG).expect("write config");

    let rfc_dir = dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).expect("create rfc dir");
    let target = rfc_dir.join("rfc-00013-my-rfc.md");
    fs::write(&target, "doc content here").expect("write doc");

    let tools = super::DocumentTools::new();
    let result = tools
        .find_doc(Parameters(super::FindDocParams {
            package: String::new(),
            root_dir: dir.path().display().to_string(),
            doc_type: "rfc".to_string(),
            code: 13,
        }))
        .await
        .expect("find_doc must succeed when the document exists");

    let expected_path =
        dunce::canonicalize(&target).expect("canonicalize").to_string_lossy().to_string();
    let response: super::FindDocResponse =
        serde_json::from_str(&result).expect("must deserialize FindDocResponse");
    assert_eq!(response.path, expected_path);
    assert_eq!(response.package, "");
    assert_eq!(response.content, "doc content here");
}

/// Verifies that the `find_doc` tool returns the correct package field in its output.
#[tokio::test]
async fn find_doc_tool_returns_package_in_output() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let package_dir = dir.path().join(".vector-database").join("packages").join("my-package");
    let vector_dir = package_dir.join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(vector_dir.join("document-types.yaml"), MINIMAL_CONFIG).expect("write config");

    let rfc_dir = package_dir.join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).expect("create rfc dir");
    fs::write(rfc_dir.join("rfc-00013-my-rfc.md"), "body").expect("write doc");

    let tools = super::DocumentTools::new();
    let result = tools
        .find_doc(Parameters(super::FindDocParams {
            package: "my-package".to_string(),
            root_dir: dir.path().display().to_string(),
            doc_type: "rfc".to_string(),
            code: 13,
        }))
        .await
        .expect("find_doc must succeed when the document exists");

    let response: super::FindDocResponse =
        serde_json::from_str(&result).expect("must deserialize FindDocResponse");
    assert_eq!(response.package, "my-package");
}

/// Verifies that the `find_doc` tool includes populated document content in its response.
#[tokio::test]
async fn find_doc_tool_returns_populated_content() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(vector_dir.join("document-types.yaml"), MINIMAL_CONFIG).expect("write config");

    let rfc_dir = dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).expect("create rfc dir");
    let expected_content = "# My RFC\n\nThis is the body of the document.\n";
    fs::write(rfc_dir.join("rfc-00013-my-rfc.md"), expected_content).expect("write doc");

    let tools = super::DocumentTools::new();
    let result = tools
        .find_doc(Parameters(super::FindDocParams {
            package: String::new(),
            root_dir: dir.path().display().to_string(),
            doc_type: "rfc".to_string(),
            code: 13,
        }))
        .await
        .expect("find_doc must succeed when the document exists");

    let response: super::FindDocResponse =
        serde_json::from_str(&result).expect("must deserialize FindDocResponse");
    assert_eq!(response.content, expected_content);
}

/// Verifies that the `find_doc` tool returns an error when no matching document exists.
#[tokio::test]
async fn find_doc_tool_returns_error_when_document_not_found() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(vector_dir.join("document-types.yaml"), MINIMAL_CONFIG).expect("write config");

    let rfc_dir = dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).expect("create rfc dir");

    let tools = super::DocumentTools::new();
    let result = tools
        .find_doc(Parameters(super::FindDocParams {
            package: String::new(),
            root_dir: dir.path().display().to_string(),
            doc_type: "rfc".to_string(),
            code: 99,
        }))
        .await;

    assert!(result.is_err(), "find_doc must return an error when no document matches the code");
    let err = result.expect_err("must be an error");
    assert!(
        !err.is_empty(),
        "error message must not be empty when document is not found; got: {err:?}"
    );
}

/// Verifies that the `find_doc` tool returns an error for an unknown document type.
#[tokio::test]
async fn find_doc_tool_returns_error_for_unknown_doc_type() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");
    fs::write(
        vector_dir.join("document-types.yaml"),
        "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}\ndocument-types: {}",
    )
    .expect("write config");

    let tools = super::DocumentTools::new();
    let result = tools
        .find_doc(Parameters(super::FindDocParams {
            package: String::new(),
            root_dir: dir.path().display().to_string(),
            doc_type: "unknown".to_string(),
            code: 1,
        }))
        .await;

    assert!(result.is_err(), "find_doc must return an error for an unregistered document type");
}

/// Verifies that `CreateDocParams` deserializes correctly without the optional `category` field.
#[test]
fn create_doc_params_deserializes_without_category() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "rfc", "name": "My RFC", "slug": "my-rfc"}"#;
    let params: super::CreateDocParams =
        serde_json::from_str(raw).expect("must deserialize without category");
    assert_eq!(params.root_dir, "/tmp/p");
    assert_eq!(params.doc_type, "rfc");
    assert_eq!(params.name, "My RFC");
    assert_eq!(params.slug, "my-rfc");
    assert!(params.category.is_none(), "category must be None when absent");
}

/// Verifies that `CreateDocParams` deserializes correctly when `category` is present.
#[test]
fn create_doc_params_deserializes_with_category() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "rfc", "name": "My RFC", "slug": "my-rfc", "category": "draft"}"#;
    let params: super::CreateDocParams =
        serde_json::from_str(raw).expect("must deserialize with category");
    assert_eq!(params.category.as_deref(), Some("draft"));
}

/// Verifies that the `create_doc` tool creates a document and returns path, code, and resolved prompt.
#[tokio::test]
async fn create_doc_tool_returns_path_code_and_prompt_for_valid_input() {
    let (_dir, root) = create_doc_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_prompt(Parameters(super::CreateDocParams {
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            name: "My New RFC".to_string(),
            slug: "my-new-rfc".to_string(),
            category: None,
        }))
        .await
        .expect("create_doc tool must succeed for valid input");

    assert!(result.contains("path:"), "result must contain path line; got: {result}");
    assert!(result.contains("code:"), "result must contain code line; got: {result}");
    assert!(result.contains("Type: rfc"), "result must contain resolved doc_type; got: {result}");
    assert!(result.contains("my-new-rfc"), "result must reference the slug; got: {result}");
}

/// Verifies that the `create_doc` tool returns an error for an invalid slug.
#[tokio::test]
async fn create_doc_tool_returns_error_for_invalid_slug() {
    let (_dir, root) = create_doc_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_prompt(Parameters(super::CreateDocParams {
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            name: "Bad Slug".to_string(),
            slug: "Invalid Slug With Spaces".to_string(),
            category: None,
        }))
        .await;

    assert!(result.is_err(), "create_doc tool must return an error for an invalid slug");
}

/// Verifies that the `create_doc` tool returns an error for an unknown document type.
#[tokio::test]
async fn create_doc_tool_returns_error_for_unknown_doc_type() {
    let (_dir, root) = create_doc_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_prompt(Parameters(super::CreateDocParams {
            root_dir: root.display().to_string(),
            doc_type: "nonexistent".to_string(),
            name: "Test".to_string(),
            slug: "test-slug".to_string(),
            category: None,
        }))
        .await;

    assert!(result.is_err(), "create_doc tool must return an error for an unregistered doc type");
}

/// Verifies that the `create_doc` tool falls back to the default prompt when the
/// document type configuration omits its explicit prompt field.
#[tokio::test]
async fn create_doc_tool_uses_default_prompt_when_type_prompt_is_missing() {
    let (_dir, root) = create_doc_tool_test_project_without_prompt();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_prompt(Parameters(super::CreateDocParams {
            root_dir: root.display().to_string(),
            doc_type: "spec".to_string(),
            name: "My Spec".to_string(),
            slug: "my-spec".to_string(),
            category: Some("notes".to_string()),
        }))
        .await
        .expect("create_doc tool must fall back to the default prompt");

    assert!(result.contains("path:"), "result must contain path line; got: {result}");
    assert!(result.contains("code: 00001"), "result must contain code line; got: {result}");
    assert!(result.contains("Type: spec"), "result must contain resolved doc_type; got: {result}");
    assert!(result.contains("my-spec"), "result must reference the slug; got: {result}");
}

/// Verifies that the `create_doc` tool surfaces the specific `RuntimeError` message
/// from the failed operation through the channel (Phase F behavior).
#[tokio::test]
async fn create_doc_tool_error_message_carries_runtime_cause() {
    let (_dir, root) = create_doc_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_prompt(Parameters(super::CreateDocParams {
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            name: "Bad".to_string(),
            slug: "INVALID SLUG".to_string(),
            category: None,
        }))
        .await;

    let err = result.expect_err("must be an error for an invalid slug");
    assert!(!err.is_empty(), "error message must not be empty; got: {err:?}");
    assert!(
        err.contains("create_doc failed:"),
        "error must contain the operation prefix; got: {err:?}"
    );
}

/// Builds a temporary project wired for `create_doc_type_prompt`.
///
/// Mirrors the layout used by `create_doc_type_test.rs`: a config with a `doc_type`
/// meta-section pointing to a prompt document, plus the prompt file on disk.
fn create_doc_type_tool_test_project() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::fs;
    let dir = tempfile::tempdir().expect("temp dir");

    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: prompts-00002-create-rfc
    create-document-form: form-00001
    initial-status: draft
    statuses:
      - draft
  prompts:
    layout: category
    code-width: 5
    prompt: prompts-00003-create-prompts
    create-document-form: form-00001
  template:
    layout: category
    code-width: 5
    prompt: prompts-00005-create-template
    create-document-form: form-00001
";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let prompts_dir = dir.path().join("doc").join("prompts").join("doc-type");
    fs::create_dir_all(&prompts_dir).expect("create prompts dir");

    let prompt_content = "---
id: prompts-00001-create-doc-type
type: prompts
code: \"00001\"
slug: create-doc-type
title: Create Document Type
description: Governed prompt for creating a new document type.
category: doc-type
created: 2026-01-01
updated: 2026-01-01
tags: []
---

You are creating a new document type: `#{doc-type}`

Layout: `#{layout}`
";
    fs::write(prompts_dir.join("prompts-00001-create-doc-type.md"), prompt_content)
        .expect("write prompt");

    let template_dir = dir.path().join("doc").join("template").join("project");
    let ai_template_dir = dir.path().join("doc").join("template").join("ai");
    fs::create_dir_all(&template_dir).expect("create template dir");
    fs::create_dir_all(&ai_template_dir).expect("create ai template dir");
    fs::write(
        ai_template_dir.join("template-00006-documentation.md"),
        "---\ncreated: <YYYY-MM-DD>\nupdated: <YYYY-MM-DD>\n---\n\n#{types}\n",
    )
    .expect("write doc template");
    fs::write(
        template_dir.join("template-00004-doc-type-template.md"),
        "---\nid: doc-type-00001-<slug>\ntype: doc-type\ncode: \"00001\"\nslug: <slug>\ntitle: <Title>\n---\n",
    )
    .expect("write doc-type template");
    fs::write(
        template_dir.join("template-00005-doc-type-prompt.md"),
        "---\nid: doc-type-prompt-00001-<slug>\ntype: doc-type-prompt\ncode: \"00001\"\nslug: <slug>\ntitle: <Title>\n---\n",
    )
    .expect("write doc-type-prompt template");

    let root = dir.path().to_path_buf();
    (dir, root)
}

/// Verifies that `CreateDocTypeParams` deserializes correctly with required fields only.
#[test]
fn create_doc_type_params_deserializes_required_fields() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "adr", "layout": "status", "code-width": 5, "statuses": ["draft"]}"#;
    let params: super::CreateDocTypeParams =
        serde_json::from_str(raw).expect("must deserialize with required fields");
    assert_eq!(params.root_dir, "/tmp/p");
    assert_eq!(params.doc_type, "adr");
    assert_eq!(params.layout, "status");
    assert_eq!(params.code_width, 5);
    assert_eq!(params.statuses.as_deref(), Some(["draft".to_string()].as_slice()));
    assert!(params.description.is_none());
    assert!(params.tags.is_none());
    assert!(params.template.is_none());
}

/// Verifies that optional fields on `CreateDocTypeParams` round-trip correctly.
#[test]
fn create_doc_type_params_deserializes_optional_fields() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "spec", "layout": "category", "code-width": 5, "description": "A spec type", "tags": ["core"], "template": "template-spec"}"#;
    let params: super::CreateDocTypeParams =
        serde_json::from_str(raw).expect("must deserialize with optional fields");
    assert_eq!(params.description.as_deref(), Some("A spec type"));
    assert_eq!(params.tags.as_deref(), Some(["core".to_string()].as_slice()));
    assert_eq!(params.template.as_deref(), Some("template-spec"));
}

/// Verifies that the `create_doc_type` tool creates the type and returns `doc_type`, layout, and prompt.
#[tokio::test]
async fn create_doc_type_tool_returns_doc_type_layout_and_prompt_for_valid_input() {
    let (_dir, root) = create_doc_type_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_type_prompt(rmcp::handler::server::wrapper::Parameters(
            super::CreateDocTypeParams {
                root_dir: root.display().to_string(),
                doc_type: "task".to_string(),
                layout: "status".to_string(),
                code_width: 5,
                statuses: Some(vec!["todo".to_string(), "done".to_string()]),
                description: None,
                tags: None,
                template: None,
            },
        ))
        .await
        .expect("create_doc_type tool must succeed for valid input");

    assert!(result.contains("doc-type:"), "result must contain doc-type line; got: {result}");
    assert!(result.contains("layout:"), "result must contain layout line; got: {result}");
    assert!(result.contains("task"), "result must reference the doc type; got: {result}");
    assert!(result.contains("status"), "result must reference the layout; got: {result}");
}

/// Verifies that the `create_doc_type` tool returns an error for an invalid doc type name.
#[tokio::test]
async fn create_doc_type_tool_returns_error_for_invalid_doc_type_name() {
    let (_dir, root) = create_doc_type_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_type_prompt(rmcp::handler::server::wrapper::Parameters(
            super::CreateDocTypeParams {
                root_dir: root.display().to_string(),
                doc_type: "Invalid Type".to_string(),
                layout: "status".to_string(),
                code_width: 5,
                statuses: Some(vec!["draft".to_string()]),
                description: None,
                tags: None,
                template: None,
            },
        ))
        .await;

    assert!(
        result.is_err(),
        "create_doc_type tool must return an error for an invalid doc type name"
    );
    let err = result.expect_err("must be an error");
    assert!(
        err.contains("create_doc_type failed:"),
        "error must carry the operation prefix; got: {err:?}"
    );
}

/// Verifies that the `create_doc_type` tool surfaces the runtime error when the prompt document is absent.
#[tokio::test]
async fn create_doc_type_tool_returns_error_when_prompt_document_is_missing() {
    use std::fs;
    let (dir, root) = create_doc_type_tool_test_project();
    fs::remove_file(
        dir.path()
            .join("doc")
            .join("prompts")
            .join("doc-type")
            .join("prompts-00001-create-doc-type.md"),
    )
    .expect("remove prompt file");

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_type_prompt(rmcp::handler::server::wrapper::Parameters(
            super::CreateDocTypeParams {
                root_dir: root.display().to_string(),
                doc_type: "design".to_string(),
                layout: "category".to_string(),
                code_width: 5,
                statuses: None,
                description: None,
                tags: None,
                template: None,
            },
        ))
        .await;

    assert!(
        result.is_err(),
        "create_doc_type tool must return an error when the prompt document is missing"
    );
}

/// Verifies that the `validate` tool reports `Validation passed` for a
/// project with a valid config and no governed documents.
#[tokio::test]
async fn validate_tool_reports_pass_for_empty_valid_project() {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config = "doc-type:
  template: t
  prompt-template: pt
  prompt: p
  create-document-type-form: f
document-types: {}
";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let tools = super::DocumentTools::new();
    let result = tools
        .validate(Parameters(super::ValidateParams {
            root_dir: dir.path().display().to_string(),
            fix: false,
        }))
        .await
        .expect("tool execution must succeed for a valid empty project");

    assert!(
        result.contains("Validation passed"),
        "tool result must report success for a project with no documents and a valid config; got: {result}"
    );
}

/// Builds a minimal project with a single RFC document containing a
/// wikilink with a `.md` extension, which is auto-correctable.
fn create_validate_fix_test_project() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::fs;

    let dir = tempfile::tempdir().expect("temp dir");
    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config = "doc-type:
  template: t
  prompt-template: pt
  prompt: p
  create-document-type-form: f
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    create-document-form: form-00001
    initial-status: draft
    statuses:
      - draft
";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let draft_dir = dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&draft_dir).expect("create draft dir");

    let doc_content = "---
_id: rfc-00001-test
_type: rfc
_code: \"00001\"
_slug: test
_title: Test
_description: Test RFC
_created: 2026-01-01
_status: draft
---

# Test RFC

See [[other-doc.md]] for details.
";
    fs::write(draft_dir.join("rfc-00001-test.md"), doc_content).expect("write doc");

    let root = dir.path().to_path_buf();
    (dir, root)
}

/// Verifies that `validate_fix` applies auto-fixes and reports them.
#[tokio::test]
async fn validate_fix_tool_applies_auto_fixes() {
    let (_dir, root) = create_validate_fix_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .validate_fix(Parameters(super::ValidateFixParams { root_dir: root.display().to_string() }))
        .await
        .expect("validate_fix tool must succeed");

    assert!(result.contains("FIXED:"), "validate_fix must report applied fixes; got: {result}");
}

/// Verifies that `validate` with `fix: false` does not apply or report fixes.
#[tokio::test]
async fn validate_tool_does_not_apply_fixes_when_fix_is_false() {
    let (_dir, root) = create_validate_fix_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .validate(Parameters(super::ValidateParams {
            root_dir: root.display().to_string(),
            fix: false,
        }))
        .await
        .expect("validate tool must succeed");

    assert!(
        !result.contains("FIXED:"),
        "validate with fix: false must not report fixes; got: {result}"
    );
}

#[tokio::test]
async fn test_create_doc_type_directory_based() {
    let (temp_dir, _root) = create_doc_type_tool_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .create_doc_type_prompt(Parameters(super::CreateDocTypeParams {
            root_dir: temp_dir.path().to_string_lossy().to_string(),
            doc_type: "research".to_string(),
            layout: "directory".to_string(),
            code_width: 5,
            statuses: None,
            description: Some("Research papers".to_string()),
            tags: Some(vec!["science".to_string()]),
            template: None,
        }))
        .await
        .expect("create_doc_type tool must succeed");

    assert!(result.contains("doc-type: research"));
    assert!(result.contains("layout: directory"));
    assert!(result.contains("Layout: `directory`"));

    let config_path = temp_dir.path().join(".vector").join("document-types.yaml");
    let config_content = std::fs::read_to_string(config_path).expect("read config");
    assert!(config_content.contains("research:"));
    assert!(config_content.contains("layout: directory"));
}

// ── patch_doc tool helpers ────────────────────────────────────────────────────

fn create_patch_doc_test_project() -> (tempfile::TempDir, std::path::PathBuf) {
    use std::fs;
    let dir = tempfile::tempdir().expect("temp dir");

    let vector_dir = dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).expect("create .vector dir");

    let config = "doc-type: {template: t, prompt-template: pt, prompt: p, create-document-type-form: f}\ndocument-types:\n  rfc:\n    layout: status\n    code-width: 5\n    prompt: prompts-00001-create-rfc\n    create-document-form: form-00001\n    statuses: [draft]";
    fs::write(vector_dir.join("document-types.yaml"), config).expect("write config");

    let rfc_dir = dir.path().join("doc").join("rfc").join("draft");
    fs::create_dir_all(&rfc_dir).expect("create rfc dir");

    let doc_content = "Original line.\n";
    fs::write(rfc_dir.join("rfc-00042-my-rfc.md"), doc_content).expect("write doc");

    let root = dir.path().to_path_buf();
    (dir, root)
}

/// Verifies that `PatchDocParams` deserializes correctly from JSON input.
#[test]
fn patch_doc_params_deserializes_correctly() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "rfc", "code": 42, "format": "unified", "patch": "--- a/x.md\n+++ b/x.md\n"}"#;
    let params: super::PatchDocParams =
        serde_json::from_str(raw).expect("must deserialize PatchDocParams");
    assert_eq!(params.package, "", "package must default to empty string");
    assert_eq!(params.root_dir, "/tmp/p");
    assert_eq!(params.doc_type, "rfc");
    assert_eq!(params.code, 42);
    assert_eq!(params.format.as_deref(), Some("unified"));
    assert!(params.patch.as_ref().is_some_and(|patch| !patch.is_empty()));
    assert!(params.git_diff.is_none());
}

/// Verifies that deprecated `git_diff` payloads remain accepted as a transition alias.
#[test]
fn patch_doc_params_deserializes_deprecated_git_diff_alias() {
    let raw = r#"{"root_dir": "/tmp/p", "doc_type": "rfc", "code": 42, "git_diff": "--- a/x.md\n+++ b/x.md\n"}"#;
    let params: super::PatchDocParams =
        serde_json::from_str(raw).expect("must deserialize PatchDocParams with git_diff");
    assert!(params.patch.is_none());
    assert!(params.format.is_none());
    assert!(params.git_diff.as_ref().is_some_and(|git_diff| !git_diff.is_empty()));
}

#[test]
fn patch_doc_input_from_params_defaults_omitted_format_to_apply_patch() {
    let input = super::patch_doc_input_from_params(super::PatchDocParams {
        package: String::new(),
        root_dir: "/tmp/p".to_string(),
        doc_type: "rfc".to_string(),
        code: 42,
        format: None,
        patch: Some("*** Begin Patch\n*** End Patch\n".to_string()),
        git_diff: None,
    })
    .expect("omitted format must default successfully");

    assert_eq!(input.format, runtime_doc::operations::PatchDocFormat::ApplyPatch);
}

#[test]
fn patch_doc_input_from_params_rejects_unknown_format() {
    let err = super::patch_doc_input_from_params(super::PatchDocParams {
        package: String::new(),
        root_dir: "/tmp/p".to_string(),
        doc_type: "rfc".to_string(),
        code: 42,
        format: Some("context".to_string()),
        patch: Some("patch".to_string()),
        git_diff: None,
    })
    .expect_err("unknown format must be rejected");

    assert!(err.contains("unknown patch format 'context'"), "{err}");
    assert!(err.contains("Supported values: unified, apply_patch"), "{err}");
}

#[test]
fn patch_doc_input_from_params_maps_git_diff_alias_to_unified() {
    let input = super::patch_doc_input_from_params(super::PatchDocParams {
        package: "pkg".to_string(),
        root_dir: "/tmp/p".to_string(),
        doc_type: "rfc".to_string(),
        code: 42,
        format: None,
        patch: None,
        git_diff: Some("--- a/x.md\n+++ b/x.md\n".to_string()),
    })
    .expect("git_diff alias must map to unified format");

    assert_eq!(input.package, "pkg");
    assert_eq!(input.format, runtime_doc::operations::PatchDocFormat::Unified);
    assert_eq!(input.patch, "--- a/x.md\n+++ b/x.md\n");
}

/// Verifies that the `patch_doc` tool applies a valid unified diff and returns the patched content.
#[tokio::test]
async fn patch_doc_tool_applies_valid_diff_and_returns_content() {
    let (dir, root) = create_patch_doc_test_project();
    let rfc_path = dir.path().join("doc").join("rfc").join("draft").join("rfc-00042-my-rfc.md");

    let git_diff =
        "--- a/rfc-00042-my-rfc.md\n+++ b/rfc-00042-my-rfc.md\n@@ -1,1 +1,1 @@\n-Original line.\n+Updated line.\n".to_string();

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 42,
            format: Some("unified".to_string()),
            patch: Some(git_diff),
            git_diff: None,
        }))
        .await
        .expect("patch_doc must succeed for a valid diff");

    let expected_path =
        dunce::canonicalize(&rfc_path).expect("canonicalize").to_string_lossy().to_string();
    assert!(
        result.contains(&format!("path: {expected_path}")),
        "result must contain the canonicalized path; got: {result}"
    );
    assert!(
        result.contains("Updated line."),
        "result must contain the patched content; got: {result}"
    );
    assert!(
        !result.contains("Original line."),
        "result must not contain the original content; got: {result}"
    );
}

/// Verifies that the `patch_doc` tool returns an error when the document does not exist.
#[tokio::test]
async fn patch_doc_tool_returns_error_for_missing_document() {
    let (_dir, root) = create_patch_doc_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 999,
            format: None,
            patch: None,
            git_diff: Some(
                "--- a/rfc-00999-x.md\n+++ b/rfc-00999-x.md\n@@ -1 +1 @@\n-old\n+new\n".to_string(),
            ),
        }))
        .await;

    assert!(result.is_err(), "patch_doc must return an error when the document does not exist");
    let err = result.expect_err("must be an error");
    assert!(
        err.contains("patch_doc failed:"),
        "error must carry the operation prefix; got: {err:?}"
    );
}

/// Verifies that the `patch_doc` tool returns an error for a malformed diff.
#[tokio::test]
async fn patch_doc_tool_returns_error_for_malformed_diff() {
    let (_dir, root) = create_patch_doc_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 42,
            format: None,
            patch: None,
            git_diff: Some("this is not a valid unified diff".to_string()),
        }))
        .await;

    assert!(result.is_err(), "patch_doc must return an error for a malformed diff");
    let err = result.expect_err("must be an error");
    assert!(
        err.contains("patch_doc failed:"),
        "error must carry the operation prefix; got: {err:?}"
    );
}

/// Verifies that malformed hunk counts surface the runtime diagnostic through the MCP prefix.
#[tokio::test]
async fn patch_doc_tool_returns_actionable_error_for_hunk_count_mismatch() {
    let (dir, root) = create_patch_doc_test_project();
    let rfc_path = dir.path().join("doc").join("rfc").join("draft").join("rfc-00042-my-rfc.md");
    let original_content = std::fs::read_to_string(&rfc_path).expect("read original doc");

    let git_diff = "\
--- a/src/main.rs
+++ b/src/main.rs
@@ -37,10 +37,10 @@ input:
-old one
-old two
-old three
-old four
-old five
-old six
-old seven
+new one
+new two
+new three
+new four
+new five
+new six
+new seven
"
    .to_string();

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 42,
            format: Some("unified".to_string()),
            patch: Some(git_diff),
            git_diff: None,
        }))
        .await;

    assert!(result.is_err(), "patch_doc must reject malformed hunk line counts");
    let err = result.expect_err("must be an error");
    assert!(
        err.starts_with("patch_doc failed:"),
        "error must keep the MCP operation prefix; got: {err:?}"
    );
    assert!(
        err.contains("hunk line-count mismatch"),
        "error must identify the malformed hunk counts; got: {err:?}"
    );
    assert!(
        err.contains("Make the @@ -a,b +c,d @@ counts match"),
        "error must explain how to repair the hunk header; got: {err:?}"
    );
    assert!(
        err.contains("Preflight detail:"),
        "error must include preflight detail for debugging; got: {err:?}"
    );
    assert!(
        err.contains("Header expected (-10, +10)")
            && err.contains("Parsed content counts (-7, +7)"),
        "error must expose declared and parsed hunk counts; got: {err:?}"
    );
    assert!(
        !err.contains("patch targets"),
        "hunk count mismatch must fail during preflight before target mismatch checks; got: {err:?}"
    );
    assert!(
        err.len() <= 600,
        "error should stay compact enough for agent feedback loops; length={} err={err:?}",
        err.len()
    );

    let on_disk = std::fs::read_to_string(&rfc_path).expect("read doc after rejected patch");
    assert_eq!(
        on_disk, original_content,
        "malformed diff rejection must not modify the target document"
    );
}

/// Verifies that the `patch_doc` tool returns an error for a diff that targets a different file.
#[tokio::test]
async fn patch_doc_tool_returns_error_for_target_mismatch() {
    let (_dir, root) = create_patch_doc_test_project();

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 42,
            format: None,
            patch: None,
            git_diff: Some(
                "--- a/rfc-00001-other.md\n+++ b/rfc-00001-other.md\n@@ -1 +1 @@\n-old\n+new\n"
                    .to_string(),
            ),
        }))
        .await;

    assert!(
        result.is_err(),
        "patch_doc must return an error when the diff targets a different file"
    );
    let err = result.expect_err("must be an error");
    assert!(
        err.contains("patch_doc failed:"),
        "error must carry the operation prefix; got: {err:?}"
    );
}

/// Verifies that the `patch_doc` tool returns an error when the diff would produce BOM content.
#[tokio::test]
async fn patch_doc_tool_returns_error_for_bom_content() {
    let (_dir, root) = create_patch_doc_test_project();

    // Introduce BOM via the replacement line so the resulting content starts with BOM.
    let bom_line = "\u{FEFF}Updated line.";
    let git_diff = format!(
        "--- a/rfc-00042-my-rfc.md\n+++ b/rfc-00042-my-rfc.md\n@@ -1,1 +1,1 @@\n-Original line.\n+{bom_line}\n"
    );

    let tools = super::DocumentTools::new();
    let result = tools
        .patch_doc(Parameters(super::PatchDocParams {
            package: String::new(),
            root_dir: root.display().to_string(),
            doc_type: "rfc".to_string(),
            code: 42,
            format: Some("unified".to_string()),
            patch: Some(git_diff),
            git_diff: None,
        }))
        .await;

    assert!(result.is_err(), "patch_doc must return an error when resulting content has a BOM");
    let err = result.expect_err("must be an error");
    assert!(
        err.contains("patch_doc failed:"),
        "error must carry the operation prefix; got: {err:?}"
    );
    assert!(err.contains("BOM"), "error must mention BOM to guide remediation; got: {err:?}");
}
