#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_io::IoPath;
use runtime_packages::types::{PackageManifest, load_manifest, save_manifest};

#[tokio::test]
async fn test_dispatch_rag_command_group_is_rejected_as_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let args = vec!["vector-database".to_string(), "rag".to_string(), "init".to_string()];
    let result = dispatch_command(root, &args).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "unknown command group 'rag'");

    let args_search = vec![
        "vector-database".to_string(),
        "rag".to_string(),
        "search".to_string(),
        "query".to_string(),
    ];
    let result_search = dispatch_command(root, &args_search).await;
    assert!(result_search.is_err());
    assert_eq!(result_search.unwrap_err(), "unknown command group 'rag'");

    let args_update =
        vec!["vector-database".to_string(), "rag".to_string(), "update-database".to_string()];
    let result_update = dispatch_command(root, &args_update).await;
    assert!(result_update.is_err());
    assert_eq!(result_update.unwrap_err(), "unknown command group 'rag'");
}

#[tokio::test]
async fn test_dispatch_package_add_retained_behavior() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join(".vector")).unwrap();

    let args = vec![
        "vector-database".to_string(),
        "package".to_string(),
        "add".to_string(),
        "test-pkg".to_string(),
        "git".to_string(),
        "https://github.com/org/test-pkg.git".to_string(),
        "v1.0.0".to_string(),
    ];

    let result = dispatch_command(root, &args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);

    let manifest = load_manifest(&IoPath::new(root)).await.unwrap();
    assert_eq!(manifest.packages.len(), 1);
    assert!(manifest.packages.contains_key("test-pkg"));
}

#[tokio::test]
async fn test_dispatch_package_sync_empty_manifest_retained_behavior() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let manifest = PackageManifest::parse("{}").unwrap();
    save_manifest(&IoPath::new(root), &manifest).await.unwrap();

    let args = vec!["vector-database".to_string(), "package".to_string(), "sync".to_string()];

    let result = dispatch_command(root, &args).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
}

#[tokio::test]
async fn test_dispatch_unknown_package_subcommand() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let args = vec!["vector-database".to_string(), "package".to_string(), "invalid".to_string()];

    let result = dispatch_command(root, &args).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "unknown package subcommand 'invalid'");
}
