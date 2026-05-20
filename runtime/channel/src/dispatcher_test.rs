#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex};

use runtime_core::CancelHandler;
use runtime_core::RuntimeError;
use runtime_core::cancel::CancelableReceiver;
use runtime_core::channel::{Receiver, Sender};
use runtime_core::event::{EventListener, ObservabilityEvent};
use runtime_core::operation::FlowOperation;
use runtime_core::plugin::{PluginOperation, PluginReceiver, PluginSender};
use runtime_core::result::RuntimeResult;

use super::{InstrumentedSender, PluginDispatcher};

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime build failed")
}

// ---------------------------------------------------------------------------
// Operation fixtures
// ---------------------------------------------------------------------------

struct EchoOp;

impl PluginOperation<InstrumentedSender<u32>> for EchoOp {
    type Input = u32;
    type Output = u32;
}

impl FlowOperation<u32, u32, InstrumentedSender<u32>> for EchoOp {
    async fn run(&self, input: u32, output: &mut InstrumentedSender<u32>) -> RuntimeResult<()> {
        output.send(input).await
    }
}

struct MultiSendOp;

impl PluginOperation<InstrumentedSender<u32>> for MultiSendOp {
    type Input = ();
    type Output = u32;
}

impl FlowOperation<(), u32, InstrumentedSender<u32>> for MultiSendOp {
    async fn run(&self, _input: (), output: &mut InstrumentedSender<u32>) -> RuntimeResult<()> {
        output.send(1).await?;
        output.send(2).await?;
        output.send(3).await
    }
}

struct NeverSendsOp;

impl PluginOperation<InstrumentedSender<u32>> for NeverSendsOp {
    type Input = ();
    type Output = u32;
}

impl FlowOperation<(), u32, InstrumentedSender<u32>> for NeverSendsOp {
    async fn run(&self, _input: (), _output: &mut InstrumentedSender<u32>) -> RuntimeResult<()> {
        tokio::task::yield_now().await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Listener fixtures
// ---------------------------------------------------------------------------

struct NoopListener;

impl EventListener<ObservabilityEvent<u32>> for NoopListener {
    async fn on_event(&mut self, _event: ObservabilityEvent<u32>) -> RuntimeResult<()> {
        Ok(())
    }
}

/// Collects received events into a shared vec for assertion.
#[derive(Clone)]
struct SpyListener {
    events: Arc<Mutex<Vec<ObservabilityEvent<u32>>>>,
}

impl SpyListener {
    fn new() -> (Self, Arc<Mutex<Vec<ObservabilityEvent<u32>>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        (Self { events: Arc::clone(&events) }, events)
    }
}

impl EventListener<ObservabilityEvent<u32>> for SpyListener {
    async fn on_event(&mut self, event: ObservabilityEvent<u32>) -> RuntimeResult<()> {
        self.events.lock().expect("lock poisoned").push(event);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Phase A: builder configuration tests
// ---------------------------------------------------------------------------

#[test]
fn dispatcher_accepts_operation_at_construction() {
    let _d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp);
}

#[test]
fn dispatcher_input_is_none_before_supply() {
    let d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp);
    assert!(d.input.is_none());
}

#[test]
fn dispatcher_input_is_some_after_supply() {
    let d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp).input(42_u32);
    assert_eq!(d.input, Some(42_u32));
}

#[test]
fn dispatcher_input_can_be_overwritten() {
    let d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp).input(1_u32).input(99_u32);
    assert_eq!(d.input, Some(99_u32));
}

#[test]
fn dispatcher_has_no_listeners_by_default() {
    // Verify the builder starts with no listeners (emitter has no registered senders).
    // We exercise this indirectly through build() + unobserved send in Phase B/C tests.
    let _d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp);
}

#[test]
fn dispatcher_observe_accepts_listener_without_error() {
    rt().block_on(async {
        let _d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp).observe(NoopListener);
    });
}

#[test]
fn dispatcher_observe_accepts_multiple_listeners() {
    rt().block_on(async {
        let _d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .observe(NoopListener)
            .observe(NoopListener)
            .observe(NoopListener);
    });
}

#[test]
fn dispatcher_observe_is_optional() {
    let _d = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp).input(7_u32);
}

