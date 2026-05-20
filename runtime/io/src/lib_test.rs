#![allow(clippy::expect_used)]

use crate::{
    CommandBuilder, CommandExecutor, CommandSpec, MockCommandHandleBuilder, ProcessCommandExecutor,
};

#[test]
fn test_crate_root_reexports_command_api() {
    fn assert_executor<E: CommandExecutor>(_executor: &E) {}

    let spec: CommandSpec = CommandBuilder::new("echo").arg("hello").build().expect("build failed");
    let executor = ProcessCommandExecutor;
    assert_executor(&executor);
    let _builder =
        MockCommandHandleBuilder::new(crate::CommandExit { success: true, code: Some(0) });
    let _ = spec;
}
