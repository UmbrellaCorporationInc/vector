//! In-process memory backend. Zero disk I/O — intended for tests and
//! environments where persistence is not required.

use crate::{Directory, File, FileList, FsError, UnixTimestamp, Writer};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// File entry stored in the in-memory backend, pairing raw bytes with a modification timestamp.
#[derive(Debug)]
struct FileData {
    content: Vec<u8>,
    last_modified: UnixTimestamp,
}

impl FileData {
    /// Constructs a new [`FileData`] stamping `last_modified` with the current wall-clock time.
    /// Falls back to `0` on the (structurally impossible) case where `SystemTime::now()`
    /// precedes the Unix epoch.
    fn new(bytes: Vec<u8>) -> Self {
        let last_modified = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Self { content: bytes, last_modified }
    }
}

type FileStore = HashMap<String, FileData>;
type Store = Arc<Mutex<FileStore>>;

/// In-memory [`Directory`] backend and primary entry point.
///
/// All data is stored in a shared `HashMap<String, FileData>` protected by a `Mutex`.
/// Cloning any handle derived from this directory is `O(1)` — the underlying store
/// is not copied. Handles with different prefixes all share the same store.
#[derive(Debug, Default, Clone)]
pub struct MemoryDir {
    store: Store,
    prefix: String,
}

impl MemoryDir {
    /// Bootstraps an empty in-memory directory rooted at the implicit root.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a new handle into the same backing store, rooted at `path` relative
    /// to this directory's current prefix.
    ///
    /// Cloning is `O(1)` — the underlying store is shared via `Arc`.
    #[must_use]
    pub fn subdir(&self, path: &str) -> Self {
        let prefix = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{path}", self.prefix)
        };
        Self { store: Arc::clone(&self.store), prefix }
    }
}

impl Directory for MemoryDir {
    fn create_dir(&self, _path: &str) -> Result<(), FsError> {
        // The memory backend is key-based — directories have no physical representation.
        // Idempotency is guaranteed structurally: creating a path twice is a no-op.
        Ok(())
    }

    fn get_file(&self, path: &str) -> Result<Box<dyn File>, FsError> {
        let key = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{path}", self.prefix)
        };
        Ok(Box::new(MemoryFile { store: Arc::clone(&self.store), path: key }))
    }

    fn list_files(&self) -> Result<FileList, FsError> {
        let strip_len = if self.prefix.is_empty() { 0 } else { self.prefix.len() + 1 };

        // Collect keys while holding the lock, then release it before returning the iterator.
        let keys: Vec<String> = {
            let store =
                self.store.lock().map_err(|_| FsError::Io("store lock poisoned".to_string()))?;
            store
                .keys()
                .filter(|key| {
                    self.prefix.is_empty()
                        || (key.starts_with(&self.prefix)
                            && key[self.prefix.len()..].starts_with('/'))
                })
                .cloned()
                .collect()
        };

        Ok(Box::new(keys.into_iter().map(move |key| Ok(key[strip_len..].to_string()))))
    }
}

/// Streaming writer for the in-memory backend.
///
/// Buffers bytes locally during `write` calls (no lock held). On `flush`, acquires
/// the store lock and replaces the entry with the buffered contents. `Drop` calls
/// `flush` as best-effort — errors are suppressed.
struct MemoryWriter {
    store: Store,
    path: String,
    buffer: Vec<u8>,
}

impl std::io::Write for MemoryWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.store
            .lock()
            .map_err(|_| std::io::Error::other("store lock poisoned"))?
            .insert(self.path.clone(), FileData::new(self.buffer.clone()));
        Ok(())
    }
}

impl Drop for MemoryWriter {
    fn drop(&mut self) {
        let _ = std::io::Write::flush(self);
    }
}

/// In-memory [`File`] handle.
#[derive(Debug)]
struct MemoryFile {
    store: Store,
    path: String,
}

impl File for MemoryFile {
    fn write_text(&self, content: &str) -> Result<(), FsError> {
        self.write_bytes(content.as_bytes())
    }

    fn write_bytes(&self, bytes: &[u8]) -> Result<(), FsError> {
        self.store
            .lock()
            .map_err(|_| FsError::Io("store lock poisoned".to_string()))?
            .insert(self.path.clone(), FileData::new(bytes.to_vec()));
        Ok(())
    }

    fn read_text(&self) -> Result<String, FsError> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).map_err(|e| FsError::Io(e.to_string()))
    }

    fn read_bytes(&self) -> Result<Vec<u8>, FsError> {
        self.store
            .lock()
            .map_err(|_| FsError::Io("store lock poisoned".to_string()))?
            .get(&self.path)
            .map(|data| data.content.clone())
            .ok_or_else(|| FsError::NotFound(self.path.clone()))
    }

    fn read_reader(&self) -> Result<crate::Reader, FsError> {
        let bytes = self.read_bytes()?;
        Ok(Box::new(std::io::Cursor::new(bytes)))
    }

    fn write_writer(&self) -> Result<Writer, FsError> {
        Ok(Box::new(MemoryWriter {
            store: Arc::clone(&self.store),
            path: self.path.clone(),
            buffer: Vec::new(),
        }))
    }

    fn delete(&self) -> Result<(), FsError> {
        self.store
            .lock()
            .map_err(|_| FsError::Io("store lock poisoned".to_string()))?
            .remove(&self.path)
            .map(|_| ())
            .ok_or_else(|| FsError::NotFound(self.path.clone()))
    }

    fn exists(&self) -> bool {
        self.store.lock().is_ok_and(|store| store.contains_key(&self.path))
    }

    fn last_modified(&self) -> Result<UnixTimestamp, FsError> {
        self.store
            .lock()
            .map_err(|_| FsError::Io("store lock poisoned".to_string()))?
            .get(&self.path)
            .map(|data| data.last_modified)
            .ok_or_else(|| FsError::NotFound(self.path.clone()))
    }
}

#[cfg(test)]
#[path = "memory_test.rs"]
mod tests;
