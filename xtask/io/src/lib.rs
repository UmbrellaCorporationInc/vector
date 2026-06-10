//! Backend-agnostic filesystem abstraction for the Forge runtime.
//!
//! Exposes three object-safe trait boundaries: [`Directory`], [`File`], and [`FileLookup`].
//! Built-in backends: [`memory::MemoryDir`] (zero disk I/O, for tests) and
//! [`disk::DiskDir`] (production, wraps `std::fs`).

pub(crate) mod disk;
mod error;
pub(crate) mod memory;
pub(crate) mod shell;
mod traits;
mod types;
pub(crate) mod zip;

pub use disk::DiskDir;
pub use error::FsError;
pub use memory::MemoryDir;
pub use shell::{CommandBuilder, Execution, InputSource};
pub use traits::{Directory, File, FileLookup};
pub use types::{FileList, Reader, UnixTimestamp, Writer};
pub use zip::{ZipDir, ZipFile};

#[cfg(feature = "test-utils")]
pub use shell::{StubShellGuard, stub_shell};

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
