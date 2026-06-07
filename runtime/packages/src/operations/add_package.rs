//! Plugin operation for package manifest insertion.

use crate::types::{PackageEntry, PackageManifest, load_manifest, save_manifest};
use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;

/// Input for the `add_package` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct AddPackageInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
    /// The name of the package to add.
    pub name: String,
    /// The source type ('git' or 'file').
    pub r#type: String,
    /// The location/URL of the package.
    pub url: String,
    /// Optional tag or branch reference.
    pub tag: Option<String>,
}

impl AddPackageInput {
    /// Construct a new `AddPackageInput`.
    #[must_use]
    pub const fn new(
        root_dir: IoPath,
        name: String,
        r#type: String,
        url: String,
        tag: Option<String>,
    ) -> Self {
        Self { root_dir, name, r#type, url, tag }
    }
}

/// Output for the `add_package` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct AddPackageOutput {}

async fn add_package(
    input: AddPackageInput,
    output: &mut impl PluginSender<AddPackageOutput>,
) -> RuntimeResult<()> {
    let path = input.root_dir.join(".vector").join("packages.yaml");
    let mut manifest = if path.as_path().exists() {
        load_manifest(&input.root_dir).await?
    } else {
        PackageManifest::default()
    };

    if input.name.trim().is_empty() {
        return Err(RuntimeError::operation("package name cannot be empty"));
    }

    if manifest.packages.contains_key(&input.name) {
        return Err(RuntimeError::operation(format!(
            "package '{}' is already present in manifest",
            input.name
        )));
    }

    if input.r#type != "git" && input.r#type != "file" {
        return Err(RuntimeError::operation(format!(
            "package '{}' has unsupported source type '{}'",
            input.name, input.r#type
        )));
    }

    if input.url.trim().is_empty() {
        return Err(RuntimeError::operation(format!(
            "package '{}' has invalid url format; url must be a string",
            input.name
        )));
    }

    if input.r#type == "git" {
        let tag = input.tag.as_deref().ok_or_else(|| {
            RuntimeError::operation(format!(
                "package '{}' has invalid or missing tag; tag is required for git packages",
                input.name
            ))
        })?;
        if tag.trim().is_empty() {
            return Err(RuntimeError::operation(format!(
                "package '{}' has invalid or missing tag; tag is required for git packages",
                input.name
            )));
        }
        if tag.starts_with("branch:") {
            let branch_name = tag.strip_prefix("branch:").unwrap_or("").trim();
            if branch_name.is_empty() {
                return Err(RuntimeError::operation(format!(
                    "package '{}' has invalid branch format in tag '{}'",
                    input.name, tag
                )));
            }
        }
    }

    manifest.packages.insert(
        input.name.clone(),
        PackageEntry {
            r#type: input.r#type.clone(),
            url: input.url.clone(),
            tag: input.tag.clone(),
        },
    );

    save_manifest(&input.root_dir, &manifest).await?;

    output.send(AddPackageOutput::default()).await?;
    Ok(())
}

declare_plugin_operations! {
    /// Operation for adding a package to the manifest.
    AddPackageOp => add_package(AddPackageInput, AddPackageOutput)
}

impl AddPackageOp {
    /// Construct a new `AddPackageOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for AddPackageOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "add_package_test.rs"]
mod tests;
