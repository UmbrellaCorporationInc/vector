use super::*;
use runtime_core::event::EventListener;
use std::sync::Arc;
use tokio::sync::Mutex;

struct SpyListener {
    received: Arc<Mutex<Vec<u32>>>,
    should_fail: bool,
}

impl EventListener<u32> for SpyListener {
    async fn on_event(&mut self, event: u32) -> RuntimeResult<()> {
        if self.should_fail {
            return Err(runtime_core::RuntimeError::ChannelClosed);
        }
        self.received.lock().await.push(event);
        Ok(())
    }
}

#[tokio::test]
async fn test_tokio_event_emitter_broadcast() -> RuntimeResult<()> {
    let mut emitter = crate::emitter::<u32>();

    let received1 = Arc::new(Mutex::new(Vec::new()));
    let received2 = Arc::new(Mutex::new(Vec::new()));

    emitter
        .register_listener(SpyListener { received: Arc::clone(&received1), should_fail: false })?;

    emitter
        .register_listener(SpyListener { received: Arc::clone(&received2), should_fail: false })?;

    emitter.emit(100).await?;
    emitter.emit(200).await?;

    // Give some time for async fan-out and listener tasks to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert_eq!(*received1.lock().await, vec![100, 200]);
    assert_eq!(*received2.lock().await, vec![100, 200]);

    Ok(())
}

#[tokio::test]
async fn test_tokio_event_emitter_failure_isolation() -> RuntimeResult<()> {
    let mut emitter = crate::emitter::<u32>();

    let received_ok = Arc::new(Mutex::new(Vec::new()));

    // This listener will fail on the first event
    emitter.register_listener(SpyListener {
        received: Arc::new(Mutex::new(Vec::new())),
        should_fail: true,
    })?;

    // This listener should still work
    emitter.register_listener(SpyListener {
        received: Arc::clone(&received_ok),
        should_fail: false,
    })?;

    emitter.emit(42).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    assert_eq!(*received_ok.lock().await, vec![42]);

    Ok(())
}

#[tokio::test]
async fn test_tokio_event_emitter_ordering() -> RuntimeResult<()> {
    let mut emitter = crate::emitter::<u32>();
    let received = Arc::new(Mutex::new(Vec::new()));

    emitter
        .register_listener(SpyListener { received: Arc::clone(&received), should_fail: false })?;

    for i in 0..50 {
        emitter.emit(i).await?;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let expected: Vec<u32> = (0..50).collect();
    assert_eq!(*received.lock().await, expected);

    Ok(())
}
