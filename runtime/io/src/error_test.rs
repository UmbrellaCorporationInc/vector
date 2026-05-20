use super::*;

#[test]
fn test_error_construction() {
    let err = IoError::File("not found".into());
    assert_eq!(err.to_string(), "file operation failed: not found");

    let err = IoError::Path("invalid".into());
    assert_eq!(err.to_string(), "path operation failed: invalid");

    let err = IoError::Text("bad bytes".into());
    assert_eq!(err.to_string(), "utf-8 processing failed: bad bytes");

    let err = IoError::Process("exit code 1".into());
    assert_eq!(err.to_string(), "process execution failed: exit code 1");
}
