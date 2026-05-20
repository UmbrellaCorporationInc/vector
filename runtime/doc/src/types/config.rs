//! Document types configuration model.
//!
//! This module defines the types for loading and validating document-type
//! configurations from `.vector/document-types.yaml`.

use serde::Deserialize;

/// Root structure matching `.vector/document-types.yaml` schema.
///
/// # DTO(Plugin operation input/output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DocumentTypesConfig {
    /// Configuration for the doc-type bootstrap process itself.
    pub doc_type: DocTypeBootstrapConfig,
    /// Map of document type names to their configurations.
    pub document_types: std::collections::HashMap<String, DocumentTypeConfig>,
}

/// Configuration for the doc-type bootstrap process.
///
/// # DTO(Plugin operation input/output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DocTypeBootstrapConfig {
    /// Template for the new document type.
    pub template: String,
    /// Template for the prompt associated with the new document type.
    pub prompt_template: String,
    /// The prompt used to create the document type.
    pub prompt: String,
    /// The form used to create a new document type.
    #[serde(default)]
    pub create_document_type_form: String,
}

/// Configuration for a single document type.
///
/// # DTO(Plugin operation input/output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct DocumentTypeConfig {
    /// Human-readable description of this document type.
    pub description: Option<String>,
    /// Template name used for this document type.
    pub template: Option<String>,
    /// Layout strategy: "status", "category", or "directory".
    pub layout: Layout,
    /// Width of the numeric code portion (e.g., 5 for "00001").
    pub code_width: u8,
    /// Initial status for status-based types.
    #[serde(default)]
    pub initial_status: Option<String>,
    /// Allowed statuses for status-based document types.
    #[serde(default)]
    pub statuses: Vec<String>,
    /// Optional tags describing this document type.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Name of the authoring prompt file (without extension) for this document type.
    ///
    /// When absent, document authoring falls back to the project default prompt.
    #[serde(default)]
    pub prompt: String,
    /// The form used to create a new document of this type.
    #[serde(default)]
    pub create_document_form: String,
}

/// Supported layout strategies for governed documents.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Grouped by workflow status (e.g., `doc/rfc/draft/`).
    Status,
    /// Grouped by topic category (e.g., `doc/spec/api/`).
    Category,
    /// Flat directory (e.g., `doc/research/`).
    Directory,
}

impl DocumentTypeConfig {
    /// Returns true if this is a status-based document type.
    #[must_use]
    pub fn is_status_based(&self) -> bool {
        self.layout == Layout::Status
    }

    /// Returns true if this is a category-based document type.
    #[must_use]
    pub fn is_category_based(&self) -> bool {
        self.layout == Layout::Category
    }

    /// Returns true if this is a directory-based document type.
    #[must_use]
    pub fn is_directory_based(&self) -> bool {
        self.layout == Layout::Directory
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
