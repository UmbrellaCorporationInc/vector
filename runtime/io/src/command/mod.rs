//! Shell command boundaries.

mod builder;
mod executor;
mod handle;
mod mock_handle;
mod process_executor;
mod spec;

pub use builder::CommandBuilder;
pub use executor::CommandExecutor;
pub use handle::{CommandExit, CommandHandle, CommandInput, CommandOutput};
pub use mock_handle::{MockCommandHandleBuilder, MockCommandHandleProbe};
pub use process_executor::ProcessCommandExecutor;
pub use spec::{CommandEnvironmentVariable, CommandSpec};

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
