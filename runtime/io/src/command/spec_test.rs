#![allow(clippy::expect_used)]

use crate::command::CommandBuilder;
use std::path::Path;

#[test]
fn test_spec_preserves_configuration() {
    let spec = CommandBuilder::new("echo")
        .args(["alpha", "beta"])
        .current_dir("runtime")
        .env("VECTOR_MODE", "test")
        .build()
        .expect("build failed");

    assert_eq!(spec.command(), "echo");
    assert_eq!(spec.args(), ["alpha", "beta"]);
    assert_eq!(spec.current_dir(), Some(Path::new("runtime")));
    assert_eq!(spec.env(), [("VECTOR_MODE".to_string(), "test".to_string())]);
}
