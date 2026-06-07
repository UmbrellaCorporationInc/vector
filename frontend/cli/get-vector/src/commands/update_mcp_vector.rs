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

/// The CLI package name passed to `cargo install`.
const CLI_PACKAGE_NAME: &str = "vector-database";

/// Outcome produced by [`run`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateOutcome {
    /// `mcp-vector` and `vector-database` were installed or updated from git HEAD.
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

/// Runs `cargo install --git <REPO_URL> --force <PACKAGE_NAME>` for both packages,
/// streaming their stdout and stderr to the provided callbacks as they run.
///
/// # Errors
///
/// Returns [`UpdateError::Spawn`] when the `cargo` process cannot be started,
/// [`UpdateError::Wait`] when waiting for it fails, or
/// [`UpdateError::InstallFailed`] when it exits with a non-zero status.
#[allow(clippy::print_stderr)]
pub async fn run<E, Out, Err>(
    executor: &E,
    mut on_stdout: Out,
    mut on_stderr: Err,
) -> Result<UpdateOutcome, UpdateError>
where
    E: CommandExecutor + Sync,
    Out: FnMut(&[u8]) + Send,
    Err: FnMut(&[u8]) + Send,
{
    let width_str = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(80)
        .to_string();

    // 1. Install mcp-vector
    eprintln!("+--------------------------------------------------+");
    eprintln!("|               Updating mcp-vector                |");
    eprintln!("+--------------------------------------------------+");
    let spec1 = CommandBuilder::new("cargo")
        .arg("install")
        .arg("--git")
        .arg(REPO_URL)
        .arg("--force")
        .arg(PACKAGE_NAME)
        .arg("--color=always")
        .env("CARGO_TERM_PROGRESS_WHEN", "always")
        .env("CARGO_TERM_PROGRESS_WIDTH", &width_str)
        .build()
        .map_err(|e| UpdateError::Spawn(e.to_string()))?;

    let mut handle1 = executor.spawn(spec1).await?;
    handle1.stream_output(&mut on_stdout, &mut on_stderr).await;
    let exit1 = handle1.wait().await.map_err(|e| UpdateError::Wait(e.to_string()))?;

    if !exit1.success {
        return Err(UpdateError::InstallFailed { code: exit1.code });
    }

    // 2. Install vector-database
    eprintln!();
    eprintln!("+--------------------------------------------------+");
    eprintln!("|            Updating vector-database              |");
    eprintln!("+--------------------------------------------------+");
    let spec2 = CommandBuilder::new("cargo")
        .arg("install")
        .arg("--git")
        .arg(REPO_URL)
        .arg("--force")
        .arg(CLI_PACKAGE_NAME)
        .arg("--color=always")
        .env("CARGO_TERM_PROGRESS_WHEN", "always")
        .env("CARGO_TERM_PROGRESS_WIDTH", &width_str)
        .build()
        .map_err(|e| UpdateError::Spawn(e.to_string()))?;

    let mut handle2 = executor.spawn(spec2).await?;
    handle2.stream_output(&mut on_stdout, &mut on_stderr).await;
    let exit2 = handle2.wait().await.map_err(|e| UpdateError::Wait(e.to_string()))?;

    if !exit2.success {
        return Err(UpdateError::InstallFailed { code: exit2.code });
    }

    Ok(UpdateOutcome::Installed)
}

#[cfg(test)]
#[path = "update_mcp_vector_test.rs"]
mod tests;
