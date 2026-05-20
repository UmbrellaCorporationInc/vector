//! Channel capacity configuration for `runtime-channel`.
//!
//! [`ChannelConfig`] supplies the bounded channel capacity used by the standard
//! Tokio-backed factories. Capacity is not hard-coded — callers either accept
//! [`ChannelConfig::default`] or construct a custom value via [`ChannelConfig::new`].

/// Configuration for the standard bounded channel.
///
/// Controls the capacity of the underlying `tokio::sync::mpsc` channel.
/// A capacity of `1` is the minimum; higher values allow the sender to queue
/// messages without waiting for the receiver to consume them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelConfig {
    capacity: usize,
}

impl ChannelConfig {
    /// Default bounded capacity used when no explicit configuration is supplied.
    pub const DEFAULT_CAPACITY: usize = 64;

    /// Construct a [`ChannelConfig`] with the given `capacity`.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero, because a zero-capacity bounded channel
    /// cannot accept any messages.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "channel capacity must be greater than zero");
        Self { capacity }
    }

    /// Returns the configured channel capacity.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self { capacity: Self::DEFAULT_CAPACITY }
    }
}

#[cfg(test)]
#[path = "config_test.rs"]
mod tests;
