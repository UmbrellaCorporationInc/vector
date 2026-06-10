//! Core filesystem trait definitions.

use super::error::FsError;
use super::types::{FileList, Reader, UnixTimestamp, Writer};

/// A generalized architectural interface for resolving file paths over any underlying
/// structural boundary (library, root directory, or network endpoint).
///
/// Implementations provide path-scoped access to [`File`] handles. The caller is
/// responsible for any path validation (e.g. traversal guards); the trait itself
/// makes no safety guarantees beyond those of its methods.
pub trait FileLookup: Send + Sync {
    /// Resolves the given relative `path` against the internal structural boundaries
    /// of this lookup.
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend cannot resolve or allocate the file resource.
    fn get_file(&self, path: &str) -> Result<Box<dyn File>, FsError>;
}

/// Directory handle. Resolves files relative to its root by accepting subpaths.
pub trait Directory: Send + Sync {
    /// Creates the directory at `path` relative to this handle, including all
    /// intermediate segments. Idempotent — calling this twice on the same path
    /// must not return an error.
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend cannot create the directory.
    fn create_dir(&self, path: &str) -> Result<(), FsError>;

    /// Returns a [`File`] handle at `path` relative to this directory.
    /// The `path` argument may be a simple filename or a relative subpath (e.g. `"src/main.rs"`).
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend cannot allocate the file resource.
    fn get_file(&self, path: &str) -> Result<Box<dyn File>, FsError>;

    /// Returns a lazy iterator providing a recursive list of all files contained
    /// within this directory and its subdirectories.
    ///
    /// Paths are strings relative to this directory handle and use `/` as the
    /// universal separator. Only files are returned — directory entries are omitted.
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend fails to initiate context for traversal.
    fn list_files(&self) -> Result<FileList, FsError>;
}

/// File handle for reading, writing, and deleting a single resource.
pub trait File: Send + Sync {
    /// Writes UTF-8 `content` to the file, replacing any existing content.
    ///
    /// # Side Effects
    /// Persists the provided string into the underlying backend (in-memory store or disk),
    /// overwriting any previous contents.
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend write fails.
    fn write_text(&self, content: &str) -> Result<(), FsError>;

    /// Writes raw `bytes` to the file, replacing any existing content.
    ///
    /// # Side Effects
    /// Persists the provided bytes into the underlying backend (in-memory store or disk),
    /// overwriting any previous contents.
    ///
    /// # Errors
    /// Returns [`FsError`] if the backend write fails.
    fn write_bytes(&self, bytes: &[u8]) -> Result<(), FsError>;

    /// Reads the file content as a UTF-8 string.
    ///
    /// # Errors
    /// Returns [`FsError::NotFound`] if the file does not exist, or [`FsError::Io`]
    /// if the content is not valid UTF-8.
    fn read_text(&self) -> Result<String, FsError>;

    /// Reads the file content as raw bytes.
    ///
    /// # Errors
    /// Returns [`FsError::NotFound`] if the file does not exist.
    fn read_bytes(&self) -> Result<Vec<u8>, FsError>;

    /// Returns a box containing a type that implements `Read + Send + Sync` for stream-based reading.
    ///
    /// # Errors
    /// Returns [`FsError::NotFound`] if the file does not exist, or [`FsError::Io`] on backend error.
    fn read_reader(&self) -> Result<Reader, FsError>;

    /// Returns a box containing a type that implements `Write + Send` for stream-based writing.
    ///
    /// The writer buffers data locally. Content is committed to the backend only when
    /// [`std::io::Write::flush`] is called or when the writer is dropped (best-effort on drop).
    ///
    /// # Errors
    /// Returns [`FsError::Io`] if the backend cannot open the resource for writing.
    fn write_writer(&self) -> Result<Writer, FsError>;

    /// Deletes the file from the backend.
    ///
    /// # Side Effects
    /// Removes the underlying resource from the backend. Subsequent reads will observe
    /// a missing file.
    ///
    /// # Errors
    /// Returns [`FsError::NotFound`] if the file does not exist.
    fn delete(&self) -> Result<(), FsError>;

    /// Returns `true` if the file exists in the backend, `false` otherwise.
    #[must_use]
    fn exists(&self) -> bool;

    /// Returns the last modification timestamp of this file as a [`UnixTimestamp`] (seconds since Unix epoch).
    ///
    /// For in-memory backends, this is updated on every `write_text` or `write_bytes` call.
    /// For disk backends, this delegates to `std::fs::metadata`.
    ///
    /// # Errors
    /// Returns [`FsError::NotFound`] if the file does not exist, or [`FsError::Io`] if the backend
    /// cannot retrieve modification metadata.
    fn last_modified(&self) -> Result<UnixTimestamp, FsError>;

    /// Returns the filesystem path of this file if backed by disk (e.g. [`DiskDir`] / [`DiskFile`]).
    /// Returns `None` for in-memory or other backends that do not have a real path (ADR 0017 script execution).
    #[must_use]
    fn path(&self) -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
#[path = "traits_test.rs"]
mod tests;
