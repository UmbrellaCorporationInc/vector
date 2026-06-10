//! Rule registry — collects all active lint rules.
//!
//! Rule registry — collects all active lint rules.
//!
//! Standard rules are always included. Future rules are registered but only
//! activate when `future` is `true` (the `--future` CLI flag).

pub(crate) mod aggregator_only_exports;
pub(crate) mod file_too_long;
pub(crate) mod no_allow_anywhere;
pub(crate) mod no_allow_outside_test;
pub(crate) mod no_inline_tests;
pub(crate) mod no_pub_struct_fields;
pub(crate) mod no_std_process_command;
pub(crate) mod no_to_string_in_map_err;
pub(crate) mod no_tuple_in_signature;
pub(crate) mod no_workspace_dependency;
mod registry;

pub(crate) use registry::all;
