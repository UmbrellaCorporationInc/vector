use super::{is_kebab_case_identifier, validate_vector_yaml_schema_content};

#[test]
fn kebab_case_identifier_rejects_underscores() {
    assert!(is_kebab_case_identifier("quality-gate"));
    assert!(!is_kebab_case_identifier("quality_gate"));
    assert!(!is_kebab_case_identifier("Quality-gate"));
}

#[test]
fn validation_preserves_dynamic_document_type_keys() {
    let yaml = "document-types:\n  rfc:\n    code-width: 5\n";
    assert!(validate_vector_yaml_schema_content(".vector/document-types.yaml", yaml).is_ok());
}

#[test]
fn validation_rejects_invalid_dashboard_field_name() {
    let yaml = "sections:\n  todo:\n    doc_type: task\n";
    let result =
        validate_vector_yaml_schema_content(".vector/dashboards/project-status.yaml", yaml);
    assert!(result.is_err(), "dashboard schema field should be rejected");
    let errors = result.err().unwrap_or_default();
    assert_eq!(errors[0].field_name(), "doc_type");
    assert_eq!(errors[0].field_path(), "sections.*.doc_type");
}
