use super::*;
use crate::RuntimeError;

struct MockListener {
    received: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl EventListener<String> for MockListener {
    async fn on_event(&mut self, event: String) -> RuntimeResult<()> {
        self.received.lock().map_err(|_| RuntimeError::ChannelClosed)?.push(event);
        Ok(())
    }
}

struct MockEmitter {
    listeners_count: usize,
}

impl Sender<String> for MockEmitter {
    async fn send(&mut self, _value: String) -> RuntimeResult<()> {
        Ok(())
    }
}

impl EventEmitter<String> for MockEmitter {
    fn register_listener(
        &mut self,
        _listener: impl EventListener<String> + 'static,
    ) -> RuntimeResult<()> {
        self.listeners_count += 1;
        Ok(())
    }
}

#[test]
fn test_event_listener_registration() -> RuntimeResult<()> {
    let mut emitter = MockEmitter { listeners_count: 0 };
    let listener = MockListener { received: std::sync::Arc::new(std::sync::Mutex::new(vec![])) };
    emitter.register_listener(listener)?;
    assert_eq!(emitter.listeners_count, 1);
    Ok(())
}

#[tokio::test]
async fn test_event_emission_surface() -> RuntimeResult<()> {
    let mut emitter = MockEmitter { listeners_count: 0 };
    // Test emit alias
    emitter.emit("test_event".to_string()).await?;
    Ok(())
}

#[test]
fn test_event_traits_are_send() {
    fn assert_send<T: Send>() {}
    assert_send::<MockEmitter>();
    assert_send::<MockListener>();
}

#[tokio::test]
async fn test_event_emitter_as_sender_substitution() -> RuntimeResult<()> {
    async fn use_as_sender(mut sender: impl Sender<String>) -> RuntimeResult<()> {
        sender.send("substituted".to_string()).await
    }

    let emitter = MockEmitter { listeners_count: 0 };
    use_as_sender(emitter).await?;
    Ok(())
}

#[test]
fn test_control_event_properties() {
    let event = ControlEvent::Cancel;

    // Test Clone
    let cloned = event.clone();
    assert_eq!(event, cloned);

    // Test Debug
    assert_eq!(format!("{event:?}"), "Cancel");

    // Test PartialEq
    assert_eq!(event, ControlEvent::Cancel);
}

#[test]
fn test_control_event_exhaustiveness() {
    let event = ControlEvent::Cancel;
    #[allow(clippy::single_match, clippy::match_same_arms, unreachable_patterns)]
    match event {
        ControlEvent::Cancel => (),
        _ => (), // Required due to #[non_exhaustive]
    }
}

#[tokio::test]
async fn test_control_event_with_emitter() -> RuntimeResult<()> {
    struct ControlEmitter;
    impl Sender<ControlEvent> for ControlEmitter {
        async fn send(&mut self, _value: ControlEvent) -> RuntimeResult<()> {
            Ok(())
        }
    }
    impl EventEmitter<ControlEvent> for ControlEmitter {
        fn register_listener(
            &mut self,
            _listener: impl EventListener<ControlEvent> + 'static,
        ) -> RuntimeResult<()> {
            Ok(())
        }
    }

    let mut emitter = ControlEmitter;
    emitter.emit(ControlEvent::Cancel).await?;
    Ok(())
}

#[test]
fn test_observability_event_properties() {
    let id = "op-1".to_string();
    let event: ObservabilityEvent<String> =
        ObservabilityEvent::OperationStarted { operation_id: id.clone() };

    // Test Clone
    let cloned = event.clone();
    assert_eq!(event, cloned);

    // Test Debug
    assert!(format!("{event:?}").contains("OperationStarted"));
    assert!(format!("{event:?}").contains("op-1"));

    // Test MessageSent
    let msg_event =
        ObservabilityEvent::MessageSent { operation_id: id, payload: "hello".to_string() };
    assert_eq!(msg_event.clone(), msg_event);
}

#[test]
fn test_observability_event_exhaustiveness() {
    let event: ObservabilityEvent<String> =
        ObservabilityEvent::OperationCompleted { operation_id: "op-1".to_string() };
    #[allow(clippy::match_same_arms, unreachable_patterns)]
    match event {
        ObservabilityEvent::OperationStarted { .. } => (),
        ObservabilityEvent::OperationCompleted { .. } => (),
        ObservabilityEvent::MessageSent { .. } => (),
        _ => (), // Required due to #[non_exhaustive]
    }
}

#[test]
fn test_observability_event_conditional_partial_eq() {
    // NonPartialEq type
    #[derive(Debug, Clone)]
    struct NonPartialEq;

    // String implements PartialEq
    let e1 = ObservabilityEvent::MessageSent { operation_id: "1".into(), payload: "a".to_string() };
    let e2 = ObservabilityEvent::MessageSent { operation_id: "1".into(), payload: "a".to_string() };
    assert_eq!(e1, e2);

    let _e3: ObservabilityEvent<NonPartialEq> =
        ObservabilityEvent::OperationStarted { operation_id: "1".into() };
    // assert_eq!(_e3, _e3); // This would fail to compile because NonPartialEq is not PartialEq
}

#[tokio::test]
async fn test_observability_event_with_emitter() -> RuntimeResult<()> {
    struct ObsEmitter;
    impl Sender<ObservabilityEvent<String>> for ObsEmitter {
        async fn send(&mut self, _value: ObservabilityEvent<String>) -> RuntimeResult<()> {
            Ok(())
        }
    }
    impl EventEmitter<ObservabilityEvent<String>> for ObsEmitter {
        fn register_listener(
            &mut self,
            _listener: impl EventListener<ObservabilityEvent<String>> + 'static,
        ) -> RuntimeResult<()> {
            Ok(())
        }
    }

    let mut emitter = ObsEmitter;
    emitter.emit(ObservabilityEvent::OperationStarted { operation_id: "op-1".into() }).await?;
    Ok(())
}
