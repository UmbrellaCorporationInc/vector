//! Executable entrypoint for the `vector-database` CLI.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_io::ProcessCommandExecutor;
use std::path::PathBuf;
use vector_database::commands::{
    package_add, package_sync, rag_init, rag_search, rag_update_database,
};

#[derive(Debug, Default)]
struct PackageAddArgs {
    name: Option<String>,
    package_type: Option<String>,
    url: Option<String>,
    tag: Option<String>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }

    let root_dir = match find_root_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let result = match args[1].as_str() {
        "package" => handle_package_command(&root_dir, &args).await,
        "rag" => handle_rag_command(&root_dir, &args).await,
        cmd => Err(format!("unknown command group '{cmd}'")),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        print_usage();
        std::process::exit(1);
    }
}

fn print_usage() {
    println!("Usage: vector-database <command-group> <subcommand> [args]");
    println!();
    println!("Command groups:");
    println!("  package sync                    Synchronize packages defined in the manifest");
    println!("  package add <name> <type> <url> [tag]  Add a new package to the manifest");
    println!("  rag init                        Create or validate the local RAG LanceDB store");
    println!("  rag search <query>              Search the local RAG store with hybrid retrieval");
    println!("  rag update-database             Index workspace documents into the RAG store");
}

fn print_add_usage() {
    println!("Usage: vector-database package add <name> <type> <url> [tag]");
}

async fn handle_package_command(root_dir: &std::path::Path, args: &[String]) -> Result<(), String> {
    match args[2].as_str() {
        "sync" => {
            let executor = ProcessCommandExecutor::default();
            package_sync::run(&executor, root_dir).await
        }
        "add" => {
            let parsed = parse_package_add_args(&args[3..]);
            let name = parsed.name.ok_or_else(|| missing_package_argument("name"))?;
            let package_type =
                parsed.package_type.ok_or_else(|| missing_package_argument("type"))?;
            let url = parsed.url.ok_or_else(|| missing_package_argument("url"))?;
            package_add::run(root_dir, name, package_type, url, parsed.tag).await
        }
        cmd => Err(format!("unknown package subcommand '{cmd}'")),
    }
}

async fn handle_rag_command(root_dir: &std::path::Path, args: &[String]) -> Result<(), String> {
    match args[2].as_str() {
        "init" => rag_init::run(root_dir).await,
        "search" => {
            let parsed = rag_search::parse_args(&args[3..])?;
            rag_search::run(root_dir, parsed).await
        }
        "update-database" => rag_update_database::run(root_dir).await,
        cmd => Err(format!("unknown rag subcommand '{cmd}'")),
    }
}

fn parse_package_add_args(args: &[String]) -> PackageAddArgs {
    let mut parsed = PackageAddArgs::default();
    let mut args_iter = args.iter();
    let mut positionals = Vec::new();

    while let Some(arg) = args_iter.next() {
        if arg.starts_with('-') {
            match arg.as_str() {
                "--name" | "-n" => parsed.name = args_iter.next().cloned(),
                "--type" | "-t" => parsed.package_type = args_iter.next().cloned(),
                "--url" | "-u" => parsed.url = args_iter.next().cloned(),
                "--tag" | "-g" => parsed.tag = args_iter.next().cloned(),
                _ => {}
            }
        } else {
            positionals.push(arg.clone());
        }
    }

    if parsed.name.is_none() && !positionals.is_empty() {
        parsed.name = Some(positionals[0].clone());
    }
    if parsed.package_type.is_none() && positionals.len() > 1 {
        parsed.package_type = Some(positionals[1].clone());
    }
    if parsed.url.is_none() && positionals.len() > 2 {
        parsed.url = Some(positionals[2].clone());
    }
    if parsed.tag.is_none() && positionals.len() > 3 {
        parsed.tag = Some(positionals[3].clone());
    }

    parsed
}

fn missing_package_argument(name: &str) -> String {
    print_add_usage();
    format!("missing package {name}")
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
