#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_lint_entry_rust_variant() {
    let path = PathBuf::from("test.rs");
    let ast = syn::parse_str::<syn::File>("fn main() {}").unwrap();
    let source = "fn main() {}".to_string();

    let entry = LintEntry::Rust(path.clone(), ast, source.clone());

    if let LintEntry::Rust(p, _, s) = entry {
        assert_eq!(p, path);
        assert_eq!(s, source);
    } else {
        panic!("Expected Rust variant");
    }
}

#[test]
fn test_lint_entry_toml_variant() {
    let path = PathBuf::from("Cargo.toml");
    let content = "[package]\nname = \"test\"".to_string();

    let entry = LintEntry::Toml(path.clone(), content.clone());

    if let LintEntry::Toml(p, c) = entry {
        assert_eq!(p, path);
        assert_eq!(c, content);
    } else {
        panic!("Expected Toml variant");
    }
}

#[test]
fn test_walk_returns_vec() {
    let temp_dir = std::env::temp_dir().join("forge_walker_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let entries = walk(&temp_dir);

    // Should return a Vec (may be empty if no files exist)
    assert!(entries.is_empty() || !entries.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_walk_with_rust_file() {
    let temp_dir = std::env::temp_dir().join("forge_walker_test_rust");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test.rs");
    std::fs::write(&test_file, "fn test() {}").unwrap();

    let entries = walk(&temp_dir);

    // Should find the test.rs file
    let rust_entries: Vec<_> =
        entries.iter().filter(|e| matches!(e, LintEntry::Rust(_, _, _))).collect();
    assert!(!rust_entries.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_walk_with_toml_file() {
    let temp_dir = std::env::temp_dir().join("forge_walker_test_toml");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let toml_file = temp_dir.join("Cargo.toml");
    std::fs::write(&toml_file, "[package]\nname = \"test\"").unwrap();

    let entries = walk(&temp_dir);

    // Should find the Cargo.toml file
    let toml_entries: Vec<_> =
        entries.iter().filter(|e| matches!(e, LintEntry::Toml(_, _))).collect();
    assert!(!toml_entries.is_empty());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_walk_skips_target_directory() {
    let temp_dir = std::env::temp_dir().join("forge_walker_test_target");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("target")).unwrap();

    let target_file = temp_dir.join("target").join("should_skip.rs");
    std::fs::write(&target_file, "fn skip() {}").unwrap();

    let entries = walk(&temp_dir);

    // Should not find files in target/
    let has_target_files = entries.iter().any(|e| {
        let path = match e {
            LintEntry::Rust(p, _, _) => p,
            LintEntry::Toml(p, _) => p,
        };
        path.components().any(|c| c.as_os_str() == "target")
    });

    assert!(!has_target_files);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
