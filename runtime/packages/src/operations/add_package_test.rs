#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use runtime_core::FlowOperation;
use runtime_core::cancel::CancelableSender;
use runtime_core::channel::Sender;

struct CapturingSender<T> {
    output: Option<T>,
}

impl<T> CapturingSender<T> {
    fn new() -> Self {
        Self { output: None }
    }
}

impl<T: Send> Sender<T> for CapturingSender<T> {
    async fn send(&mut self, value: T) -> runtime_core::result::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl<T: Send> CancelableSender<T> for CapturingSender<T> {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_add_package_success_git() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let input = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/user/pkg1.git".to_string(),
        Some("v1.0.0".to_string()),
    );
    let mut sender = CapturingSender::new();
    AddPackageOp.run(input, &mut sender).await.unwrap();

    let manifest = load_manifest(&root_path).await.unwrap();
    assert_eq!(manifest.packages.len(), 1);
    let pkg = manifest.packages.get("pkg1").unwrap();
    assert_eq!(pkg.r#type, "git");
    assert_eq!(pkg.url, "https://github.com/user/pkg1.git");
    assert_eq!(pkg.tag, Some("v1.0.0".to_string()));
}

#[tokio::test]
async fn test_add_package_success_file() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let input = AddPackageInput::new(
        root_path.clone(),
        "pkg_file".to_string(),
        "file".to_string(),
        "./local/path".to_string(),
        None,
    );
    let mut sender = CapturingSender::new();
    AddPackageOp.run(input, &mut sender).await.unwrap();

    let manifest = load_manifest(&root_path).await.unwrap();
    assert_eq!(manifest.packages.len(), 1);
    let pkg = manifest.packages.get("pkg_file").unwrap();
    assert_eq!(pkg.r#type, "file");
    assert_eq!(pkg.url, "./local/path");
    assert_eq!(pkg.tag, None);
}

#[tokio::test]
async fn test_add_package_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    // First add
    let input1 = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/user/pkg1.git".to_string(),
        Some("v1.0.0".to_string()),
    );
    let mut sender = CapturingSender::new();
    AddPackageOp.run(input1, &mut sender).await.unwrap();

    // Second add with same name
    let input2 = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "file".to_string(),
        "./another/path".to_string(),
        None,
    );
    let err = AddPackageOp.run(input2, &mut sender).await.unwrap_err();
    assert!(err.to_string().contains("package 'pkg1' is already present in manifest"));
}

#[tokio::test]
async fn test_add_package_git_missing_tag() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let input = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/user/pkg1.git".to_string(),
        None,
    );
    let mut sender = CapturingSender::new();
    let err = AddPackageOp.run(input, &mut sender).await.unwrap_err();
    assert!(err.to_string().contains("tag is required for git packages"));
}

#[tokio::test]
async fn test_add_package_git_invalid_branch() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let input = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "git".to_string(),
        "https://github.com/user/pkg1.git".to_string(),
        Some("branch:".to_string()),
    );
    let mut sender = CapturingSender::new();
    let err = AddPackageOp.run(input, &mut sender).await.unwrap_err();
    assert!(err.to_string().contains("invalid branch format in tag 'branch:'"));
}

#[tokio::test]
async fn test_add_package_unsupported_type() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let input = AddPackageInput::new(
        root_path.clone(),
        "pkg1".to_string(),
        "hg".to_string(),
        "https://github.com/user/pkg1.git".to_string(),
        None,
    );
    let mut sender = CapturingSender::new();
    let err = AddPackageOp.run(input, &mut sender).await.unwrap_err();
    assert!(err.to_string().contains("has unsupported source type 'hg'"));
}
