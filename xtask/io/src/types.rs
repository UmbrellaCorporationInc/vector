//! Type aliases for filesystem operations.

use super::error::FsError;

/// Seconds elapsed since the Unix epoch, used to represent file modification timestamps.
pub type UnixTimestamp = u64;

/// Boxed streaming reader returned by [`super::traits::File::read_reader`].
pub type Reader = Box<dyn std::io::Read + Send + Sync>;

/// Boxed streaming writer returned by [`super::traits::File::write_writer`].
pub type Writer = Box<dyn std::io::Write + Send>;

/// Boxed streaming file-path iterator returned by [`super::traits::Directory::list_files`].
pub type FileList = Box<dyn Iterator<Item = Result<String, FsError>> + Send>;

#[cfg(test)]
#[path = "types_test.rs"]
mod tests;
