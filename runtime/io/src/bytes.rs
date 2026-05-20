//! Byte aliases.

/// Standard byte vector alias for IO operations.
pub type Bytes = Vec<u8>;

#[cfg(test)]
#[path = "bytes_test.rs"]
mod tests;
