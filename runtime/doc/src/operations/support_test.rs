#![allow(clippy::unwrap_used)]

use runtime_core::{Sender, cancel::CancelableSender};

use super::{CapturingSender, DiscardSender};

#[tokio::test]
async fn test_capturing_sender_keeps_last_output() {
    let mut sender = CapturingSender::new();

    sender.send("first").await.unwrap();
    sender.send("second").await.unwrap();

    assert_eq!(sender.into_output(), Some("second"));
}

#[tokio::test]
async fn test_discard_sender_accepts_output() {
    let mut sender = DiscardSender;

    sender.send("ignored").await.unwrap();

    assert!(!<DiscardSender as CancelableSender<&str>>::is_cancelled(&sender));
}
