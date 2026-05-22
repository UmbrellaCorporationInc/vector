//! Executable entrypoint for the `mcp-vector` stdio server.

use std::ffi::OsString;
use std::io::Write;

use mcp_vector::release::version::workspace_version;
use mcp_vector::server::VectorServer;

enum ProcessMode {
    PrintVersion(&'static str),
    ServeMcp,
}

fn process_mode(args: impl IntoIterator<Item = OsString>) -> ProcessMode {
    match args.into_iter().next().as_deref() {
        Some(flag) if flag == "--version" => ProcessMode::PrintVersion(workspace_version()),
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
        ProcessMode::ServeMcp => VectorServer::new().serve_stdio().await,
    }
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;
