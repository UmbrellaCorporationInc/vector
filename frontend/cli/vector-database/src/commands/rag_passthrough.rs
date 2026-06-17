//! Passthrough command boundary for the `vector-rag` companion CLI.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_io::{CommandBuilder, CommandExecutor};
use std::io::Write;

const VECTOR_RAG_BINARY: &str = "vector-rag";
const INSTALL_GUIDANCE: &str = "vector-rag is not available on PATH. \
Install RAG support with `get-vector install rag` and try again.";

/// Result of delegating a RAG command to the companion CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DelegatedExit {
    /// The companion command exited successfully.
    Success,
    /// The companion command exited with a non-zero or signal-only status.
    Failure(Option<i32>),
}

impl DelegatedExit {
    /// Returns the process exit code that `vector-database` should use.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Failure(Some(code)) => code,
            Self::Failure(None) => 1,
        }
    }
}

/// Delegate `vector-database rag ...` to `vector-rag rag ...`.
///
/// # Errors
///
/// Returns an actionable installation error when `vector-rag` cannot be
/// spawned, when the command specification cannot be built, or when waiting for
/// the companion process fails.
pub async fn run<E>(
    executor: &E,
    root_dir: &std::path::Path,
    args: &[String],
) -> Result<DelegatedExit, String>
where
    E: CommandExecutor + Sync,
{
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();

    let mut on_stdout = |bytes: &[u8]| {
        let _ = stdout.write_all(bytes);
        let _ = stdout.flush();
    };
    let mut on_stderr = |bytes: &[u8]| {
        let _ = stderr.write_all(bytes);
        let _ = stderr.flush();
    };

    run_with_output(executor, root_dir, args, &mut on_stdout, &mut on_stderr).await
}

async fn run_with_output<E, F, G>(
    executor: &E,
    root_dir: &std::path::Path,
    args: &[String],
    on_stdout: &mut F,
    on_stderr: &mut G,
) -> Result<DelegatedExit, String>
where
    E: CommandExecutor + Sync,
    F: FnMut(&[u8]) + Send,
    G: FnMut(&[u8]) + Send,
{
    let spec = CommandBuilder::new(VECTOR_RAG_BINARY)
        .arg("rag")
        .args(args.iter().cloned())
        .current_dir(root_dir)
        .build()
        .map_err(|error| format!("failed to prepare vector-rag command: {error}"))?;

    let mut handle = executor.spawn(spec).await.map_err(|_| INSTALL_GUIDANCE.to_owned())?;

    handle.stream_output(on_stdout, on_stderr).await;

    let exit =
        handle.wait().await.map_err(|error| format!("failed waiting for vector-rag: {error}"))?;

    if exit.success { Ok(DelegatedExit::Success) } else { Ok(DelegatedExit::Failure(exit.code)) }
}

#[cfg(test)]
#[path = "rag_passthrough_test.rs"]
mod tests;
