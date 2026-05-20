#![allow(clippy::expect_used)]

use super::CommandBuilder;
use std::path::Path;

#[test]
fn test_build_rejects_empty_command() {
    let error = CommandBuilder::new("   ").build().expect_err("build should fail");

    assert!(matches!(error, crate::IoError::Process(_)));
}

#[test]
fn test_builder_preserves_composed_configuration_order() {
    let spec = CommandBuilder::new("git")
        .arg("status")
        .args(["--short", "--branch"])
        .current_dir("runtime/io")
        .env("FIRST", "alpha")
        .env("SECOND", "beta")
        .build()
        .expect("build should succeed");

    assert_eq!(spec.command(), "git");
    assert_eq!(spec.args(), ["status", "--short", "--branch"]);
    assert_eq!(spec.current_dir(), Some(Path::new("runtime/io")));
    assert_eq!(
        spec.env(),
        [("FIRST".to_string(), "alpha".to_string()), ("SECOND".to_string(), "beta".to_string()),]
    );
}
