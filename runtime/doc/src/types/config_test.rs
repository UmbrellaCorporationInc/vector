#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Tests for `DocumentTypesConfig` deserialization.

const VALID_CONFIG_YAML: &str = "doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
document-types:
  rfc:
    template: template-00001-rfc
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    initial-status: draft
    statuses:
      - draft
      - review
      - accepted
  spec:
    template: template-00003-spec
    layout: category
    code-width: 5
    prompt: prompts-00002-create-spec
";

const MINIMAL_CONFIG_YAML: &str = "doc-type:
  template: t
  prompt-template: pt
  prompt: p
  create-document-type-form: f
document-types:
  task:
    layout: status
    code-width: 5
    prompt: prompts-00003-create-task
    statuses:
      - todo
      - done
";

const CONFIG_WITH_TAGS_YAML: &str = "doc-type:
  template: t
  prompt-template: pt
  prompt: p
  create-document-type-form: f
document-types:
  adr:
    layout: category
    code-width: 4
    prompt: prompts-00004-create-adr
    tags:
      - architecture
      - governance
";

#[test]
fn test_deserialize_valid_config() {
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(VALID_CONFIG_YAML).expect("valid yaml should parse");
    assert_eq!(config.document_types.len(), 2);
    assert_eq!(config.doc_type.prompt, "prompts-00001-create-doc-type");
    assert_eq!(config.doc_type.create_document_type_form, "form-00002-create-document-type");

    let rfc = config.document_types.get("rfc").expect("rfc type should exist");
    assert!(rfc.is_status_based());
    assert!(!rfc.is_category_based());
    assert_eq!(rfc.statuses, vec!["draft", "review", "accepted"]);
    assert_eq!(rfc.code_width, 5);
}

#[test]
fn test_deserialize_minimal_config() {
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(MINIMAL_CONFIG_YAML).expect("valid yaml should parse");
    assert_eq!(config.doc_type.template, "t");
    let task = config.document_types.get("task").expect("task type should exist");
    assert!(task.is_status_based());
    assert!(task.template.is_none());
    assert_eq!(task.prompt, "prompts-00003-create-task");
}

#[test]
fn test_category_based_document_type() {
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(VALID_CONFIG_YAML).expect("valid yaml should parse");
    let spec = config.document_types.get("spec").expect("spec type should exist");
    assert!(!spec.is_status_based());
    assert!(spec.is_category_based());
    assert!(!spec.is_directory_based());
}

#[test]
fn test_directory_based_document_type() {
    let yaml = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  research:
    layout: directory
    code-width: 5
";
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(yaml).expect("valid yaml should parse");
    let research = config.document_types.get("research").expect("research type should exist");
    assert!(!research.is_status_based());
    assert!(!research.is_category_based());
    assert!(research.is_directory_based());
}

#[test]
fn test_deserialize_optional_tags() {
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(CONFIG_WITH_TAGS_YAML).expect("valid yaml should parse");
    let adr = config.document_types.get("adr").expect("adr type should exist");
    assert_eq!(adr.tags, Some(vec!["architecture".to_string(), "governance".to_string()]));
    assert_eq!(adr.prompt, "prompts-00004-create-adr");
}

#[test]
fn test_rejects_unknown_fields() {
    let yaml = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    prompt: prompts-00001-create-rfc
    unknown_field: value
";
    let result: Result<crate::types::DocumentTypesConfig, _> = serde_yaml::from_str(yaml);
    assert!(result.is_err());
}

#[test]
fn test_defaults_missing_document_type_prompt_to_empty() {
    let yaml = "doc-type: {template: t, prompt-template: pt, prompt: p}
document-types:
  rfc:
    layout: status
    code-width: 5
    statuses:
      - draft
";
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(yaml).expect("missing prompt should default");
    assert_eq!(config.document_types["rfc"].prompt, "");
}

#[test]
fn test_defaults_missing_doc_type_create_form_to_empty() {
    let yaml = "doc-type:
  template: t
  prompt-template: pt
  prompt: p
document-types:
  rfc:
    layout: status
    code-width: 5
    statuses:
      - draft
";
    let config: crate::types::DocumentTypesConfig =
        serde_yaml::from_str(yaml).expect("missing create-document-type-form should default");
    assert_eq!(config.doc_type.create_document_type_form, "");
}
