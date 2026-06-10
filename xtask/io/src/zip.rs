//! ZIP-backed filesystem backend.
//!
//! Provides read-only access to compressed archives (.babel, .doc) by treating
//! them as path-scoped virtual directories. Employs independent archive handles
//! for each file access to ensure thread-safe concurrent streaming (ADR 0098).

use std::collections::BTreeSet;
use std::io::{Read, Seek};
use zip::ZipArchive;

use crate::{Directory, File, FileList, FsError};

/// Read-only ZIP-backed [`Directory`] implementation.
///
/// Wraps a cloneable, seekable source `R` (such as `std::fs::File` or `Arc<Vec<u8>>`)
/// and maintains a cached index of all entry paths for efficient traversal.
#[derive(Debug, Clone)]
pub struct ZipDir<R: Read + Seek + Send + Sync + Clone> {
    source: R,
    prefix: String,
    index: BTreeSet<String>,
}

impl<R: Read + Seek + Send + Sync + Clone> ZipDir<R> {
    /// Bootstraps a ZIP directory from the given `source`.
    ///
    /// Scans the archive central directory exactly once to populate the index.
    ///
    /// # Errors
    /// Returns [`FsError::Io`] if the source is not a valid ZIP archive.
    pub fn new(mut source: R) -> Result<Self, FsError> {
        let archive = ZipArchive::new(&mut source)
            .map_err(|e| FsError::Io(format!("invalid zip archive: {e}")))?;

        let index = archive.file_names().map(String::from).collect::<BTreeSet<String>>();

        Ok(Self { source, prefix: String::new(), index })
    }

    /// Returns a new handle into the same ZIP source, rooted at `path` relative
    /// to this directory's current prefix.
    #[must_use]
    pub fn subdir(&self, path: &str) -> Self {
        let prefix = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{path}", self.prefix)
        };
        Self { source: self.source.clone(), prefix, index: self.index.clone() }
    }
}

impl<R: Read + Seek + Send + Sync + Clone + 'static> Directory for ZipDir<R> {
    fn create_dir(&self, path: &str) -> Result<(), FsError> {
        let full_path = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{path}", self.prefix)
        };

        // If the path already "exists" as a directory entry or file prefix, we treat creation as idempotent.
        if self.index.contains(&full_path)
            || self.index.contains(&format!("{full_path}/"))
            || self.index.iter().any(|name| name.starts_with(&format!("{full_path}/")))
        {
            return Ok(());
        }

        // Otherwise, ZIP backends are read-only (ADR 0098).
        Err(FsError::PermissionDenied(full_path))
    }

    fn get_file(&self, path: &str) -> Result<Box<dyn File>, FsError> {
        let full_path = if self.prefix.is_empty() {
            path.to_string()
        } else {
            format!("{}/{path}", self.prefix)
        };

        Ok(Box::new(ZipFile {
            source: self.source.clone(),
            inner_path: full_path,
            index: self.index.clone(),
        }))
    }

    fn list_files(&self) -> Result<FileList, FsError> {
        let strip_len = if self.prefix.is_empty() { 0 } else { self.prefix.len() + 1 };

        let entries: Vec<String> = self
            .index
            .iter()
            .filter(|&name| {
                self.prefix.is_empty()
                    || (name.starts_with(&self.prefix)
                        && name.get(self.prefix.len()..self.prefix.len() + 1) == Some("/"))
            })
            .map(|name| name[strip_len..].to_string())
            .collect();

        Ok(Box::new(entries.into_iter().map(Ok)))
    }
}

/// Handle to a single file within a ZIP archive.
#[derive(Debug, Clone)]
pub struct ZipFile<R: Read + Seek + Send + Sync + Clone> {
    source: R,
    inner_path: String,
    index: BTreeSet<String>,
}

impl<R: Read + Seek + Send + Sync + Clone + 'static> File for ZipFile<R> {
    fn write_text(&self, _content: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied(self.inner_path.clone()))
    }

    fn write_bytes(&self, _bytes: &[u8]) -> Result<(), FsError> {
        Err(FsError::PermissionDenied(self.inner_path.clone()))
    }

    fn read_text(&self) -> Result<String, FsError> {
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).map_err(|e| FsError::Io(e.to_string()))
    }

    fn read_bytes(&self) -> Result<Vec<u8>, FsError> {
        let mut source = self.source.clone();
        let mut archive =
            ZipArchive::new(&mut source).map_err(|e| FsError::Io(format!("zip error: {e}")))?;

        let mut entry = archive
            .by_name(&self.inner_path)
            .map_err(|_| FsError::NotFound(self.inner_path.clone()))?;

        #[allow(clippy::cast_possible_truncation)]
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(|e| FsError::Io(e.to_string()))?;
        Ok(bytes)
    }

    fn read_reader(&self) -> Result<crate::Reader, FsError> {
        let (reader, mut writer) = os_pipe::pipe().map_err(|e| FsError::Io(e.to_string()))?;
        let source = self.source.clone();
        let inner_path = self.inner_path.clone();

        // Spawn a background streamer to bridge the lifetime of ZipFile handle.
        // Each reader gets its own archive handle (from source clone), ensuring
        // concurrent Seek operations don't collide.
        std::thread::spawn(move || {
            let mut source = source;
            let Ok(mut archive) = ZipArchive::new(&mut source) else { return };

            if let Ok(mut entry) = archive.by_name(&inner_path) {
                let _ = std::io::copy(&mut entry, &mut writer);
            }
        });

        Ok(Box::new(reader))
    }

    fn write_writer(&self) -> Result<crate::Writer, FsError> {
        Err(FsError::PermissionDenied(self.inner_path.clone()))
    }

    fn delete(&self) -> Result<(), FsError> {
        Err(FsError::PermissionDenied(self.inner_path.clone()))
    }

    fn exists(&self) -> bool {
        self.index.contains(&self.inner_path)
    }

    fn last_modified(&self) -> Result<crate::UnixTimestamp, FsError> {
        let mut source = self.source.clone();
        let mut archive =
            ZipArchive::new(&mut source).map_err(|e| FsError::Io(format!("zip error: {e}")))?;

        let entry = archive
            .by_name(&self.inner_path)
            .map_err(|_| FsError::NotFound(self.inner_path.clone()))?;

        let dt = entry.last_modified();
        // Zip DateTime uses 1980 as base.
        // We match it to chrono for Unix Epoch conversion.
        let naive = chrono::NaiveDate::from_ymd_opt(
            i32::from(dt.year()),
            u32::from(dt.month()),
            u32::from(dt.day()),
        )
        .and_then(|d| {
            d.and_hms_opt(u32::from(dt.hour()), u32::from(dt.minute()), u32::from(dt.second()))
        });

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(naive.map_or(0, |n| n.and_utc().timestamp() as u64))
    }
}

#[cfg(test)]
#[path = "zip_test.rs"]
mod tests;
