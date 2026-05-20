//! Document types configuration loader.
//!
//! This module provides functionality for loading document-type configurations
//! from `.vector/document-types.yaml` files.

use runtime_core::{RuntimeError, RuntimeResult};
use runtime_io::path::IoPath;
use runtime_io::read_file_text;
use std::path::Path;

use crate::internal::vector_yaml::validate_vector_yaml_schema_content;
use crate::types::DocumentTypesConfig;

/// Load and deserialize the document types configuration from a project root.
///
/// # Errors
/// Returns `RuntimeError::Operation` if the file is missing, unreadable, or malformed.
pub async fn load_document_types_config(root_dir: &IoPath) -> RuntimeResult<DocumentTypesConfig> {
    let config_path = root_dir.join(".vector").join("document-types.yaml");

    let content = read_file_text(&config_path).await.map_err(|error| {
        RuntimeError::operation(format!("failed to read .vector/document-types.yaml: {error}"))
    })?;

    if let Err(field_errors) =
        validate_vector_yaml_schema_content(".vector/document-types.yaml", &content)
    {
        let error = field_errors
            .into_iter()
            .map(|field_error| field_error.message())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RuntimeError::operation(error));
    }

    let config: DocumentTypesConfig = serde_yaml::from_str(&content).map_err(|error| {
        RuntimeError::operation(format!("failed to parse .vector/document-types.yaml: {error}"))
    })?;

    Ok(config)
}

/// Load document types config from an explicit path.
///
/// # Errors
/// Returns `RuntimeError::Operation` if the file is missing, unreadable, or malformed.
pub async fn load_from_path<P: AsRef<Path>>(path: P) -> RuntimeResult<DocumentTypesConfig> {
    let io_path = IoPath::new(path.as_ref());
    let content = runtime_io::read_file_text(&io_path).await.map_err(|error| {
        RuntimeError::operation(format!("failed to read document-types config: {error}"))
    })?;

    let display_path = path.as_ref().to_string_lossy().replace('\\', "/");
    if let Err(field_errors) = validate_vector_yaml_schema_content(&display_path, &content) {
        let error = field_errors
            .into_iter()
            .map(|field_error| field_error.message())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(RuntimeError::operation(error));
    }

    let config: DocumentTypesConfig = serde_yaml::from_str(&content).map_err(|error| {
        RuntimeError::operation(format!("failed to parse document-types config: {error}"))
    })?;

    Ok(config)
}

#[cfg(test)]
#[path = "loader_test.rs"]
mod tests;
