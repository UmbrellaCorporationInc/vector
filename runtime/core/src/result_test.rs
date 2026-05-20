use crate::{RuntimeError, RuntimeResult};

#[test]
fn runtime_result_alias_uses_runtime_error() {
    let result: RuntimeResult<()> = Err(RuntimeError::operation("test"));

    assert!(matches!(result, Err(RuntimeError::Operation(_))));
}
