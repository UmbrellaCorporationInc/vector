use super::ChannelConfig;

#[test]
fn default_capacity_is_non_zero() {
    const { assert!(ChannelConfig::DEFAULT_CAPACITY > 0) }
}

#[test]
fn default_config_uses_default_capacity() {
    assert_eq!(ChannelConfig::default().capacity(), ChannelConfig::DEFAULT_CAPACITY);
}

#[test]
fn new_stores_supplied_capacity() {
    let cfg = ChannelConfig::new(32);
    assert_eq!(cfg.capacity(), 32);
}

#[test]
fn new_capacity_one_is_valid() {
    let cfg = ChannelConfig::new(1);
    assert_eq!(cfg.capacity(), 1);
}

#[test]
#[should_panic(expected = "channel capacity must be greater than zero")]
fn new_zero_capacity_panics() {
    let _ = ChannelConfig::new(0);
}
