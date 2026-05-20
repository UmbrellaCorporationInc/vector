//! Plugin execution primitives for the vector runtime.
//!
//! This module defines the contracts for plugin identity, operations, and construction.
//! It follows the specifications in RFC-00007.

use crate::cancel::{CancelableReceiver, CancelableSender};

/// Named alias for a cancel-aware plugin sender boundary.
///
/// Every `PluginSender<T>` is a [`CancelableSender<T>`], which in turn is a [`Sender<T>`].
pub trait PluginSender<T>: CancelableSender<T> {}
impl<T, S: CancelableSender<T>> PluginSender<T> for S {}

/// Named alias for a cancel-aware plugin receiver boundary.
///
/// Every `PluginReceiver<T>` is a [`CancelableReceiver<T>`], which in turn is a [`Receiver<T>`].
pub trait PluginReceiver<T>: CancelableReceiver<T> {}
impl<T, R: CancelableReceiver<T>> PluginReceiver<T> for R {}

/// Named alias for a cancel-aware plugin receiver boundary.
///
/// Every `PluginReceiver<T>` is a [`CancelableReceiver<T>`], which in turn is a [`Receiver<T>`].
/// Specialized trait for plugin-oriented dataflow operations.
///
/// `PluginOperation` is the fundamental execution unit for the reactive pipeline.
/// It specializes [`FlowOperation`] by fixing the sender to [`PluginSender<Self::Output>`]
/// and adding plugin-specific metadata.
pub trait PluginOperation<S: PluginSender<Self::Output>>:
    crate::operation::FlowOperation<Self::Input, Self::Output, S>
{
    /// The type of data this operation consumes.
    type Input: Send + 'static;

    /// The type of data this operation produces.
    type Output: Send + 'static;
}

/// Macro to register independent operations from external functions.
///
/// This macro generates a struct that implements [`PluginOperation`]
/// by wrapping an external async function.
///
/// # Example
///
/// ```rust
/// use runtime_core::{declare_plugin_operations, result::RuntimeResult, plugin::PluginSender, operation::FlowOperation};
///
/// # struct MockSender;
/// # impl runtime_core::channel::Sender<i32> for MockSender { async fn send(&mut self, _v: i32) -> RuntimeResult<()> { Ok(()) } }
/// # impl runtime_core::cancel::CancelableSender<i32> for MockSender { fn is_cancelled(&self) -> bool { false } }
///
/// async fn add_one(input: i32, output: &mut impl PluginSender<i32>) -> RuntimeResult<()> {
///     output.send(input + 1).await
/// }
///
/// declare_plugin_operations! {
///     AddOneOp => add_one(i32, i32)
/// }
/// ```
#[macro_export]
macro_rules! declare_plugin_operations {
    (
        $(
            $(#[$attr:meta])*
            $struct_name:ident => $op_fn:ident($input_ty:ty, $output_ty:ty)
        )*
    ) => {
        $(
            $(#[$attr])*
            #[non_exhaustive]
            /// Operation struct for the `$op_fn` plugin operation.
            pub struct $struct_name;

            impl<S> $crate::plugin::PluginOperation<S> for $struct_name
            where S: $crate::plugin::PluginSender<$output_ty>
            {
                type Input = $input_ty;
                type Output = $output_ty;
            }

            impl<S> $crate::operation::FlowOperation<$input_ty, $output_ty, S> for $struct_name
            where S: $crate::plugin::PluginSender<$output_ty>
            {
                async fn run(&self, input: $input_ty, output: &mut S) -> $crate::result::RuntimeResult<()> {
                    $op_fn(input, output).await
                }
            }
        )*
    };
}

#[cfg(test)]
#[path = "plugin_test.rs"]
mod tests;
