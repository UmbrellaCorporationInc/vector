//! Plugin operation for package synchronization planning.

use crate::types::load_manifest;
use runtime_core::{RuntimeError, RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use serde::{Deserialize, Serialize};

/// The type of command to execute to synchronize a package.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncCommandType {
    /// Clone a Git repository.
    Clone,
    /// Fetch updates for an existing Git repository.
    Fetch,
    /// Copy local files.
    Copy,
}

impl SyncCommandType {
    /// Get the string representation of the command type.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Clone => "clone",
            Self::Fetch => "fetch",
            Self::Copy => "copy",
        }
    }
}

impl std::fmt::Display for SyncCommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single planned sync action for a package.
///
/// # DTO(Sync actions use public fields for direct deserialization and access)
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAction {
    /// The name of the package.
    pub name: String,
    /// The command to execute.
    pub command_type: SyncCommandType,
    /// Agent-facing description of the operation.
    pub description: String,
}

/// Input for the `sync_packages` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SyncPackagesInput {
    /// The root directory of the project.
    pub root_dir: IoPath,
}

impl SyncPackagesInput {
    /// Construct a new `SyncPackagesInput`.
    #[must_use]
    pub const fn new(root_dir: IoPath) -> Self {
        Self { root_dir }
    }
}

/// Output for the `sync_packages` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SyncPackagesOutput {
    /// The list of planned sync actions.
    pub actions: Vec<SyncAction>,
}

async fn sync_packages(
    input: SyncPackagesInput,
    output: &mut impl PluginSender<SyncPackagesOutput>,
) -> RuntimeResult<()> {
    let manifest = load_manifest(&input.root_dir).await?;
    let mut actions = Vec::new();
    let packages_dir = input.root_dir.join(".vector-database").join("packages");

    for (name, entry) in &manifest.packages {
        let package_path = packages_dir.join(name);
        let exists = package_path.as_path().exists();

        let (command_type, description) = match entry.r#type.as_str() {
            "git" => {
                let target = entry.tag.as_ref().map_or("main", |tag_val| {
                    tag_val.strip_prefix("branch:").map_or(tag_val.as_str(), |branch| branch.trim())
                });

                if exists {
                    (
                        SyncCommandType::Fetch,
                        format!(
                            "git fetch and update the package in .vector-database/packages/{name}"
                        ),
                    )
                } else {
                    (
                        SyncCommandType::Clone,
                        format!(
                            "clone the Git source and switch to {target} in .vector-database/packages/{name}"
                        ),
                    )
                }
            }
            "file" => (
                SyncCommandType::Copy,
                format!("copy data from the file source into .vector-database/packages/{name}"),
            ),
            _ => {
                return Err(RuntimeError::operation(format!(
                    "unsupported package source type: {}",
                    entry.r#type
                )));
            }
        };

        actions.push(SyncAction { name: name.clone(), command_type, description });
    }

    output.send(SyncPackagesOutput { actions }).await?;
    Ok(())
}

declare_plugin_operations! {
    /// Operation for package synchronization planning.
    SyncPackagesOp => sync_packages(SyncPackagesInput, SyncPackagesOutput)
}

impl SyncPackagesOp {
    /// Construct a new `SyncPackagesOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for SyncPackagesOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "sync_packages_test.rs"]
mod tests;
