//! Executable entrypoint for the `mcp-vector` stdio server.

use std::ffi::OsString;
use std::io::Write;

use mcp_vector::release::version::workspace_version;
use mcp_vector::server::VectorServer;
use runtime_core::channel::Receiver;

const HELP_TEXT: &str = "\
mcp-vector: Canonical Model Context Protocol (MCP) server for the vector system.

Usage:
  mcp-vector [OPTIONS]
  mcp-vector [SUBCOMMAND]

Options:
  -h, --help       Print help information
  -V, --version    Print version information

Subcommands:
  create-project   Scaffold a governed vector project with vault and workspace
";

enum ProcessMode {
    PrintVersion(&'static str),
    PrintHelp,
    CreateProject { project_name: Option<String> },
    ServeMcp,
}

fn process_mode(args: impl IntoIterator<Item = OsString>) -> ProcessMode {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some(flag) if flag == "--version" || flag == "-V" => {
            ProcessMode::PrintVersion(workspace_version())
        }
        Some(flag) if flag == "--help" || flag == "-h" => ProcessMode::PrintHelp,
        Some(cmd) if cmd == "create-project" => {
            let project_name = iter.next().map(|s| s.to_string_lossy().into_owned());
            ProcessMode::CreateProject { project_name }
        }
        _ => ProcessMode::ServeMcp,
    }
}

#[tokio::main]
async fn main() -> Result<(), mcp_vector::error::VectorServerError> {
    match process_mode(std::env::args_os().skip(1)) {
        ProcessMode::PrintVersion(version) => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(version.as_bytes())?;
            stdout.write_all(b"\n")?;
            Ok(())
        }
        ProcessMode::PrintHelp => {
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(HELP_TEXT.as_bytes())?;
            Ok(())
        }
        ProcessMode::CreateProject { project_name } => {
            let current_dir = std::env::current_dir()?;
            let name = project_name.unwrap_or_else(|| {
                current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("vector-project")
                    .to_string()
            });

            let input = runtime_project::ProjectSetupInput::new(
                runtime_io::path::IoPath::new(current_dir),
                name,
                false,
            );

            let (_cancel, mut receiver) =
                runtime_channel::PluginDispatcher::new(runtime_project::ProjectSetupOp::default())
                    .input(input)
                    .build()
                    .map_err(|e| std::io::Error::other(format!("dispatcher build failed: {e}")))?;

            let mut stdout = std::io::stdout().lock();
            while let Ok(Some(result)) = receiver.recv().await {
                stdout.write_all(result.project.message.as_bytes())?;
                stdout.write_all(b"\n")?;
            }
            Ok(())
        }
        ProcessMode::ServeMcp => VectorServer::new().serve_stdio().await,
    }
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;
