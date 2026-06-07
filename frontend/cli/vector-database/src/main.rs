//! Executable entrypoint for the `vector-database` CLI.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_io::ProcessCommandExecutor;
use std::path::PathBuf;
use vector_database::commands::{package_add, package_sync};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 || args[1] != "package" {
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

    match args[2].as_str() {
        "sync" => {
            let executor = ProcessCommandExecutor::default();
            if let Err(e) = package_sync::run(&executor, &root_dir).await {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        "add" => {
            let mut name = None;
            let mut r#type = None;
            let mut url = None;
            let mut tag = None;

            let mut args_iter = args[3..].iter();
            let mut positionals = Vec::new();
            while let Some(arg) = args_iter.next() {
                if arg.starts_with('-') {
                    match arg.as_str() {
                        "--name" | "-n" => {
                            if let Some(val) = args_iter.next() {
                                name = Some(val.clone());
                            }
                        }
                        "--type" | "-t" => {
                            if let Some(val) = args_iter.next() {
                                r#type = Some(val.clone());
                            }
                        }
                        "--url" | "-u" => {
                            if let Some(val) = args_iter.next() {
                                url = Some(val.clone());
                            }
                        }
                        "--tag" | "-g" => {
                            if let Some(val) = args_iter.next() {
                                tag = Some(val.clone());
                            }
                        }
                        _ => {}
                    }
                } else {
                    positionals.push(arg.clone());
                }
            }

            if name.is_none() && !positionals.is_empty() {
                name = Some(positionals[0].clone());
            }
            if r#type.is_none() && positionals.len() > 1 {
                r#type = Some(positionals[1].clone());
            }
            if url.is_none() && positionals.len() > 2 {
                url = Some(positionals[2].clone());
            }
            if tag.is_none() && positionals.len() > 3 {
                tag = Some(positionals[3].clone());
            }

            let Some(name) = name else {
                eprintln!("error: missing package name");
                print_add_usage();
                std::process::exit(1);
            };
            let Some(r#type) = r#type else {
                eprintln!("error: missing package type");
                print_add_usage();
                std::process::exit(1);
            };
            let Some(url) = url else {
                eprintln!("error: missing package url");
                print_add_usage();
                std::process::exit(1);
            };

            if let Err(e) = package_add::run(&root_dir, name, r#type, url, tag).await {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        cmd => {
            eprintln!("error: unknown package subcommand '{cmd}'");
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Usage: vector-database package <subcommand> [args]");
    println!();
    println!("Subcommands:");
    println!("  sync                     Synchronize packages defined in the manifest");
    println!("  add <name> <type> <url> [tag]  Add a new package to the manifest");
}

fn print_add_usage() {
    println!("Usage: vector-database package add <name> <type> <url> [tag]");
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