#[test]
fn dispatcher_builder_is_generic_over_plugin_operation() {
    rt().block_on(async {
        struct StringOp;

        impl PluginOperation<InstrumentedSender<String>> for StringOp {
            type Input = String;
            type Output = String;
        }

        impl FlowOperation<String, String, InstrumentedSender<String>> for StringOp {
            async fn run(
                &self,
                input: String,
                output: &mut InstrumentedSender<String>,
            ) -> RuntimeResult<()> {
                output.send(input).await
            }
        }

        struct StringListener;
        impl EventListener<ObservabilityEvent<String>> for StringListener {
            async fn on_event(&mut self, _event: ObservabilityEvent<String>) -> RuntimeResult<()> {
                Ok(())
            }
        }

        let _d = PluginDispatcher::<StringOp, String, String>::new(StringOp)
            .input("hello".to_string())
            .observe(StringListener);
    });
}

#[test]
fn dispatcher_satisfies_plugin_sender_bound_on_instrumented_sender() {
    fn assert_plugin_sender<T, S: PluginSender<T>>() {}
    assert_plugin_sender::<u32, InstrumentedSender<u32>>();
}

// ---------------------------------------------------------------------------
// Phase B: connected cancelable output channel wiring
// ---------------------------------------------------------------------------

#[test]
fn build_fails_without_input() {
    let result = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp).build();
    assert!(result.is_err());
}

#[test]
fn build_returns_cancel_handler_and_receiver() {
    rt().block_on(async {
        let (handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(1_u32)
            .build()
            .expect("build failed");
        assert!(!handler.is_cancelled());
        let _ = rx.recv().await;
    });
}

#[test]
fn build_receiver_delivers_operation_output() {
    rt().block_on(async {
        let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(42_u32)
            .build()
            .expect("build failed");
        assert_eq!(rx.recv().await, Ok(Some(42_u32)));
    });
}

#[test]
fn build_receiver_is_connected_to_operation_sender() {
    rt().block_on(async {
        let (_handler, mut rx) = PluginDispatcher::<MultiSendOp, (), u32>::new(MultiSendOp)
            .input(())
            .build()
            .expect("build failed");
        assert_eq!(rx.recv().await, Ok(Some(1_u32)));
        assert_eq!(rx.recv().await, Ok(Some(2_u32)));
        assert_eq!(rx.recv().await, Ok(Some(3_u32)));
    });
}

#[test]
fn build_cancel_handler_cancels_both_endpoints() {
    rt().block_on(async {
        let (handler, rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(1_u32)
            .build()
            .expect("build failed");
        handler.cancel();
        assert!(handler.is_cancelled());
        assert!(rx.is_cancelled());
    });
}

#[test]
fn build_receiver_returns_none_after_cancel() {
    rt().block_on(async {
        let (handler, mut rx) = PluginDispatcher::<NeverSendsOp, (), u32>::new(NeverSendsOp)
            .input(())
            .build()
            .expect("build failed");
        handler.cancel();
        assert_eq!(rx.recv().await, Ok(None));
    });
}

#[test]
fn build_receiver_satisfies_plugin_receiver_bound() {
    fn assert_plugin_receiver<T, R: PluginReceiver<T>>(_: &mut R) {}
    rt().block_on(async {
        let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(0_u32)
            .build()
            .expect("build failed");
        assert_plugin_receiver::<u32, _>(&mut rx);
    });
}

#[test]
fn build_cancel_handler_satisfies_cancel_handler_bound() {
    fn assert_cancel_handler<H: CancelHandler>(_: &H) {}
    rt().block_on(async {
        let (handler, _rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(0_u32)
            .build()
            .expect("build failed");
        assert_cancel_handler(&handler);
    });
}

// ---------------------------------------------------------------------------
// Phase C: sender observability instrumentation
// ---------------------------------------------------------------------------

#[test]
fn unobserved_build_delivers_output_without_listener() {
    // No observe() call — InstrumentedSender with empty emitter must still deliver.
    rt().block_on(async {
        let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(7_u32)
            .build()
            .expect("build failed");
        assert_eq!(rx.recv().await, Ok(Some(7_u32)));
    });
}

#[tokio::test]
async fn observed_sender_emits_message_sent_event() {
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(99_u32)
        .observe(spy)
        .build()
        .expect("build failed");

    // Drain the output so the operation task completes.
    assert_eq!(rx.recv().await, Ok(Some(99_u32)));

    // Yield to allow the listener task to process the event.
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");
    let sent: Vec<_> =
        captured.iter().filter(|e| matches!(e, ObservabilityEvent::MessageSent { .. })).collect();
    assert_eq!(sent.len(), 1);
    assert!(matches!(
        sent[0],
        ObservabilityEvent::MessageSent { payload, .. } if *payload == 99_u32
    ));
    drop(captured);
}

#[tokio::test]
async fn observed_sender_emits_one_event_per_send() {
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<MultiSendOp, (), u32>::new(MultiSendOp)
        .input(())
        .observe(spy)
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(1_u32)));
    assert_eq!(rx.recv().await, Ok(Some(2_u32)));
    assert_eq!(rx.recv().await, Ok(Some(3_u32)));

    // Allow listener tasks to process all events.
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");
    let sent_count =
        captured.iter().filter(|e| matches!(e, ObservabilityEvent::MessageSent { .. })).count();
    assert_eq!(sent_count, 3);
    drop(captured);
}

#[tokio::test]
async fn multiple_listeners_each_receive_message_sent_event() {
    let (spy1, events1) = SpyListener::new();
    let (spy2, events2) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(5_u32)
        .observe(spy1)
        .observe(spy2)
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(5_u32)));
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let count1 = events1
        .lock()
        .expect("lock poisoned")
        .iter()
        .filter(|e| matches!(e, ObservabilityEvent::MessageSent { .. }))
        .count();
    let count2 = events2
        .lock()
        .expect("lock poisoned")
        .iter()
        .filter(|e| matches!(e, ObservabilityEvent::MessageSent { .. }))
        .count();
    assert_eq!(count1, 1);
    assert_eq!(count2, 1);
}

