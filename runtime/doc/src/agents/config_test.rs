//! Table-driven tests for `instructions-dir` configuration validation.
//!
//! These tests cover the same observable cases as the TypeScript tests in
//! `extension.test.ts` independently, without shared cross-language fixtures.

use noyalib::compat::serde_yaml::{Mapping, Value};

use super::{InstructionsDirError, validate_instructions_dir, validate_instructions_dir_from_yaml};

// ── validate_instructions_dir_from_yaml: absent / non-string cases ────────────

#[test]
fn reject_absent_instructions_dir() {
    assert_eq!(validate_instructions_dir_from_yaml(None), Err(InstructionsDirError::Missing));
}

#[test]
fn reject_non_string_instructions_dir_number() {
    let val = Value::Number(noyalib::compat::serde_yaml::Number::from(42));
    assert_eq!(
        validate_instructions_dir_from_yaml(Some(&val)),
        Err(InstructionsDirError::NotAString)
    );
}

#[test]
fn reject_non_string_instructions_dir_mapping() {
    let mut map = Mapping::new();
    map.insert("key", Value::String("value".to_string()));
    let val = Value::Mapping(map);
    assert_eq!(
        validate_instructions_dir_from_yaml(Some(&val)),
        Err(InstructionsDirError::NotAString)
    );
}

// ── validate_instructions_dir: string-level cases ────────────────────────────

struct Case {
    input: &'static str,
    expected: Result<(), InstructionsDirError>,
}

/// Run a batch of table-driven cases against `validate_instructions_dir`.
fn run_cases(cases: &[Case]) {
    for case in cases {
        let result = validate_instructions_dir(case.input);
        assert_eq!(result, case.expected, "input {:?} produced unexpected result", case.input);
    }
}

#[test]
fn reject_empty_string() {
    run_cases(&[
        Case { input: "", expected: Err(InstructionsDirError::Empty) },
        Case { input: "   ", expected: Err(InstructionsDirError::Empty) },
        Case { input: "\t\n", expected: Err(InstructionsDirError::Empty) },
    ]);
}

#[test]
fn reject_no_variable_expression() {
    run_cases(&[
        Case {
            input: "/tmp/vector/instructions",
            expected: Err(InstructionsDirError::MissingSystemTempVar),
        },
        Case {
            input: "vector/instructions",
            expected: Err(InstructionsDirError::MissingSystemTempVar),
        },
        Case { input: "plain-string", expected: Err(InstructionsDirError::MissingSystemTempVar) },
    ]);
}

#[test]
fn reject_unsupported_variable_expressions() {
    run_cases(&[
        Case {
            input: "${project-root}/vector/instructions",
            expected: Err(InstructionsDirError::UnsupportedVariable("${project-root}".to_string())),
        },
        Case {
            input: "${HOME}/vector",
            expected: Err(InstructionsDirError::UnsupportedVariable("${HOME}".to_string())),
        },
        Case {
            input: "${system-temp}/${other-var}/instructions",
            expected: Err(InstructionsDirError::UnsupportedVariable("${other-var}".to_string())),
        },
    ]);
}

#[test]
fn reject_repeated_system_temp_variable() {
    run_cases(&[
        Case {
            input: "${system-temp}/${system-temp}/instructions",
            expected: Err(InstructionsDirError::RepeatedSystemTempVar),
        },
        Case {
            input: "${system-temp}${system-temp}",
            expected: Err(InstructionsDirError::RepeatedSystemTempVar),
        },
    ]);
}

#[test]
fn reject_system_temp_not_at_start() {
    run_cases(&[
        Case {
            input: "prefix/${system-temp}/instructions",
            expected: Err(InstructionsDirError::NotStartingWithSystemTemp),
        },
        Case {
            input: " ${system-temp}/instructions",
            // Leading space — not at start, and also not an empty string.
            // The variable scanner finds ${system-temp} once, so we reach the starts_with check.
            expected: Err(InstructionsDirError::NotStartingWithSystemTemp),
        },
    ]);
}

#[test]
fn reject_path_traversal_in_suffix() {
    run_cases(&[
        Case {
            input: "${system-temp}/../escape",
            expected: Err(InstructionsDirError::ContainsTraversal),
        },
        Case {
            input: "${system-temp}/a/../../escape",
            expected: Err(InstructionsDirError::ContainsTraversal),
        },
        Case {
            input: "${system-temp}/valid/../still-valid/..",
            expected: Err(InstructionsDirError::ContainsTraversal),
        },
    ]);
}

#[test]
fn accept_valid_instructions_dir_values() {
    run_cases(&[
        Case { input: "${system-temp}/vector/instructions", expected: Ok(()) },
        Case { input: "${system-temp}", expected: Ok(()) },
        Case { input: "${system-temp}/", expected: Ok(()) },
        Case { input: "${system-temp}/a/b/c", expected: Ok(()) },
    ]);
}

#[test]
fn valid_value_message_field_is_unused_but_compiles() {
    // Confirm all error variants produce non-empty messages (compile-time coverage).
    let variants = [
        InstructionsDirError::Missing,
        InstructionsDirError::NotAString,
        InstructionsDirError::Empty,
        InstructionsDirError::UnsupportedVariable("${x}".to_string()),
        InstructionsDirError::MissingSystemTempVar,
        InstructionsDirError::RepeatedSystemTempVar,
        InstructionsDirError::NotStartingWithSystemTemp,
        InstructionsDirError::ContainsTraversal,
        InstructionsDirError::EscapesSystemTemp,
    ];
    for variant in &variants {
        assert!(!variant.message().is_empty(), "message must not be empty for {variant:?}");
    }
}
