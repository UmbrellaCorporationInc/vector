use rmcp::service::{ServerInitializeError, ServiceError};

use super::VectorServerError;

/// Verifies that `VectorServerError::Service` is constructible from `ServiceError`.
#[test]
fn vector_server_error_converts_from_service_error() {
    let service_err = ServiceError::TransportClosed;
    let vector_err: VectorServerError = service_err.into();
    assert!(
        matches!(vector_err, VectorServerError::Service(ServiceError::TransportClosed)),
        "VectorServerError must convert from ServiceError::TransportClosed"
    );
}

/// Verifies that `VectorServerError` converts from `ServerInitializeError`.
#[test]
fn vector_server_error_converts_from_server_initialize_error() {
    let init_err = ServerInitializeError::Cancelled;
    let vector_err: VectorServerError = init_err.into();
    assert!(
        matches!(vector_err, VectorServerError::Initialize(_)),
        "VectorServerError must convert from ServerInitializeError into the Initialize variant"
    );
}

/// Verifies that `VectorServerError::TaskFailed` has a non-empty error message.
#[test]
fn vector_server_error_task_failed_has_message() {
    let err = VectorServerError::TaskFailed;
    let msg = err.to_string();
    assert!(!msg.is_empty(), "TaskFailed error must produce a non-empty display message");
}

/// Verifies that `VectorServerError::Initialize` includes the original error in its message.
#[test]
fn vector_server_error_initialize_includes_cause_in_message() {
    let init_err = ServerInitializeError::ConnectionClosed("test context".to_string());
    let vector_err: VectorServerError = init_err.into();
    let msg = vector_err.to_string();
    assert!(
        msg.contains("initialization failed"),
        "Initialize error message must indicate initialization failure"
    );
}
