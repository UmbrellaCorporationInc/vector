//! Recursive workspace walker that yields `(PathBuf, syn::File)` pairs.
//!
//! Skips `target/` and `.git/` directories. Files that fail `syn::parse_file`
//! are skipped with a warning printed to `stderr`.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// An entry produced by [`walk`]: either a parsed Rust AST or a raw TOML string.
pub(crate) enum LintEntry {
    /// A successfully parsed Rust source file.
    ///
    /// Fields: `(path, ast, raw_source)`.
    Rust(PathBuf, syn::File, String),
    /// A raw TOML manifest file.
    Toml(PathBuf, String),
}

/// Walk all `.rs` and `Cargo.toml` files under `workspace_root`, skipping `target/` and `.git/`.
///
/// Rust files that fail to parse are skipped with a warning to `stderr`.
#[must_use]
pub(crate) fn walk(workspace_root: &Path) -> Vec<LintEntry> {
    WalkDir::new(workspace_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != "target" && name != ".git"
        })
        .filter_map(|entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    eprintln!("lint_rules: walkdir error: {err}");
                    return None;
                }
            };
            let path = entry.path().to_path_buf();
            if !path.is_file() {
                return None;
            }

            let name = path.file_name()?.to_string_lossy();
            let extension = path.extension().and_then(|e| e.to_str());

            if extension == Some("rs") {
                let source = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(err) => {
                        eprintln!("lint_rules: cannot read {}: {err}", path.display());
                        return None;
                    }
                };
                match syn::parse_file(&source) {
                    Ok(ast) => Some(LintEntry::Rust(path, ast, source)),
                    Err(err) => {
                        eprintln!("lint_rules: syn parse error in {}: {err}", path.display());
                        None
                    }
                }
            } else if name == "Cargo.toml" {
                let source = match std::fs::read_to_string(&path) {
                    Ok(s) => s,
                    Err(err) => {
                        eprintln!("lint_rules: cannot read {}: {err}", path.display());
                        return None;
                    }
                };
                Some(LintEntry::Toml(path, source))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "walker_test.rs"]
mod tests;
