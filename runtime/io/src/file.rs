//! File IO boundaries

use crate::{Bytes, IoError, IoPath};
use runtime_core::{Receiver, RuntimeError, RuntimeResult, Sender};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A file-backed receiver that produces bytes.
#[derive(Debug)]
pub struct FileReader {
    file: File,
    buffer_size: usize,
}

impl FileReader {
    /// Open a file for reading.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::File`] if the file cannot be opened.
    pub async fn open(path: &IoPath, buffer_size: usize) -> Result<Self, IoError> {
        let file = File::open(path.as_path()).await.map_err(|e| IoError::File(e.to_string()))?;
        Ok(Self { file, buffer_size })
    }
}

impl Receiver<Bytes> for FileReader {
    async fn recv(&mut self) -> RuntimeResult<Option<Bytes>> {
        let mut buf = vec![0u8; self.buffer_size];
        match self.file.read(&mut buf).await {
            Ok(0) | Err(_) => Ok(None), // EOF or I/O error treated as EOF
            Ok(n) => {
                buf.truncate(n);
                Ok(Some(buf))
            }
        }
    }
}

impl Drop for FileReader {
    fn drop(&mut self) {
        // tokio::fs::File closes automatically
    }
}

/// A file-backed sender that writes bytes.
#[derive(Debug)]
pub struct FileWriter {
    file: Option<File>,
    buffer: Vec<u8>,
    buffer_size: usize,
}

impl FileWriter {
    /// Create a file for writing. Ensures the parent directory exists.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::File`] if the file or its parent directory cannot be created.
    pub async fn create(path: &IoPath, buffer_size: usize) -> Result<Self, IoError> {
        if let Some(parent) = path.as_path().parent()
            && !parent.exists()
        {
            tokio::fs::create_dir_all(parent).await.map_err(|e| IoError::File(e.to_string()))?;
        }
        let file = File::create(path.as_path()).await.map_err(|e| IoError::File(e.to_string()))?;
        Ok(Self { file: Some(file), buffer: Vec::new(), buffer_size })
    }

    /// Flush the internal buffer to the file.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::File`] if writing to the file fails.
    pub async fn flush(&mut self) -> Result<(), IoError> {
        if !self.buffer.is_empty() {
            if let Some(file) = &mut self.file {
                file.write_all(&self.buffer).await.map_err(|e| IoError::File(e.to_string()))?;
            }
            self.buffer.clear();
        }
        Ok(())
    }

    /// Close the file explicitly, flushing first.
    ///
    /// # Errors
    ///
    /// Returns [`IoError::File`] if the final flush fails.
    pub async fn close(mut self) -> Result<(), IoError> {
        self.flush().await?;
        self.file.take();
        Ok(())
    }
}

impl Sender<Bytes> for FileWriter {
    async fn send(&mut self, mut value: Bytes) -> RuntimeResult<()> {
        self.buffer.append(&mut value);
        if self.buffer.len() >= self.buffer_size {
            if let Some(file) = &mut self.file {
                file.write_all(&self.buffer)
                    .await
                    .map_err(|e| RuntimeError::operation(format!("file write failed: {e}")))?;
            }
            self.buffer.clear();
        }
        Ok(())
    }
}

/// Convenience helper to read a full file as bytes using `FileReader`.
///
/// # Errors
/// Returns [`IoError::File`] if the file cannot be opened.
pub async fn read_file_bytes(path: &IoPath) -> Result<Bytes, IoError> {
    let mut reader = FileReader::open(path, 8192).await?;
    let mut buffer = Vec::new();
    while let Ok(Some(mut chunk)) = reader.recv().await {
        buffer.append(&mut chunk);
    }
    Ok(buffer)
}

/// Convenience helper to write full bytes to a file using `FileWriter`.
/// Ensures the parent directory exists before writing.
///
/// # Errors
/// Returns [`IoError::File`] if the file cannot be created or written.
pub async fn write_file_bytes(path: &IoPath, data: Bytes) -> Result<(), IoError> {
    let mut writer = FileWriter::create(path, 8192).await?;
    writer.send(data).await.map_err(|_| IoError::File("write failed".into()))?;
    writer.close().await?;
    Ok(())
}

/// Create all directories in the given path.
///
/// # Errors
/// Returns [`IoError::File`] if directory creation fails.
pub async fn create_dir_all(path: &IoPath) -> Result<(), IoError> {
    tokio::fs::create_dir_all(path.as_ref()).await.map_err(|e| IoError::File(e.to_string()))
}
#[cfg(test)]
#[path = "file_test.rs"]
mod tests;
