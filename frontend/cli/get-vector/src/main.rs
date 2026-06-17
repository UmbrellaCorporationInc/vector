//! Executable entrypoint for the `get-vector` operator CLI.

use get_vector::commands::update_mcp_vector::{UpdateOutcome, run, run_rag};
use runtime_io::ProcessCommandExecutor;

const HELP_TEXT: &str = "\
get-vector: Operator CLI for managing the local mcp-vector installation.

Usage:
  get-vector [OPTIONS] <COMMAND>

Options:
  -h, --help       Print help information
  -V, --version    Print version information

Commands:
  update-mcp-vector  Install or update the local mcp-vector and vector-database binaries
  install rag        Install or update optional local RAG support

To install or update get-vector itself, run:
+--------------------------------------------------------------------------------+
| cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector |
+--------------------------------------------------------------------------------+
";

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    Help,
    Version,
    Update,
    InstallRag,
    Unknown(String),
    Missing,
}

fn parse_args(args: &[String]) -> CliAction {
    if args.len() < 2 {
        return CliAction::Missing;
    }
    match args[1].as_str() {
        "--help" | "-h" => CliAction::Help,
        "--version" | "-V" => CliAction::Version,
        "update-mcp-vector" => CliAction::Update,
        "install" if args.get(2).is_some_and(|arg| arg == "rag") && args.get(3).is_none() => {
            CliAction::InstallRag
        }
        "install" => CliAction::Unknown(args[1..].join(" ")),
        unknown => CliAction::Unknown(unknown.to_string()),
    }
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        CliAction::Help => {
            println!("{HELP_TEXT}");
            std::process::exit(0);
        }
        CliAction::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        CliAction::Update => {
            let executor = ProcessCommandExecutor::default();
            eprintln!("Updating tools from git...");
            match run(
                &executor,
                |b| print!("{}", String::from_utf8_lossy(b)),
                |b| eprint!("{}", String::from_utf8_lossy(b)),
            )
            .await
            {
                Ok(UpdateOutcome::Installed) => {
                    println!("Tools installed successfully.");
                    println!(
                        "RAG support is not installed by this command. Run `get-vector install rag` to add it."
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            }
        }
        CliAction::InstallRag => {
            let executor = ProcessCommandExecutor::default();
            eprintln!("Installing RAG support from git...");
            match run_rag(
                &executor,
                |b| print!("{}", String::from_utf8_lossy(b)),
                |b| eprint!("{}", String::from_utf8_lossy(b)),
            )
            .await
            {
                Ok(UpdateOutcome::Installed) => {
                    println!("RAG support installed successfully.");
                }
                Ok(_) => {}
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            }
        }
        CliAction::Missing => {
            eprintln!("{HELP_TEXT}");
            std::process::exit(1);
        }
        CliAction::Unknown(unknown) => {
            eprintln!("error: unknown command or option '{unknown}'\n");
            eprintln!("{HELP_TEXT}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;
