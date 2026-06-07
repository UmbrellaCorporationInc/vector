//! Package manifest definition, parsing, and validation.

use runtime_core::{RuntimeError, RuntimeResult};
use runtime_io::{
    path::IoPath,
    text::{read_file_text, write_file_text},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Error representing a package manifest validation or parsing failure.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
#[non_exhaustive]
pub enum ManifestError {
    /// The document is not a YAML mapping.
    #[error("manifest is not a valid YAML mapping")]
    NotAMap,
    /// An entry in the manifest is not a mapping.
    #[error("package '{0}' is not a mapping")]
    EntryNotAMap(String),
    /// The entry is missing the required 'type' field.
    #[error("package '{0}' is missing required field 'type'")]
    MissingType(String),
    /// The entry is missing the required 'url' field.
    #[error("package '{0}' is missing required field 'url'")]
    MissingUrl(String),
    /// The entry has an unsupported package source type.
    #[error("package '{0}' has unsupported source type '{1}'")]
    UnsupportedType(String, String),
    /// Git package is missing the required tag field.
    #[error("package '{0}' has invalid or missing tag; tag is required for git packages")]
    MissingTagForGit(String),
    /// The tag field has an invalid format (not a string).
    #[error("package '{0}' has invalid tag format; tag must be a string")]
    InvalidTagFormat(String),
    /// The url field has an invalid format (not a string).
    #[error("package '{0}' has invalid url format; url must be a string")]
    InvalidUrlFormat(String),
    /// The type field has an invalid format (not a string).
    #[error("package '{0}' has invalid type format; type must be a string")]
    InvalidTypeFormat(String),
    /// The branch name tracking HEAD is invalid/empty.
    #[error("package '{0}' has invalid branch format in tag '{1}'")]
    InvalidBranchFormat(String, String),
    /// Any other YAML parsing error.
    #[error("failed to parse YAML: {0}")]
    YamlParse(String),
}

/// A single package entry in the manifest.
///
/// # DTO(Package manifest entries use public fields for direct deserialization and access)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    /// The source type, currently 'git' or 'file'.
    pub r#type: String,
    /// The location/URL of the package.
    pub url: String,
    /// Optional tag or branch reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// The collection of packages defined in the manifest.
///
/// # DTO(Package manifest wrapper uses public fields for direct deserialization and access)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PackageManifest {
    /// Map of package name to its configuration.
    #[serde(flatten)]
    pub packages: BTreeMap<String, PackageEntry>,
}

impl PackageManifest {
    /// Parse and validate a package manifest from YAML text.
    ///
    /// # Errors
    ///
    /// Returns a [`ManifestError`] if the YAML is malformed or violates validation rules.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let yaml_val: serde_yaml::Value =
            serde_yaml::from_str(text).map_err(|e| ManifestError::YamlParse(e.to_string()))?;

        let mapping = yaml_val.as_mapping().ok_or(ManifestError::NotAMap)?;

        let mut packages = BTreeMap::new();

        for (key_val, val_val) in mapping {
            let key = key_val
                .as_str()
                .ok_or_else(|| {
                    ManifestError::YamlParse("Package name must be a string".to_string())
                })?
                .to_string();

            if !val_val.is_mapping() {
                return Err(ManifestError::EntryNotAMap(key));
            }

            // 1. Validate 'type'
            let type_val =
                val_val.get("type").ok_or_else(|| ManifestError::MissingType(key.clone()))?;
            let type_str =
                type_val.as_str().ok_or_else(|| ManifestError::InvalidTypeFormat(key.clone()))?;

            if type_str != "git" && type_str != "file" {
                return Err(ManifestError::UnsupportedType(key.clone(), type_str.to_string()));
            }

            // 2. Validate 'url'
            let url_val =
                val_val.get("url").ok_or_else(|| ManifestError::MissingUrl(key.clone()))?;
            let url_str =
                url_val.as_str().ok_or_else(|| ManifestError::InvalidUrlFormat(key.clone()))?;

            // 3. Validate 'tag'
            let tag_val = val_val.get("tag");
            let tag_opt = match tag_val {
                Some(v) => {
                    if v.is_null() {
                        None
                    } else {
                        let t_str = v
                            .as_str()
                            .ok_or_else(|| ManifestError::InvalidTagFormat(key.clone()))?;
                        Some(t_str.to_string())
                    }
                }
                None => None,
            };

            // Enforce tag rules:
            // - tag is required for git packages
            if type_str == "git" {
                let tag = tag_opt
                    .as_deref()
                    .ok_or_else(|| ManifestError::MissingTagForGit(key.clone()))?;
                // Accept tag: branch:main and tag: branch:<name>
                if tag.starts_with("branch:") {
                    let branch_name = tag.strip_prefix("branch:").unwrap_or("").trim();
                    if branch_name.is_empty() {
                        return Err(ManifestError::InvalidBranchFormat(
                            key.clone(),
                            tag.to_string(),
                        ));
                    }
                }
            }

            packages.insert(
                key,
                PackageEntry {
                    r#type: type_str.to_string(),
                    url: url_str.to_string(),
                    tag: tag_opt,
                },
            );
        }

        Ok(Self { packages })
    }

    /// Serialize the manifest into YAML format.
    ///
    /// # Errors
    ///
    /// Returns a [`ManifestError`] if serialization fails.
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        serde_yaml::to_string(self).map_err(|e| ManifestError::YamlParse(e.to_string()))
    }
}

/// Load the package manifest from the default `.vector/packages.yaml` location within the root directory.
///
/// # Errors
///
/// Returns a [`RuntimeError`] if reading or parsing the manifest fails.
pub async fn load_manifest(root_dir: &IoPath) -> RuntimeResult<PackageManifest> {
    let path = root_dir.join(".vector").join("packages.yaml");
    if !path.as_path().exists() {
        return Ok(PackageManifest::default());
    }
    let text = read_file_text(&path).await.map_err(|error| {
        RuntimeError::operation(format!("failed to read .vector/packages.yaml: {error}"))
    })?;
    PackageManifest::parse(&text).map_err(|error| {
        RuntimeError::operation(format!("failed to parse .vector/packages.yaml: {error}"))
    })
}

/// Save the package manifest to `.vector/packages.yaml` under the project root.
///
/// # Errors
///
/// Returns a [`RuntimeError`] if serialization or writing fails.
pub async fn save_manifest(root_dir: &IoPath, manifest: &PackageManifest) -> RuntimeResult<()> {
    let path = root_dir.join(".vector").join("packages.yaml");
    let yaml = manifest.to_yaml().map_err(|error| {
        RuntimeError::operation(format!("failed to serialize manifest: {error}"))
    })?;
    write_file_text(&path, yaml).await.map_err(|error| {
        RuntimeError::operation(format!("failed to write .vector/packages.yaml: {error}"))
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "manifest_test.rs"]
mod tests;
