//! Memory IO boundaries

use crate::Bytes;
use runtime_core::{Receiver, RuntimeResult, Sender};

/// A memory-backed receiver that produces bytes in chunks.
#[derive(Debug)]
pub struct MemReader {
    data: Bytes,
    position: usize,
    chunk_size: usize,
}

impl MemReader {
    /// Create a new memory reader from the given bytes.
    #[must_use]
    pub fn new(data: impl Into<Bytes>, chunk_size: usize) -> Self {
        Self {
            data: data.into(),
            position: 0,
            chunk_size: chunk_size.max(1), // Prevent infinite loops of 0-length chunks
        }
    }
}

impl Receiver<Bytes> for MemReader {
    async fn recv(&mut self) -> RuntimeResult<Option<Bytes>> {
        if self.position >= self.data.len() {
            return Ok(None);
        }
        let end = (self.position + self.chunk_size).min(self.data.len());
        let chunk = self.data[self.position..end].to_vec();
        self.position = end;
        Ok(Some(chunk))
    }
}

/// A memory-backed sender that collects bytes.
#[derive(Debug, Default)]
pub struct MemWriter {
    buffer: Bytes,
}

impl MemWriter {
    /// Create a new memory writer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new memory writer with a specified initial capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { buffer: Vec::with_capacity(capacity) }
    }

    /// Consume the writer and return the collected bytes.
    #[must_use]
    pub fn into_inner(self) -> Bytes {
        self.buffer
    }

    /// Get a reference to the collected bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

impl Sender<Bytes> for MemWriter {
    async fn send(&mut self, mut value: Bytes) -> RuntimeResult<()> {
        self.buffer.append(&mut value);
        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_test.rs"]
mod tests;
