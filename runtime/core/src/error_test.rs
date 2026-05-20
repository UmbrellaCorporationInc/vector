use super::RuntimeError;

#[test]
fn operation_error_displays_message_with_payload() {
    let err = RuntimeError::operation("invalid slug");
    assert_eq!(err.to_string(), "runtime operation failed: invalid slug");
}

#[test]
fn operation_constructor_builds_correct_variant() {
    let err = RuntimeError::operation("test reason");
    assert!(matches!(err, RuntimeError::Operation(ref msg) if msg == "test reason"));
}
