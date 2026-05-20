use super::*;
use crate::Receiver;

struct IdentityOperation;

impl Operation<i32, i32> for IdentityOperation {
    async fn run(&self, input: i32) -> RuntimeResult<i32> {
        Ok(input)
    }
}

struct MultiplierOperation {
    factor: i32,
}

impl Operation<i32, i32> for MultiplierOperation {
    async fn run(&self, input: i32) -> RuntimeResult<i32> {
        Ok(input * self.factor)
    }
}

#[tokio::test]
async fn test_operation_1_1_identity() -> RuntimeResult<()> {
    let op = IdentityOperation;
    let result = op.run(42).await?;
    assert_eq!(result, 42);
    Ok(())
}

#[tokio::test]
async fn test_operation_1_1_multiplier() -> RuntimeResult<()> {
    let op = MultiplierOperation { factor: 10 };
    let result = op.run(5).await?;
    assert_eq!(result, 50);
    Ok(())
}

struct MockReceiver {
    values: Vec<i32>,
}

impl Receiver<i32> for MockReceiver {
    async fn recv(&mut self) -> RuntimeResult<Option<i32>> {
        if self.values.is_empty() { Ok(None) } else { Ok(Some(self.values.remove(0))) }
    }
}

struct SumOperation;

impl Operation<i32, i32> for SumOperation {
    async fn run(&self, input: i32) -> RuntimeResult<i32> {
        Ok(input)
    }
}

// N:1 implemented using the base Operation trait with a receiver as input
impl<R: Receiver<i32>> Operation<&mut R, i32> for SumOperation {
    async fn run(&self, input: &mut R) -> RuntimeResult<i32> {
        let mut sum = 0;
        while let Ok(Some(val)) = input.recv().await {
            sum += val;
        }
        Ok(sum)
    }
}

#[tokio::test]
async fn test_operation_n_1_sum() -> RuntimeResult<()> {
    let op = SumOperation;
    let mut rx = MockReceiver { values: vec![1, 2, 3, 4, 5] };
    let result = op.run(&mut rx).await?;
    assert_eq!(result, 15);
    Ok(())
}

#[tokio::test]
async fn test_operation_n_1_empty() -> RuntimeResult<()> {
    let op = SumOperation;
    let mut rx = MockReceiver { values: vec![] };
    let result = op.run(&mut rx).await?;
    assert_eq!(result, 0);
    Ok(())
}

#[test]
fn test_operation_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<IdentityOperation>();
    assert_send::<SumOperation>();
    assert_send::<SplitOperation>();
    assert_send::<FilterMultiplierOperation>();
}

struct MockSender {
    outputs: Vec<i32>,
}

impl Sender<i32> for MockSender {
    async fn send(&mut self, value: i32) -> RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

struct SplitOperation;

impl<S: Sender<i32>> FlowOperation<i32, i32, S> for SplitOperation {
    async fn run(&self, input: i32, output: &mut S) -> RuntimeResult<()> {
        output.send(input).await?;
        output.send(input * 10).await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_flow_operation_1_n() -> RuntimeResult<()> {
    let op = SplitOperation;
    let mut tx = MockSender { outputs: vec![] };
    op.run(5, &mut tx).await?;
    assert_eq!(tx.outputs, vec![5, 50]);
    Ok(())
}

struct FilterMultiplierOperation {
    factor: i32,
}

impl<S: Sender<i32>> FlowOperation<i32, i32, S> for FilterMultiplierOperation {
    async fn run(&self, input: i32, output: &mut S) -> RuntimeResult<()> {
        output.send(input * self.factor).await
    }
}

// N:N implemented using the base FlowOperation trait with a receiver as input
impl<R: Receiver<i32>, S: Sender<i32>> FlowOperation<&mut R, i32, S> for FilterMultiplierOperation {
    async fn run(&self, input: &mut R, output: &mut S) -> RuntimeResult<()> {
        while let Ok(Some(val)) = input.recv().await {
            output.send(val * self.factor).await?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_flow_operation_n_n() -> RuntimeResult<()> {
    let op = FilterMultiplierOperation { factor: 3 };
    let mut rx = MockReceiver { values: vec![1, 2, 3] };
    let mut tx = MockSender { outputs: vec![] };
    op.run(&mut rx, &mut tx).await?;
    assert_eq!(tx.outputs, vec![3, 6, 9]);
    Ok(())
}
