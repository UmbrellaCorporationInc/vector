//! Type aliases for filesystem operations.

/// Boxed streaming reader used by shell execution handles.
pub type Reader = Box<dyn std::io::Read + Send + Sync>;

#[cfg(test)]
#[path = "types_test.rs"]
mod tests;
