//! Document types configuration model.
//!
//! This module defines the types for loading and validating document-type
//! configurations from `.vector/document-types.yaml`.

mod config;
mod loader;

pub use config::{DocumentTypeConfig, DocumentTypesConfig};
pub use loader::{load_document_types_config, load_from_path};
