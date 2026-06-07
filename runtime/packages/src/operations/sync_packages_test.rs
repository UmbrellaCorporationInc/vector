#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::*;
use crate::types::manifest::PackageEntry;
use crate::types::{PackageManifest, save_manifest};
use runtime_core::FlowOperation;
use runtime_core::cancel::CancelableSender;
use runtime_core::channel::Sender;
use std::collections::BTreeMap;

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
async fn test_sync_packages_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    let manifest = PackageManifest { packages: BTreeMap::new() };
    save_manifest(&root_path, &manifest).await.unwrap();

    let input = SyncPackagesInput::new(root_path);
    let mut sender = CapturingSender::new();
    SyncPackagesOp.run(input, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    assert!(output.actions.is_empty());
}

#[tokio::test]
async fn test_sync_packages_plan() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = IoPath::new(dir.path());

    // Create a mock packages directory structure
    let packages_dir = dir.path().join(".vector-database").join("packages");
    std::fs::create_dir_all(&packages_dir).unwrap();

    // pkg2 (git) already exists, pkg1 (git) does not, pkg3 (file) always copies
    std::fs::create_dir(packages_dir.join("pkg2")).unwrap();

    let mut packages = BTreeMap::new();
    packages.insert(
        "pkg2".to_string(),
        PackageEntry {
            r#type: "git".to_string(),
            url: "https://github.com/user/pkg2.git".to_string(),
            tag: Some("v1.0.0".to_string()),
        },
    );
    packages.insert(
        "pkg1".to_string(),
        PackageEntry {
            r#type: "git".to_string(),
            url: "https://github.com/user/pkg1.git".to_string(),
            tag: Some("branch:feature-xyz".to_string()),
        },
    );
    packages.insert(
        "pkg3".to_string(),
        PackageEntry { r#type: "file".to_string(), url: "./local/pkg3".to_string(), tag: None },
    );

    let manifest = PackageManifest { packages };
    save_manifest(&root_path, &manifest).await.unwrap();

    let input = SyncPackagesInput::new(root_path);
    let mut sender = CapturingSender::new();
    SyncPackagesOp.run(input, &mut sender).await.unwrap();

    let output = sender.output.unwrap();
    let actions = output.actions;

    // Verify ordering is deterministic (alphabetical: pkg1, pkg2, pkg3)
    assert_eq!(actions.len(), 3);

    // pkg1 - clone
    assert_eq!(actions[0].name, "pkg1");
    assert_eq!(actions[0].command_type, SyncCommandType::Clone);
    assert_eq!(
        actions[0].description,
        "clone the Git source and switch to feature-xyz in .vector-database/packages/pkg1"
    );

    // pkg2 - fetch
    assert_eq!(actions[1].name, "pkg2");
    assert_eq!(actions[1].command_type, SyncCommandType::Fetch);
    assert_eq!(
        actions[1].description,
        "git fetch and update the package in .vector-database/packages/pkg2"
    );

    // pkg3 - copy
    assert_eq!(actions[2].name, "pkg3");
    assert_eq!(actions[2].command_type, SyncCommandType::Copy);
    assert_eq!(
        actions[2].description,
        "copy data from the file source into .vector-database/packages/pkg3"
    );
}

#[test]
fn test_sync_command_type_helpers() {
    assert_eq!(SyncCommandType::Clone.as_str(), "clone");
    assert_eq!(SyncCommandType::Fetch.as_str(), "fetch");
    assert_eq!(SyncCommandType::Copy.as_str(), "copy");

    assert_eq!(format!("{}", SyncCommandType::Clone), "clone");
    assert_eq!(format!("{}", SyncCommandType::Fetch), "fetch");
    assert_eq!(format!("{}", SyncCommandType::Copy), "copy");
}
