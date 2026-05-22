//! Command: install or update the local `mcp-vector` binary.
//!
//! V1 always runs `cargo install --git --force` from the repository HEAD.
//! Version-aware reconciliation (skip when already current) is deferred until
//! a runtime strategy for resolving the latest published version is defined.

use runtime_io::{CommandBuilder, CommandExecutor, IoError};

/// The `cargo install --git` URL for the vector repository.
const REPO_URL: &str = "https://github.com/UmbrellaCorporationInc/vector";

/// The binary package name passed to `cargo install`.
const PACKAGE_NAME: &str = "mcp-vector";

/// Outcome produced by [`run`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateOutcome {
    /// `mcp-vector` was installed or updated from git HEAD.
    Installed,
}

/// Errors produced by the update command.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum UpdateError {
    /// Failed to spawn the `cargo install` process.
    #[error("failed to spawn cargo install: {0}")]
    Spawn(String),
    /// `cargo install` exited with a non-zero status code.
    #[error("cargo install failed with exit code {code:?}")]
    InstallFailed {
        /// The exit code, if available.
        code: Option<i32>,
    },
    /// Failed to wait for the `cargo install` process.
    #[error("failed to wait for cargo install: {0}")]
    Wait(String),
}

impl From<IoError> for UpdateError {
    fn from(error: IoError) -> Self {
        Self::Spawn(error.to_string())
    }
}

/// Runs `cargo install --git <REPO_URL> --force <PACKAGE_NAME>`.
///
/// Always performs a full install from git HEAD. No version comparison is
/// done in V1 — this avoids the chicken-and-egg problem where a
/// compile-time version constant would cause an outdated CLI binary to
/// incorrectly skip reinstallation after a workspace version bump.
///
/// # Errors
///
/// Returns [`UpdateError::Spawn`] when the `cargo` process cannot be started,
/// [`UpdateError::Wait`] when waiting for it fails, or
/// [`UpdateError::InstallFailed`] when it exits with a non-zero status.
// The future is only ever driven on a single-threaded CLI runtime; `E` is not
// required to be `Sync` because the reference is never sent across threads.
#[allow(clippy::future_not_send)]
pub async fn run<E: CommandExecutor>(executor: &E) -> Result<UpdateOutcome, UpdateError> {
    let spec = CommandBuilder::new("cargo")
        .arg("install")
        .arg("--git")
        .arg(REPO_URL)
        .arg("--force")
        .arg(PACKAGE_NAME)
        .build()
        .map_err(|e| UpdateError::Spawn(e.to_string()))?;

    let handle = executor.spawn(spec).await?;
    let exit = handle.wait().await.map_err(|e| UpdateError::Wait(e.to_string()))?;

    if exit.success {
        Ok(UpdateOutcome::Installed)
    } else {
        Err(UpdateError::InstallFailed { code: exit.code })
    }
}

#[cfg(test)]
#[path = "update_mcp_vector_test.rs"]
mod tests;
