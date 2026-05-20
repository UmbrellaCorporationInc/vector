//! Runtime IO boundary
//!
//! Provides file, memory, text, path, and shell command IO boundaries over
//! runtime-core sender and receiver contracts.
//!
//! # Domain aliases
//!
//! [`Writer<T>`] and [`Reader<T>`] are named sub-traits over the
//! [`Sender<T>`][runtime_core::channel::Sender] and
//! [`Receiver<T>`][runtime_core::channel::Receiver] contracts. Every concrete
//! `Sender<T>` or `Receiver<T>` satisfies the corresponding alias automatically.
//!
//! Shell command support is split into `CommandBuilder` and `CommandSpec` for
//! planning, `CommandExecutor` and `ProcessCommandExecutor` for execution, and
//! `CommandHandle` plus `MockCommandHandleBuilder` for running or deterministic
//! test boundaries.

pub mod alias;
pub mod bytes;
pub mod command;
pub mod error;
pub mod file;
pub mod memory;
pub mod path;
pub mod text;

pub use alias::{Reader, Writer};
pub use bytes::*;
pub use command::*;
pub use error::*;
pub use file::*;
pub use memory::*;
pub use path::*;
pub use text::*;

#[cfg(test)]
#[path = "lib_test.rs"]
mod tests;
