use super::*;
use crate::channel::Sender;
use crate::operation::FlowOperation;
use crate::result::RuntimeResult;

// Mock structures for testing
struct MockSender;
impl crate::channel::Sender<i32> for MockSender {
    async fn send(&mut self, _v: i32) -> RuntimeResult<()> {
        Ok(())
    }
}
impl crate::cancel::CancelableSender<i32> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

// External logic functions
async fn add_one(input: i32, output: &mut impl PluginSender<i32>) -> RuntimeResult<()> {
    output.send(input + 1).await
}

// Manifest declaration
declare_plugin_operations! {
    AddOneOp => add_one(i32, i32)
}

#[tokio::test]
async fn test_plugin_operation_macro() -> RuntimeResult<()> {
    // 1. Instantiate the generated struct
    let op = AddOneOp;

    // 2. Execute through FlowOperation trait
    let mut tx = MockSender;
    op.run(10, &mut tx).await?;

    Ok(())
}

struct ManualOp;
impl crate::plugin::PluginOperation<MockSender> for ManualOp {
    type Input = i32;
    type Output = i32;
}
impl crate::operation::FlowOperation<i32, i32, MockSender> for ManualOp {
    async fn run(&self, input: i32, output: &mut MockSender) -> RuntimeResult<()> {
        output.send(input + 1).await
    }
}

#[tokio::test]
async fn test_manual_plugin_operation() -> RuntimeResult<()> {
    let op = ManualOp;

    let mut tx = MockSender;
    op.run(10, &mut tx).await?;
    Ok(())
}
