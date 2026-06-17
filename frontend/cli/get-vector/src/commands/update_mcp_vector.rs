//! Command: install or update local Vector binaries.
//!
//! V1 always runs `cargo install --git --force` from the repository HEAD.
//! Version-aware reconciliation (skip when already current) is deferred until
//! a runtime strategy for resolving the latest published version is defined.

use runtime_io::{CommandBuilder, CommandExecutor, IoError};

/// The `cargo install --git` URL for the vector repository.
const REPO_URL: &str = "https://github.com/UmbrellaCorporationInc/vector";

/// The MCP binary package name passed to `cargo install`.
const MCP_PACKAGE_NAME: &str = "mcp-vector";

/// The CLI package name passed to `cargo install`.
const CLI_PACKAGE_NAME: &str = "vector-database";

/// The RAG companion CLI package name passed to `cargo install`.
const RAG_PACKAGE_NAME: &str = "vector-rag";

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

/// Runs `cargo install --git <REPO_URL> --force` for the base packages,
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
    on_stdout: Out,
    on_stderr: Err,
) -> Result<UpdateOutcome, UpdateError>
where
    E: CommandExecutor + Sync,
    Out: FnMut(&[u8]) + Send,
    Err: FnMut(&[u8]) + Send,
{
    run_packages(executor, &[MCP_PACKAGE_NAME, CLI_PACKAGE_NAME], on_stdout, on_stderr).await
}

/// Runs `cargo install --git <REPO_URL> --force vector-rag`, streaming stdout and
/// stderr to the provided callbacks as the install runs.
///
/// # Errors
///
/// Returns [`UpdateError::Spawn`] when the `cargo` process cannot be started,
/// [`UpdateError::Wait`] when waiting for it fails, or
/// [`UpdateError::InstallFailed`] when it exits with a non-zero status.
#[allow(clippy::print_stderr)]
pub async fn run_rag<E, Out, Err>(
    executor: &E,
    on_stdout: Out,
    on_stderr: Err,
) -> Result<UpdateOutcome, UpdateError>
where
    E: CommandExecutor + Sync,
    Out: FnMut(&[u8]) + Send,
    Err: FnMut(&[u8]) + Send,
{
    run_packages(executor, &[RAG_PACKAGE_NAME], on_stdout, on_stderr).await
}

#[allow(clippy::print_stderr)]
async fn run_packages<E, Out, Err>(
    executor: &E,
    packages: &[&str],
    mut on_stdout: Out,
    mut on_stderr: Err,
) -> Result<UpdateOutcome, UpdateError>
where
    E: CommandExecutor + Sync,
    Out: FnMut(&[u8]) + Send,
    Err: FnMut(&[u8]) + Send,
{
    let width_str = terminal_size::terminal_size()
        .map_or(80, |(terminal_size::Width(w), _)| usize::from(w))
        .to_string();

    for (index, package) in packages.iter().enumerate() {
        if index > 0 {
            eprintln!();
        }
        eprintln!("+--------------------------------------------------+");
        eprintln!("| {:^48} |", format!("Updating {package}"));
        eprintln!("+--------------------------------------------------+");

        let spec = CommandBuilder::new("cargo")
            .arg("install")
            .arg("--git")
            .arg(REPO_URL)
            .arg("--force")
            .arg(*package)
            .arg("--color=always")
            .env("CARGO_TERM_PROGRESS_WHEN", "always")
            .env("CARGO_TERM_PROGRESS_WIDTH", &width_str)
            .build()
            .map_err(|e| UpdateError::Spawn(e.to_string()))?;

        let mut handle = executor.spawn(spec).await?;
        handle.stream_output(&mut on_stdout, &mut on_stderr).await;
        let exit = handle.wait().await.map_err(|e| UpdateError::Wait(e.to_string()))?;

        if !exit.success {
            return Err(UpdateError::InstallFailed { code: exit.code });
        }
    }

    Ok(UpdateOutcome::Installed)
}

#[cfg(test)]
#[path = "update_mcp_vector_test.rs"]
mod tests;
