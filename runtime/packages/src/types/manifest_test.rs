#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;

#[test]
fn test_parse_valid_manifest() {
    let yaml = r"
pkg1:
  type: git
  url: https://github.com/user/pkg1.git
  tag: v1.0.0
pkg2:
  type: file
  url: ./local/pkg2
  tag: branch:main
pkg3:
  type: file
  url: /absolute/path
";
    let manifest = PackageManifest::parse(yaml).unwrap();
    assert_eq!(manifest.packages.len(), 3);

    let pkg1 = manifest.packages.get("pkg1").unwrap();
    assert_eq!(pkg1.r#type, "git");
    assert_eq!(pkg1.url, "https://github.com/user/pkg1.git");
    assert_eq!(pkg1.tag, Some("v1.0.0".to_string()));

    let pkg2 = manifest.packages.get("pkg2").unwrap();
    assert_eq!(pkg2.r#type, "file");
    assert_eq!(pkg2.url, "./local/pkg2");
    assert_eq!(pkg2.tag, Some("branch:main".to_string()));

    let pkg3 = manifest.packages.get("pkg3").unwrap();
    assert_eq!(pkg3.r#type, "file");
    assert_eq!(pkg3.url, "/absolute/path");
    assert_eq!(pkg3.tag, None);
}

#[test]
fn test_parse_not_a_map() {
    let yaml = "hello";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::NotAMap);
}

#[test]
fn test_parse_entry_not_a_map() {
    let yaml = r"
pkg1: hello
";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::EntryNotAMap("pkg1".to_string()));
}

#[test]
fn test_parse_missing_fields() {
    let yaml = r"
pkg1:
  url: https://github.com/user/pkg1.git
";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::MissingType("pkg1".to_string()));

    let yaml2 = r"
pkg1:
  type: git
";
    let err2 = PackageManifest::parse(yaml2).unwrap_err();
    assert_eq!(err2, ManifestError::MissingUrl("pkg1".to_string()));
}

#[test]
fn test_parse_unsupported_type() {
    let yaml = r"
pkg1:
  type: hg
  url: https://github.com/user/pkg1.git
";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::UnsupportedType("pkg1".to_string(), "hg".to_string()));
}

#[test]
fn test_parse_git_missing_tag() {
    let yaml = r"
pkg1:
  type: git
  url: https://github.com/user/pkg1.git
";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::MissingTagForGit("pkg1".to_string()));
}

#[test]
fn test_parse_git_invalid_branch() {
    let yaml = r#"
pkg1:
  type: git
  url: https://github.com/user/pkg1.git
  tag: "branch:"
"#;
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert_eq!(err, ManifestError::InvalidBranchFormat("pkg1".to_string(), "branch:".to_string()));

    let yaml2 = r#"
pkg1:
  type: git
  url: https://github.com/user/pkg1.git
  tag: "branch:  "
"#;
    let err2 = PackageManifest::parse(yaml2).unwrap_err();
    assert_eq!(
        err2,
        ManifestError::InvalidBranchFormat("pkg1".to_string(), "branch:  ".to_string())
    );
}

#[test]
fn test_parse_invalid_formats() {
    let yaml = r"
pkg1:
  type: 123
  url: https://github.com/user/pkg1.git
";
    let err = PackageManifest::parse(yaml).unwrap_err();
    assert!(matches!(err, ManifestError::InvalidTypeFormat(_)));

    let yaml_url = r"
pkg1:
  type: git
  url: 123
  tag: v1.0.0
";
    let err_url = PackageManifest::parse(yaml_url).unwrap_err();
    assert!(matches!(err_url, ManifestError::InvalidUrlFormat(_)));

    let yaml2 = r"
pkg1:
  type: git
  url: https://github.com/user/pkg1.git
  tag: 456
";
    let err2 = PackageManifest::parse(yaml2).unwrap_err();
    assert!(matches!(err2, ManifestError::InvalidTagFormat(_)));
}

#[test]
fn test_to_yaml_roundtrip() {
    let mut packages = std::collections::BTreeMap::new();
    packages.insert(
        "pkg1".to_string(),
        PackageEntry {
            r#type: "git".to_string(),
            url: "https://github.com/user/pkg1.git".to_string(),
            tag: Some("v1.0.0".to_string()),
        },
    );
    packages.insert(
        "pkg2".to_string(),
        PackageEntry { r#type: "file".to_string(), url: "./local/pkg2".to_string(), tag: None },
    );

    let manifest = PackageManifest { packages };
    let yaml = manifest.to_yaml().unwrap();
    let parsed = PackageManifest::parse(&yaml).unwrap();
    assert_eq!(parsed, manifest);
}

#[tokio::test]
async fn test_load_save_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let mut packages = std::collections::BTreeMap::new();
    packages.insert(
        "pkg1".to_string(),
        PackageEntry {
            r#type: "git".to_string(),
            url: "https://github.com/user/pkg1.git".to_string(),
            tag: Some("v1.0.0".to_string()),
        },
    );
    let manifest = PackageManifest { packages };

    save_manifest(&root_path, &manifest).await.unwrap();

    let loaded = load_manifest(&root_path).await.unwrap();
    assert_eq!(loaded, manifest);
}

#[tokio::test]
async fn test_load_non_existent_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());
    let err = load_manifest(&root_path).await.unwrap_err();
    assert!(err.to_string().contains("failed to read .vector/packages.yaml"));
}