#[test]
fn instrumented_sender_satisfies_plugin_sender_bound() {
    fn assert_plugin_sender<T, S: PluginSender<T>>() {}
    assert_plugin_sender::<u32, InstrumentedSender<u32>>();
}

#[test]
fn instrumented_sender_is_cancelled_delegates_to_inner() {
    rt().block_on(async {
        let (handler, rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
            .input(0_u32)
            .build()
            .expect("build failed");
        assert!(!rx.is_cancelled());
        handler.cancel();
        assert!(rx.is_cancelled());
    });
}

// ---------------------------------------------------------------------------
// Phase D: dispatcher execution wiring — lifecycle observability
// ---------------------------------------------------------------------------

#[tokio::test]
async fn build_emits_operation_started_event() {
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(1_u32)
        .observe(spy)
        .build()
        .expect("build failed");

    let _ = rx.recv().await;
    // Allow listener task to process the events.
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");
    let has_started =
        captured.iter().any(|e| matches!(e, ObservabilityEvent::OperationStarted { .. }));
    assert!(has_started, "expected OperationStarted event");
    drop(captured);
}

#[tokio::test]
async fn build_emits_operation_completed_event() {
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(1_u32)
        .observe(spy)
        .build()
        .expect("build failed");

    let _ = rx.recv().await;
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");
    let has_completed =
        captured.iter().any(|e| matches!(e, ObservabilityEvent::OperationCompleted { .. }));
    assert!(has_completed, "expected OperationCompleted event");
    drop(captured);
}

#[tokio::test]
async fn build_lifecycle_events_share_operation_id_with_message_sent() {
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(7_u32)
        .observe(spy)
        .build()
        .expect("build failed");

    let _ = rx.recv().await;
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");

    let started_id = captured.iter().find_map(|e| {
        if let ObservabilityEvent::OperationStarted { operation_id } = e {
            Some(operation_id.clone())
        } else {
            None
        }
    });
    let completed_id = captured.iter().find_map(|e| {
        if let ObservabilityEvent::OperationCompleted { operation_id } = e {
            Some(operation_id.clone())
        } else {
            None
        }
    });
    let sent_id = captured.iter().find_map(|e| {
        if let ObservabilityEvent::MessageSent { operation_id, .. } = e {
            Some(operation_id.clone())
        } else {
            None
        }
    });

    assert!(started_id.is_some(), "expected OperationStarted");
    assert!(completed_id.is_some(), "expected OperationCompleted");
    assert!(sent_id.is_some(), "expected MessageSent");
    assert_eq!(started_id, completed_id, "operation_id must match across lifecycle events");
    assert_eq!(started_id, sent_id, "operation_id must match between lifecycle and MessageSent");
    drop(captured);
}

#[tokio::test]
async fn build_without_listeners_still_executes_operation() {
    // Lifecycle events are only emitted when listeners are registered.
    // Without listeners the operation must still run and deliver output.
    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(55_u32)
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(55_u32)));
}

