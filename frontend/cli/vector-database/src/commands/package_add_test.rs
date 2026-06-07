#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_packages::types::load_manifest;

#[tokio::test]
async fn test_package_add_success() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    std::fs::create_dir_all(root.join(".vector")).unwrap();

    run(
        root,
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/org/pkg1.git".to_string(),
        Some("v1.0.0".to_string()),
    )
    .await
    .expect("adding package should succeed");

    let manifest = load_manifest(&IoPath::new(root)).await.unwrap();
    assert_eq!(manifest.packages.len(), 1);

    let entry = manifest.packages.get("pkg1").unwrap();
    assert_eq!(entry.r#type, "git");
    assert_eq!(entry.url, "https://github.com/org/pkg1.git");
    assert_eq!(entry.tag, Some("v1.0.0".to_string()));
}

#[tokio::test]
async fn test_package_add_duplicate_fails() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    std::fs::create_dir_all(root.join(".vector")).unwrap();

    run(
        root,
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/org/pkg1.git".to_string(),
        Some("v1.0.0".to_string()),
    )
    .await
    .unwrap();

    let result = run(
        root,
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/org/pkg1.git".to_string(),
        Some("v2.0.0".to_string()),
    )
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already present"));
}
