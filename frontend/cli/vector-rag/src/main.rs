//! Executable entrypoint for the `vector-rag` companion CLI.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::path::{Path, PathBuf};
use vector_rag::commands::{rag_init, rag_search, rag_update_database};

const HELP_TEXT: &str = "\
vector-rag: Companion CLI for local RAG runtime execution.

Usage:
  vector-rag rag init
  vector-rag rag search <query> [--package <name>] [--document <stem>] [--limit <n>] [--json]
  vector-rag rag update-database

Commands:
  rag init             Create or validate the local RAG LanceDB store
  rag search           Search the local RAG store with hybrid retrieval
  rag update-database  Index workspace documents into the RAG store
";

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliAction {
    Help,
    Version,
    RagInit,
    RagSearch(Vec<String>),
    RagUpdateDatabase,
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
        "rag" => parse_rag_args(&args[2..]),
        unknown => CliAction::Unknown(unknown.to_owned()),
    }
}

fn parse_rag_args(args: &[String]) -> CliAction {
    let Some(command) = args.first() else {
        return CliAction::Missing;
    };
    match command.as_str() {
        "init" => CliAction::RagInit,
        "search" => CliAction::RagSearch(args[1..].to_vec()),
        "update-database" => CliAction::RagUpdateDatabase,
        unknown => CliAction::Unknown(format!("rag {unknown}")),
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    match run_cli(&args).await {
        Ok(()) => {}
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{HELP_TEXT}");
            std::process::exit(1);
        }
    }
}

async fn run_cli(args: &[String]) -> Result<(), String> {
    match parse_args(args) {
        CliAction::Help => {
            println!("{HELP_TEXT}");
            Ok(())
        }
        CliAction::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        CliAction::RagInit => {
            let root_dir = find_root_dir()?;
            handle_rag_command(&root_dir, CliAction::RagInit).await
        }
        CliAction::RagSearch(search_args) => {
            let root_dir = find_root_dir()?;
            handle_rag_command(&root_dir, CliAction::RagSearch(search_args)).await
        }
        CliAction::RagUpdateDatabase => {
            let root_dir = find_root_dir()?;
            handle_rag_command(&root_dir, CliAction::RagUpdateDatabase).await
        }
        CliAction::Missing => Err("missing command".to_owned()),
        CliAction::Unknown(unknown) => Err(format!("unknown command or option '{unknown}'")),
    }
}

async fn handle_rag_command(root_dir: &Path, action: CliAction) -> Result<(), String> {
    match action {
        CliAction::RagInit => rag_init::run(root_dir).await,
        CliAction::RagSearch(args) => {
            let parsed = rag_search::parse_args(&args)?;
            rag_search::run(root_dir, parsed).await
        }
        CliAction::RagUpdateDatabase => rag_update_database::run(root_dir).await,
        _ => Err("internal error: non-RAG action routed to RAG handler".to_owned()),
    }
}

fn find_root_dir() -> Result<PathBuf, String> {
    let mut dir =
        std::env::current_dir().map_err(|e| format!("failed to get current directory: {e}"))?;
    loop {
        if dir.join(".vector").is_dir() {
            return Ok(dir);
        }
        if let Some(parent) = dir.parent() {
            dir = parent.to_path_buf();
        } else {
            return Err(
                "could not find project root directory containing .vector folder".to_string()
            );
        }
    }
}

#[cfg(test)]
#[path = "main_test.rs"]
mod tests;
