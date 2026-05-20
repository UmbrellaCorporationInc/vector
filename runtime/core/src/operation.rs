use crate::{RuntimeResult, Sender};
use std::future::Future;

/// Canonical async input-result contract for ordinary asynchronous computation.
///
/// `Operation<Input, Output>` is intended to cover `1:1` execution shapes.
/// It receives one input value and resolves to one output value wrapped in [`RuntimeResult<Output>`].
///
/// This contract must not encode scheduling, retries, cancellation, backpressure,
/// supervision, ordering guarantees, lifecycle ownership, or transport policy.
pub trait Operation<Input, Output>: Send {
    /// Executes the operation with a single input value.
    fn run(&self, input: Input) -> impl Future<Output = RuntimeResult<Output>> + Send;
}

/// Canonical async input-output-flow contract for asynchronous dataflow.
///
/// `FlowOperation<Input, Output, S>` is intended to cover `1:N` execution shapes.
/// It receives a plain input value and writes results through a [`Sender<Output>`].
///
/// This contract must not encode scheduling, retries, cancellation, backpressure,
/// supervision, ordering guarantees, lifecycle ownership, or transport policy.
pub trait FlowOperation<Input, Output, S: Sender<Output>>: Send {
    /// Executes the flow operation with a single input value and a sender for outputs.
    fn run(&self, input: Input, output: &mut S) -> impl Future<Output = RuntimeResult<()>> + Send;
}

#[cfg(test)]
#[path = "operation_test.rs"]
mod tests;
