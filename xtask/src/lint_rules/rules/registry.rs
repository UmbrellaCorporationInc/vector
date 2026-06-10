//! Rule registry implementation.

use super::super::rule::Rule;
use super::{
    aggregator_only_exports, file_too_long, no_allow_anywhere, no_allow_outside_test,
    no_inline_tests, no_pub_struct_fields, no_std_process_command, no_to_string_in_map_err,
    no_tuple_in_signature, no_workspace_dependency,
};

/// Returns all registered rules.
///
/// The returned `Vec` contains both standard and future rules. Callers use
/// [`Rule::is_active`] to filter by run mode before checking.
#[must_use]
pub(crate) fn all(future: bool) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(aggregator_only_exports::AggregatorOnlyExports),
        Box::new(file_too_long::FileTooLong { limit: 700, is_future: future }),
        Box::new(no_allow_anywhere::NoAllowAnywhere { is_future: future }),
        Box::new(no_allow_outside_test::NoAllowOutsideTest),
        Box::new(no_inline_tests::NoInlineTests),
        Box::new(no_pub_struct_fields::NoPubStructFields),
        Box::new(no_to_string_in_map_err::NoToStringInMapErr),
        Box::new(no_tuple_in_signature::NoTupleInSignature),
        Box::new(no_std_process_command::NoStdProcessCommand),
        Box::new(no_workspace_dependency::NoWorkspaceDependency),
    ]
}

#[cfg(test)]
#[path = "registry_test.rs"]
mod tests;
