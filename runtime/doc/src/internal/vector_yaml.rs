//! Validation helpers for `.vector/*.yaml` schema field names.

use std::path::Path;

/// A single `.vector` YAML schema validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorYamlFieldError {
    /// Relative path to the YAML file.
    path: String,
    /// Offending field name.
    field_name: String,
    /// Dot-separated location of the field within the YAML document.
    field_path: String,
}

impl VectorYamlFieldError {
    /// Construct one schema validation error.
    #[must_use]
    pub const fn new(path: String, field_name: String, field_path: String) -> Self {
        Self { path, field_name, field_path }
    }

    /// Return the relative path to the YAML file.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Return the offending field name.
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Return the dot-separated location of the field within the YAML document.
    #[must_use]
    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    /// Render a stable validation message.
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "Invalid .vector YAML field name '{}' at '{}'; schema fields must be kebab-case",
            self.field_name, self.field_path
        )
    }
}

/// Validate `.vector` YAML schema field names for one file.
///
/// # Errors
/// Returns field-name validation failures.
pub fn validate_vector_yaml_schema_content(
    relative_path: &str,
    content: &str,
) -> Result<(), Vec<VectorYamlFieldError>> {
    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(content) else {
        return Ok(());
    };

    let mut errors = Vec::new();
    visit_value(relative_path, &yaml, &mut Vec::new(), &mut errors);
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Returns true when a schema field name matches the kebab-case contract.
#[must_use]
pub fn is_kebab_case_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }

    let mut previous_was_hyphen = false;
    for ch in chars {
        if ch == '-' {
            if previous_was_hyphen {
                return false;
            }
            previous_was_hyphen = true;
            continue;
        }
        previous_was_hyphen = false;
        if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() {
            return false;
        }
    }

    !name.ends_with('-')
}

/// Convert a path to a slash-normalized relative display string.
#[must_use]
pub fn relative_display_path(root_dir: &Path, path: &Path) -> String {
    path.strip_prefix(root_dir).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn visit_value(
    relative_path: &str,
    value: &serde_yaml::Value,
    current_path: &mut Vec<String>,
    errors: &mut Vec<VectorYamlFieldError>,
) {
    let serde_yaml::Value::Mapping(mapping) = value else {
        return;
    };

    let dynamic_children = has_dynamic_children(relative_path, current_path);
    for (key, child) in mapping {
        let serde_yaml::Value::String(field_name) = key else {
            continue;
        };

        if dynamic_children {
            current_path.push("*".to_string());
            visit_value(relative_path, child, current_path, errors);
            current_path.pop();
            continue;
        }

        if !is_kebab_case_identifier(field_name) {
            let mut field_path = current_path.clone();
            field_path.push(field_name.clone());
            errors.push(VectorYamlFieldError::new(
                relative_path.to_string(),
                field_name.clone(),
                field_path.join("."),
            ));
        }

        current_path.push(field_name.clone());
        visit_value(relative_path, child, current_path, errors);
        current_path.pop();
    }
}

fn has_dynamic_children(relative_path: &str, current_path: &[String]) -> bool {
    let matches_path = |expected: &[&str]| {
        current_path.len() == expected.len()
            && current_path.iter().map(std::string::String::as_str).eq(expected.iter().copied())
    };

    match relative_path {
        ".vector/document-types.yaml" => matches_path(&["document-types"]),
        ".vector/language-rules.yaml" => current_path.is_empty(),
        ".vector/agents.yaml" => matches_path(&["agents"]) || matches_path(&["profiles"]),
        ".vector/dashboards/project-status.yaml" => matches_path(&["sections"]),
        _ => false,
    }
}

#[cfg(test)]
#[path = "vector_yaml_test.rs"]
mod tests;
