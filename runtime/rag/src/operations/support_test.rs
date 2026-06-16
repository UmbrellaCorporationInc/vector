#![allow(clippy::unwrap_used)]

use super::*;
use runtime_core::channel::Sender;

#[tokio::test]
async fn capturing_sender_returns_none_before_any_send() {
    let sender = CapturingSender::<u32>::new();
    assert!(sender.into_output().is_none());
}

#[tokio::test]
async fn capturing_sender_returns_sent_value() {
    let mut sender = CapturingSender::new();
    sender.send(42u32).await.unwrap();
    assert_eq!(sender.into_output(), Some(42));
}

#[tokio::test]
async fn capturing_sender_retains_last_sent_value() {
    let mut sender = CapturingSender::new();
    sender.send(1u32).await.unwrap();
    sender.send(2u32).await.unwrap();
    assert_eq!(sender.into_output(), Some(2));
}

#[test]
fn capturing_sender_is_never_cancelled() {
    use runtime_core::cancel::CancelableSender;
    let sender = CapturingSender::<u32>::new();
    assert!(!sender.is_cancelled());
}
