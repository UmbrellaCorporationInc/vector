#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn unix_timestamp_is_u64() {
    let ts: UnixTimestamp = 1_234_567_890;
    assert_eq!(ts, 1_234_567_890_u64);
}

#[test]
fn type_aliases_compile() {
    // Verify type aliases are correctly defined
}
