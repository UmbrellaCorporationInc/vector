//! Path handling API

use std::path::{Path, PathBuf};

/// A normalized path representation for IO boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IoPath {
    inner: PathBuf,
}

impl IoPath {
    /// Create a new `IoPath` from an existing path representation.
    #[must_use]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self { inner: path.as_ref().to_path_buf() }
    }

    /// Join a segment to this path, returning a new `IoPath`.
    #[must_use]
    pub fn join(&self, segment: impl AsRef<Path>) -> Self {
        Self { inner: self.inner.join(segment) }
    }

    /// Expose the underlying `Path` reference.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        self.inner.as_path()
    }
}

impl AsRef<Path> for IoPath {
    fn as_ref(&self) -> &Path {
        self.inner.as_path()
    }
}

impl From<PathBuf> for IoPath {
    fn from(path: PathBuf) -> Self {
        Self { inner: path }
    }
}

#[cfg(test)]
#[path = "path_test.rs"]
mod tests;
