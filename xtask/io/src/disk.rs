//! Local disk backend. Delegates to `std::fs` — intended for production use.

use crate::{Directory, File, FileList, FsError, Writer};
use std::path::{Path, PathBuf};

fn map_io_err(err: &std::io::Error, path: &Path) -> FsError {
    match err.kind() {
        std::io::ErrorKind::NotFound => FsError::NotFound(path.display().to_string()),
        std::io::ErrorKind::PermissionDenied => {
            FsError::PermissionDenied(path.display().to_string())
        }
        _ => FsError::Io(err.to_string()),
    }
}

/// Local disk [`Directory`] backend and primary entry point.
///
/// Resolves all paths relative to the `root` directory supplied at construction.
#[derive(Debug, Clone)]
pub struct DiskDir {
    base: PathBuf,
}

impl DiskDir {
    /// Bootstraps a disk directory rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { base: root.into() }
    }
}

impl Directory for DiskDir {
    fn create_dir(&self, path: &str) -> Result<(), FsError> {
        let target = self.base.join(path);
        std::fs::create_dir_all(&target).map_err(|e| map_io_err(&e, &target))
    }

    fn get_file(&self, path: &str) -> Result<Box<dyn File>, FsError> {
        Ok(Box::new(DiskFile { path: self.base.join(path) }))
    }

    fn list_files(&self) -> Result<FileList, FsError> {
        // Imperative stack-based traversal — a functional pipeline cannot naturally
        // express recursive directory descent without an external crate.
        let mut stack = vec![self.base.clone()];
        let mut files: Vec<Result<String, FsError>> = Vec::new();

        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).map_err(|e| map_io_err(&e, &dir))? {
                let path = entry.map_err(|e| map_io_err(&e, &dir))?.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let relative =
                        path.strip_prefix(&self.base).map_err(|e| FsError::Io(e.to_string()))?;
                    files.push(Ok(relative.to_string_lossy().replace('\\', "/")));
                }
            }
        }

        Ok(Box::new(files.into_iter()))
    }
}

/// Local disk [`File`] handle.
#[derive(Debug, Clone)]
pub struct DiskFile {
    path: PathBuf,
}

impl File for DiskFile {
    fn write_text(&self, content: &str) -> Result<(), FsError> {
        self.write_bytes(content.as_bytes())
    }

    fn write_bytes(&self, bytes: &[u8]) -> Result<(), FsError> {
        std::fs::write(&self.path, bytes).map_err(|e| map_io_err(&e, &self.path))
    }

    fn read_text(&self) -> Result<String, FsError> {
        std::fs::read_to_string(&self.path).map_err(|e| map_io_err(&e, &self.path))
    }

    fn read_bytes(&self) -> Result<Vec<u8>, FsError> {
        std::fs::read(&self.path).map_err(|e| map_io_err(&e, &self.path))
    }

    fn read_reader(&self) -> Result<crate::Reader, FsError> {
        let file = std::fs::File::open(&self.path).map_err(|e| map_io_err(&e, &self.path))?;
        Ok(Box::new(file))
    }

    fn write_writer(&self) -> Result<Writer, FsError> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| map_io_err(&e, &self.path))?;
        Ok(Box::new(std::io::BufWriter::new(file)))
    }

    fn delete(&self) -> Result<(), FsError> {
        std::fs::remove_file(&self.path).map_err(|e| map_io_err(&e, &self.path))
    }

    fn exists(&self) -> bool {
        self.path.is_file()
    }

    fn last_modified(&self) -> Result<crate::UnixTimestamp, FsError> {
        let modified = std::fs::metadata(&self.path)
            .map_err(|e| map_io_err(&e, &self.path))?
            .modified()
            .map_err(|e| FsError::Io(e.to_string()))?;
        modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| FsError::Io(e.to_string()))
    }

    fn path(&self) -> Option<std::path::PathBuf> {
        Some(self.path.clone())
    }
}

#[cfg(test)]
#[path = "disk_test.rs"]
mod tests;
