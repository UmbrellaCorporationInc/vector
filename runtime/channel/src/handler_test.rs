use runtime_core::CancelHandler;

use super::{CancelState, TokioCancelHandler};

#[test]
fn not_cancelled_before_cancel_called() {
    let (state, _rx) = CancelState::new();
    let handler = TokioCancelHandler::new(state);
    assert!(!handler.is_cancelled());
}

#[test]
fn cancel_sets_cancelled_flag() {
    let (state, _rx) = CancelState::new();
    let handler = TokioCancelHandler::new(state);
    handler.cancel();
    assert!(handler.is_cancelled());
}

#[test]
fn cancel_sends_true_on_watch_channel() {
    let (state, rx) = CancelState::new();
    let handler = TokioCancelHandler::new(state);
    handler.cancel();
    assert!(*rx.borrow());
}

#[test]
fn handler_is_send() {
    fn requires_send<T: Send>(_: T) {}
    let (state, _rx) = CancelState::new();
    requires_send(TokioCancelHandler::new(state));
}
