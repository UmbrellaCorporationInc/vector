//! Directory traversal boundaries.

use crate::{IoError, IoPath};
use std::time::SystemTime;
use tokio::fs::{self, DirEntry};

/// Generic file type classification for directory entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DirectoryEntryType {
    /// Regular file.
    File,

    /// Directory.
    Directory,

    /// Symbolic link.
    Symlink,

    /// Any file type that is not classified by the standard metadata API.
    Other,
}

/// Generic directory entry metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    path: IoPath,
    entry_type: DirectoryEntryType,
    modified: Option<SystemTime>,
}

impl DirectoryEntry {
    /// Create a directory entry value.
    #[must_use]
    pub const fn new(
        path: IoPath,
        entry_type: DirectoryEntryType,
        modified: Option<SystemTime>,
    ) -> Self {
        Self { path, entry_type, modified }
    }

    /// Return the entry path.
    #[must_use]
    pub const fn path(&self) -> &IoPath {
        &self.path
    }

    /// Return the entry file type.
    #[must_use]
    pub const fn entry_type(&self) -> DirectoryEntryType {
        self.entry_type
    }

    /// Return the last modification time when the platform exposes one.
    #[must_use]
    pub const fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    /// Return true when the entry is a regular file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self.entry_type, DirectoryEntryType::File)
    }

    /// Return true when the entry is a directory.
    #[must_use]
    pub const fn is_directory(&self) -> bool {
        matches!(self.entry_type, DirectoryEntryType::Directory)
    }
}

/// Generic options for recursive directory traversal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirectoryTraversalOptions {
    ignored_paths: Vec<IoPath>,
}

impl DirectoryTraversalOptions {
    /// Create traversal options with no ignored paths.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return options with one additional ignored path.
    #[must_use]
    pub fn with_ignored_path(mut self, path: IoPath) -> Self {
        self.ignored_paths.push(path);
        self
    }

    /// Return options with additional ignored paths.
    #[must_use]
    pub fn with_ignored_paths(mut self, paths: impl IntoIterator<Item = IoPath>) -> Self {
        self.ignored_paths.extend(paths);
        self
    }

    /// Return the ignored path prefixes.
    #[must_use]
    pub fn ignored_paths(&self) -> &[IoPath] {
        &self.ignored_paths
    }

    fn is_ignored(&self, path: &IoPath) -> bool {
        self.ignored_paths.iter().any(|ignored_path| {
            path.as_path() == ignored_path.as_path() || path.as_path().starts_with(ignored_path)
        })
    }
}

/// List the direct children of a directory in deterministic path order.
///
/// # Errors
/// Returns [`IoError::File`] if the directory cannot be read or an entry cannot
/// be inspected.
pub async fn list_directory(path: &IoPath) -> Result<Vec<DirectoryEntry>, IoError> {
    let mut entries = Vec::new();
    let mut read_dir = fs::read_dir(path.as_path())
        .await
        .map_err(|error| IoError::File(format!("failed to read directory: {error}")))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|error| IoError::File(format!("failed to read directory entry: {error}")))?
    {
        entries.push(directory_entry(entry).await?);
    }

    sort_entries(&mut entries);
    Ok(entries)
}

/// Recursively traverse a directory in deterministic path order.
///
/// The returned entries include descendants of `root`, but not `root` itself.
///
/// # Errors
/// Returns [`IoError::File`] if any directory in the traversal cannot be read or
/// an entry cannot be inspected.
pub async fn traverse_directory(root: &IoPath) -> Result<Vec<DirectoryEntry>, IoError> {
    traverse_directory_with_options(root, &DirectoryTraversalOptions::default()).await
}

/// Recursively traverse a directory in deterministic path order with options.
///
/// The returned entries include descendants of `root`, but not `root` itself.
/// Entries matching an ignored path, or descendants of an ignored path, are
/// excluded from the result and ignored directories are not descended into.
///
/// # Errors
/// Returns [`IoError::File`] if any non-ignored directory in the traversal
/// cannot be read or an entry cannot be inspected.
pub async fn traverse_directory_with_options(
    root: &IoPath,
    options: &DirectoryTraversalOptions,
) -> Result<Vec<DirectoryEntry>, IoError> {
    traverse_directories_with_options(std::slice::from_ref(root), options).await
}

/// Recursively traverse multiple directory roots in deterministic path order.
///
/// The returned entries include descendants of each root, but not the roots
/// themselves.
///
/// # Errors
/// Returns [`IoError::File`] if any directory in the traversal cannot be read or
/// an entry cannot be inspected.
pub async fn traverse_directories(roots: &[IoPath]) -> Result<Vec<DirectoryEntry>, IoError> {
    traverse_directories_with_options(roots, &DirectoryTraversalOptions::default()).await
}

/// Recursively traverse multiple directory roots in deterministic path order
/// with options.
///
/// The returned entries include descendants of each root, but not the roots
/// themselves. Entries matching an ignored path, or descendants of an ignored
/// path, are excluded from the result and ignored directories are not descended
/// into.
///
/// # Errors
/// Returns [`IoError::File`] if any non-ignored directory in the traversal
/// cannot be read or an entry cannot be inspected.
pub async fn traverse_directories_with_options(
    roots: &[IoPath],
    options: &DirectoryTraversalOptions,
) -> Result<Vec<DirectoryEntry>, IoError> {
    let mut entries = Vec::new();
    let mut pending_roots = roots.to_vec();
    pending_roots.sort_by(|left, right| left.as_path().cmp(right.as_path()));

    for root in pending_roots {
        if options.is_ignored(&root) {
            continue;
        }

        let mut pending = vec![root];
        while let Some(directory) = pending.pop() {
            let children = list_directory(&directory).await?;
            for child in children {
                if options.is_ignored(child.path()) {
                    continue;
                }

                if child.is_directory() {
                    pending.push(child.path.clone());
                }
                entries.push(child);
            }
            pending.sort_by(|left, right| right.as_path().cmp(left.as_path()));
        }
    }

    sort_entries(&mut entries);
    Ok(entries)
}

async fn directory_entry(entry: DirEntry) -> Result<DirectoryEntry, IoError> {
    let file_type = entry.file_type().await.map_err(|error| {
        IoError::File(format!("failed to inspect directory entry type: {error}"))
    })?;
    let metadata = entry.metadata().await.map_err(|error| {
        IoError::File(format!("failed to inspect directory entry metadata: {error}"))
    })?;

    let entry_type = if file_type.is_file() {
        DirectoryEntryType::File
    } else if file_type.is_dir() {
        DirectoryEntryType::Directory
    } else if file_type.is_symlink() {
        DirectoryEntryType::Symlink
    } else {
        DirectoryEntryType::Other
    };

    Ok(DirectoryEntry::new(IoPath::from(entry.path()), entry_type, metadata.modified().ok()))
}

fn sort_entries(entries: &mut [DirectoryEntry]) {
    entries.sort_by(|left, right| left.path.as_path().cmp(right.path.as_path()));
}

#[cfg(test)]
#[path = "directory_test.rs"]
mod tests;
