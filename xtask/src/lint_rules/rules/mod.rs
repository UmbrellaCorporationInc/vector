//! Rule registry — collects all active lint rules.
//!
//! Rule registry — collects all active lint rules.
//!
//! Standard rules are always included. Future rules are registered but only
//! activate when `future` is `true` (the `--future` CLI flag).

pub mod aggregator_only_exports;
pub mod file_too_long;
pub mod no_allow_anywhere;
pub mod no_allow_outside_test;
pub mod no_inline_tests;
pub mod no_pub_struct_fields;
pub mod no_std_process_command;
pub mod no_to_string_in_map_err;
pub mod no_tuple_in_signature;
pub mod no_workspace_dependency;
mod registry;

pub use registry::all;