#[tokio::test]
async fn build_lifecycle_does_not_affect_plugin_operation_contract() {
    // PluginOperation::run receives only the sender — no lifecycle arguments.
    // This test confirms operation output is correct regardless of lifecycle wiring.
    let (_handler, mut rx) = PluginDispatcher::<MultiSendOp, (), u32>::new(MultiSendOp)
        .input(())
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(1_u32)));
    assert_eq!(rx.recv().await, Ok(Some(2_u32)));
    assert_eq!(rx.recv().await, Ok(Some(3_u32)));
}

// ---------------------------------------------------------------------------
// Phase E: failure behavior
// ---------------------------------------------------------------------------

struct FailingOp;

impl PluginOperation<InstrumentedSender<u32>> for FailingOp {
    type Input = ();
    type Output = u32;
}

impl FlowOperation<(), u32, InstrumentedSender<u32>> for FailingOp {
    async fn run(&self, _input: (), _output: &mut InstrumentedSender<u32>) -> RuntimeResult<()> {
        Err(RuntimeError::operation("intentional test failure"))
    }
}

/// Listener that always returns an error on the first event it receives.
struct FailingListener;

impl EventListener<ObservabilityEvent<u32>> for FailingListener {
    async fn on_event(&mut self, _event: ObservabilityEvent<u32>) -> RuntimeResult<()> {
        Err(RuntimeError::operation("intentional listener failure"))
    }
}

#[tokio::test]
async fn operation_failure_surfaces_error_through_channel() {
    // When the operation returns an error, the dispatcher sends it through the channel.
    // The receiver must yield Err(e) with the operation's message, not Ok(None).
    let (_handler, mut rx) = PluginDispatcher::<FailingOp, (), u32>::new(FailingOp)
        .input(())
        .build()
        .expect("build failed");

    let result = rx.recv().await;
    assert!(result.is_err(), "expected Err from a failing operation; got: {result:?}");
    let msg = result.expect_err("checked above").to_string();
    assert!(
        msg.contains("intentional test failure"),
        "error message must carry the operation message; got: {msg}"
    );
}

#[tokio::test]
async fn operation_failure_does_not_surface_to_caller_via_build() {
    // build() itself must succeed — the error happens inside the spawned task.
    let result = PluginDispatcher::<FailingOp, (), u32>::new(FailingOp).input(()).build();

    assert!(result.is_ok());
}

#[tokio::test]
async fn listener_failure_does_not_fail_output_publication() {
    // A listener that always errors must not prevent values from reaching the receiver.
    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(42_u32)
        .observe(FailingListener)
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(42_u32)));
}

#[tokio::test]
async fn failing_listener_does_not_affect_other_listeners() {
    // A listener that errors must not prevent a healthy listener from receiving events.
    let (spy, events) = SpyListener::new();

    let (_handler, mut rx) = PluginDispatcher::<EchoOp, u32, u32>::new(EchoOp)
        .input(7_u32)
        .observe(FailingListener)
        .observe(spy)
        .build()
        .expect("build failed");

    assert_eq!(rx.recv().await, Ok(Some(7_u32)));
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    let captured = events.lock().expect("lock poisoned");
    let has_message_sent =
        captured.iter().any(|e| matches!(e, ObservabilityEvent::MessageSent { .. }));
    assert!(has_message_sent, "healthy listener must still receive MessageSent");
    drop(captured);
}

#[tokio::test]
async fn operation_failure_with_observer_surfaces_error_through_channel() {
    // Operation failure must surface the error through the channel even when listeners are registered.
    let (_handler, mut rx) = PluginDispatcher::<FailingOp, (), u32>::new(FailingOp)
        .input(())
        .observe(NoopListener)
        .build()
        .expect("build failed");

    assert!(rx.recv().await.is_err(), "expected Err from a failing operation with observer");
}
