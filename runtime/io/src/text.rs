//! Text adapters

use crate::Bytes;
use runtime_core::{Encoding, Receiver, RuntimeResult, Sender};

/// A text reader adapter that converts bytes to UTF-8 strings.
#[derive(Debug)]
pub struct TextReader<R> {
    inner: R,
    buffer: Bytes,
    buffer_size: usize,
}

impl<R: Receiver<Bytes> + Send + Unpin> TextReader<R> {
    /// Create a new text reader from a byte receiver.
    #[must_use]
    pub fn new(inner: R, buffer_size: usize) -> Self {
        Self { inner, buffer: Vec::new(), buffer_size: buffer_size.max(1) }
    }
}

impl<R: Receiver<Bytes> + Send + Unpin> Receiver<String> for TextReader<R> {
    async fn recv(&mut self) -> RuntimeResult<Option<String>> {
        loop {
            if !self.buffer.is_empty() {
                match std::str::from_utf8(&self.buffer) {
                    Ok(text) => {
                        let mut yield_len = self.buffer.len().min(self.buffer_size);
                        while yield_len > 0 && !text.is_char_boundary(yield_len) {
                            yield_len -= 1;
                        }
                        if yield_len == 0 {
                            if let Some(c) = text.chars().next() {
                                yield_len = c.len_utf8();
                            } else {
                                return Ok(None);
                            }
                        }
                        let chunk = self.buffer.drain(..yield_len).collect::<Vec<_>>();
                        if let Ok(result) = Encoding::decode(&chunk) {
                            return Ok(Some(result));
                        }
                        return Ok(None);
                    }
                    Err(err) => {
                        let valid_len = err.valid_up_to();
                        if valid_len > 0 {
                            if let Ok(text) = std::str::from_utf8(&self.buffer[..valid_len]) {
                                let mut yield_len = valid_len.min(self.buffer_size);
                                while yield_len > 0 && !text.is_char_boundary(yield_len) {
                                    yield_len -= 1;
                                }
                                if yield_len == 0 {
                                    if let Some(c) = text.chars().next() {
                                        yield_len = c.len_utf8();
                                    } else {
                                        return Ok(None);
                                    }
                                }
                                let chunk = self.buffer.drain(..yield_len).collect::<Vec<_>>();
                                if let Ok(result) = Encoding::decode(&chunk) {
                                    return Ok(Some(result));
                                }
                            }
                            return Ok(None);
                        }

                        if err.error_len().is_some() {
                            // Invalid UTF-8 sequence, abort
                            self.buffer.clear();
                            return Ok(None);
                        }

                        // Incomplete sequence, need more data
                    }
                }
            }

            match self.inner.recv().await {
                Ok(Some(mut chunk)) => {
                    self.buffer.append(&mut chunk);
                }
                Ok(None) => {
                    // EOF
                    if !self.buffer.is_empty() {
                        self.buffer.clear();
                    }
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// A text writer adapter that converts UTF-8 strings to bytes.
#[derive(Debug)]
pub struct TextWriter<S> {
    inner: S,
    buffer: Bytes,
    buffer_size: usize,
}

impl<S: Sender<Bytes> + Send + Unpin> TextWriter<S> {
    /// Create a new text writer from a byte sender.
    #[must_use]
    pub fn new(inner: S, buffer_size: usize) -> Self {
        Self { inner, buffer: Vec::new(), buffer_size: buffer_size.max(1) }
    }

    /// Flush remaining bytes to the inner sender.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] if the inner sender fails.
    pub async fn flush(&mut self) -> RuntimeResult<()> {
        if !self.buffer.is_empty() {
            let chunk = std::mem::take(&mut self.buffer);
            self.inner.send(chunk).await?;
        }
        Ok(())
    }

    /// Close the writer and flush.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::ChannelClosed`] if the flush fails.
    pub async fn close(mut self) -> RuntimeResult<()> {
        self.flush().await
    }

    /// Consume the writer and return the inner sender.
    #[must_use]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Sender<Bytes> + Send + Unpin> Sender<String> for TextWriter<S> {
    async fn send(&mut self, text: String) -> RuntimeResult<()> {
        let mut encoded = Encoding::encode(&text);
        self.buffer.append(&mut encoded);

        while self.buffer.len() >= self.buffer_size {
            let chunk = self.buffer.drain(..self.buffer_size).collect();
            self.inner.send(chunk).await?;
        }
        Ok(())
    }
}

use crate::error::IoError;
use crate::file::{FileReader, FileWriter};
use crate::path::IoPath;

/// Convenience helper to read a full file as a UTF-8 string using `TextReader`.
///
/// # Errors
/// Returns [`IoError::File`] if the file cannot be opened.
pub async fn read_file_text(path: &IoPath) -> Result<String, IoError> {
    let reader = FileReader::open(path, 8192).await?;
    let mut text_reader = TextReader::new(reader, 8192);
    let mut output = String::new();
    while let Ok(Some(chunk)) = text_reader.recv().await {
        output.push_str(&chunk);
    }
    Ok(output)
}

/// Convenience helper to write a UTF-8 string to a file using `TextWriter`.
///
/// # Errors
/// Returns [`IoError::File`] if the file cannot be created or written.
pub async fn write_file_text(path: &IoPath, text: String) -> Result<(), IoError> {
    let writer = FileWriter::create(path, 8192).await?;
    let mut text_writer = TextWriter::new(writer, 8192);
    text_writer.send(text).await.map_err(|_| IoError::File("write failed".into()))?;
    text_writer.flush().await.map_err(|_| IoError::File("flush failed".into()))?;
    let writer = text_writer.into_inner();
    writer.close().await?;
    Ok(())
}

#[cfg(test)]
#[path = "text_test.rs"]
mod tests;
