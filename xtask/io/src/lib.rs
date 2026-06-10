//! Minimal shell I/O surface vendored for `xtask`.

mod error;
pub(crate) mod shell;
mod types;
pub use error::FsError;
pub use shell::{CommandBuilder, Execution, InputSource};
pub use types::Reader;

#[cfg(feature = "test-utils")]
pub use shell::{StubShellGuard, stub_shell};

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
