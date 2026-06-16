//! Shared operation composition utilities.

use runtime_core::{RuntimeResult, cancel::CancelableSender};

pub struct CapturingSender<T> {
    output: Option<T>,
}

impl<T> CapturingSender<T> {
    pub const fn new() -> Self {
        Self { output: None }
    }

    pub fn into_output(self) -> Option<T> {
        self.output
    }
}

impl<T: Send> runtime_core::Sender<T> for CapturingSender<T> {
    async fn send(&mut self, value: T) -> RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl<T: Send> CancelableSender<T> for CapturingSender<T> {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "support_test.rs"]
mod tests;
