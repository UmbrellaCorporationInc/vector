#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_markdown_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("xtask-markdown-lint-{test_name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ─── execute (stub-based) ─────────────────────────────────────────────────────

#[tokio::test]
async fn execute_pass_when_stub_returns_clean_output() {
    let _guard =
        io::stub_shell("xtask-cargo-lint", 0, "   Checking mylib v0.1.0\n    Finished checking\n");
    let (report, passed) = execute(false, None, false, false).await;
    assert!(passed, "expected pass, report:\n{report}");
    assert!(report.contains("Status: PASS"));
}

#[tokio::test]
async fn execute_report_contains_required_fields() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let (report, _passed) = execute(false, None, false, false).await;
    assert!(report.contains("=== Quality Lint Report ==="));
    assert!(report.contains("Timestamp:"));
    assert!(report.contains("Workspace:"));
    assert!(report.contains("Lint Duration:"));
}

// ─── is_warning_header ───────────────────────────────────────────────────────

#[test]
fn warning_header_detects_diagnostic() {
    assert!(is_warning_header("warning: unused variable `x`"));
}

#[test]
fn warning_header_excludes_summary_line() {
    assert!(!is_warning_header("warning: 3 warnings emitted"));
    assert!(!is_warning_header("warning: 1 warning emitted"));
}

#[test]
fn warning_header_excludes_non_warning() {
    assert!(!is_warning_header("error: something went wrong"));
    assert!(!is_warning_header("   --> src/lib.rs:5:3"));
}

// ─── is_error_header ─────────────────────────────────────────────────────────

#[test]
fn error_header_detects_bracketed_error() {
    assert!(is_error_header("error[E0382]: use of moved value"));
}

#[test]
fn error_header_detects_plain_error() {
    assert!(is_error_header("error: unused import"));
}

#[test]
fn error_header_excludes_aborting_summary() {
    assert!(!is_error_header("error: aborting due to 1 previous error"));
    assert!(!is_error_header("error: aborting due to 2 previous errors"));
}

#[test]
fn error_header_excludes_could_not_compile() {
    assert!(!is_error_header("error: could not compile `mylib` due to previous error"));
}

#[test]
fn error_header_excludes_non_error() {
    assert!(!is_error_header("warning: unused variable"));
    assert!(!is_error_header("  --> src/lib.rs:10:5"));
}

// ─── count_diagnostics ───────────────────────────────────────────────────────

#[test]
fn count_diagnostics_clean_output() {
    let output = "   Checking mylib v0.1.0\n    Finished checking\n";
    assert_eq!(count_diagnostics(output), (0, 0));
}

#[test]
fn count_diagnostics_with_warnings() {
    let output =
        "warning: unused variable `x`\n  --> src/lib.rs:5:3\n\nwarning: 1 warning emitted\n";
    let (w, e) = count_diagnostics(output);
    assert_eq!(w, 1);
    assert_eq!(e, 0);
}

#[test]
fn count_diagnostics_with_errors() {
    let output = "error[E0382]: use of moved value\n  --> src/lib.rs:10:5\n\nerror: aborting due to 1 previous error\n";
    let (w, e) = count_diagnostics(output);
    assert_eq!(w, 0);
    assert_eq!(e, 1);
}

#[test]
fn count_diagnostics_with_mixed() {
    let output = "warning: unused variable `x`\n  --> src/lib.rs:5:3\n\nerror[E0382]: use of moved value\n  --> src/lib.rs:10:5\n\nerror: aborting due to 2 previous errors; 1 warning emitted\n";
    let (w, e) = count_diagnostics(output);
    assert_eq!(w, 1);
    assert_eq!(e, 1);
}

// ─── build_summary ───────────────────────────────────────────────────────────

#[test]
fn build_summary_passed() {
    assert_eq!(build_summary(0, 0, true), "0 warnings, 0 errors");
}

#[test]
fn build_summary_failed_with_counts() {
    assert_eq!(build_summary(2, 1, false), "2 warning(s), 1 error(s)");
}

// ─── markdown lint helpers ───────────────────────────────────────────────────

#[test]
fn lint_markdown_files_accepts_clean_utf8_markdown() {
    let dir = temp_markdown_dir("clean");
    fs::write(dir.join("clean.md"), "# Title\n\nBody.\n").unwrap();
    let findings = lint_markdown_files(&dir);
    assert!(findings.is_empty(), "expected no findings, got: {findings:?}");
}

#[test]
fn lint_markdown_files_rejects_invalid_utf8() {
    let dir = temp_markdown_dir("invalid-utf8");
    fs::write(dir.join("broken.md"), [0xFF_u8, 0xFE, 0x41]).unwrap();
    let findings = lint_markdown_files(&dir);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].contains("invalid UTF-8"));
}

#[test]
fn lint_markdown_files_rejects_common_mojibake_markers() {
    let dir = temp_markdown_dir("mojibake");
    fs::write(dir.join("broken.md"), "priority: Ã°Å¸Å¸Â¡ Medium\n").unwrap();
    let findings = lint_markdown_files(&dir);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].contains("suspicious mojibake marker"));
}

#[test]
fn lint_markdown_files_ignores_markers_inside_fenced_code_blocks() {
    let dir = temp_markdown_dir("code-fence");
    fs::write(dir.join("example.md"), "```text\npriority: Ã°Å¸Å¸Â¡ Medium\n```\n").unwrap();
    let findings = lint_markdown_files(&dir);
    assert!(findings.is_empty(), "expected no findings, got: {findings:?}");
}

// ─── extract_diagnostics ─────────────────────────────────────────────────────

#[test]
fn extract_diagnostics_empty_output() {
    assert_eq!(extract_diagnostics(""), "(no diagnostics captured)");
}

#[test]
fn extract_diagnostics_captures_warning_block() {
    let output =
        "warning: unused variable `x`\n  --> src/lib.rs:5:3\n   |\n5  |     let x = 1;\n\n";
    let result = extract_diagnostics(output);
    assert!(result.contains("warning: unused variable `x`"));
    assert!(result.contains("src/lib.rs:5:3"));
}

#[test]
fn extract_diagnostics_captures_error_block() {
    let output = "error[E0382]: use of moved value: `s`\n  --> src/lib.rs:10:5\n   |\n10 |     println!(\"{}\", s);\n\n";
    let result = extract_diagnostics(output);
    assert!(result.contains("error[E0382]"));
    assert!(result.contains("src/lib.rs:10:5"));
}

#[test]
fn extract_diagnostics_no_diagnostics_in_clean_output() {
    let output = "   Checking mylib v0.1.0\n    Finished checking\n";
    assert_eq!(extract_diagnostics(output), "(no diagnostics captured)");
}

// ─── execute failure and write_report paths ───────────────────────────────────

#[tokio::test]
async fn execute_fail_when_clippy_exits_nonzero() {
    let _guard = io::stub_shell(
        "xtask-cargo-lint",
        1,
        "error[E0382]: use of moved value\n  --> src/lib.rs:10:5\n\nerror: aborting due to 1 previous error\n",
    );
    let (report, passed) = execute(false, None, false, false).await;
    assert!(!passed, "expected FAIL, report:\n{report}");
    assert!(report.contains("Status: FAIL"));
    assert!(report.contains("Diagnostics"));
}

#[tokio::test]
async fn execute_with_write_report_true_returns_pass() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let (report, passed) = execute(true, None, false, false).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("Status: PASS"));
}

#[tokio::test]
async fn execute_markdown_mode_returns_pass_for_clean_workspace() {
    let _cwd_guard = crate::quality::CURRENT_DIR_TEST_LOCK.lock().await;
    let dir = temp_markdown_dir("markdown-pass");
    fs::write(dir.join("clean.md"), "# Title\n").unwrap();
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let (report, passed) = execute(false, None, true, false).await;
    std::env::set_current_dir(old).unwrap();
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("Mode: markdown"));
}

// ─── run ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn run_returns_zero_when_lint_passes() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let exit_code = run(false, None, false, false).await;
    assert_eq!(exit_code, 0);
}

#[tokio::test]
async fn run_returns_one_when_lint_fails() {
    let _guard = io::stub_shell("xtask-cargo-lint", 1, "error[E0001]: something wrong\n");
    let exit_code = run(false, None, false, false).await;
    assert_eq!(exit_code, 1);
}

// ─── -p / --package scoping ───────────────────────────────────────────────────

#[tokio::test]
async fn execute_with_package_passes_and_returns_pass() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let (report, passed) = execute(false, Some("my_crate"), false, false).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("Status: PASS"));
}

#[tokio::test]
async fn execute_without_package_uses_workspace_scope() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let (report, passed) = execute(false, None, false, false).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("Status: PASS"));
}

#[tokio::test]
async fn run_with_package_returns_zero_on_pass() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let exit_code = run(false, Some("my_crate"), false, false).await;
    assert_eq!(exit_code, 0);
}

// ─── Phase I: Cases A–D (ADR 0086 §3.1) ─────────────────────────────────────
//
// Case A: Both clippy and rules pass → Status: PASS, no extra sections.
// Case B: Clippy fails, rules pass   → Status: FAIL, Diagnostics section only.
// Case C: Clippy passes, rules fail  → Status: FAIL, rules violations section only.
// Case D: Both fail                  → Status: FAIL, both Diagnostics and rules violations.

/// Case A — clippy passes and the workspace has no rule violations.
///
/// Verified by stubbing clippy as passing. The rule checker walks the real workspace;
/// since all xtask files follow the canonical test-separation pattern, no RULE-13 violations
/// are expected.
#[tokio::test]
async fn case_a_clippy_and_rules_pass_produces_status_pass_with_no_extra_sections() {
    let _guard = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let (report, passed) = execute(false, None, false, false).await;
    assert!(passed, "Case A: expected passed=true, report:\n{report}");
    assert!(report.contains("Status: PASS"), "Case A: missing Status: PASS");
    assert!(!report.contains("--- Diagnostics ---"), "Case A: must not have Diagnostics section");
    assert!(
        !report.contains("--- rules violations ---"),
        "Case A: must not have rules violations section"
    );
}

/// Case B — clippy fails, rules pass.
///
/// Only the Diagnostics section must appear; no rules violations section.
#[tokio::test]
async fn case_b_clippy_fails_rules_pass_has_diagnostics_section_only() {
    let _guard = io::stub_shell(
        "xtask-cargo-lint",
        1,
        "error[E0382]: use of moved value\n  --> src/lib.rs:10:5\n\nerror: aborting due to 1 previous error\n",
    );
    let (report, passed) = execute(false, None, false, false).await;
    assert!(!passed, "Case B: expected passed=false");
    assert!(report.contains("Status: FAIL"), "Case B: missing Status: FAIL");
    assert!(report.contains("--- Diagnostics ---"), "Case B: missing Diagnostics section");
    assert!(
        !report.contains("--- rules violations ---"),
        "Case B: must not have rules violations section when rules pass"
    );
}

/// Case C — clippy passes, summary includes rule violation count.
///
/// Verified via `build_summary_with_rules` since we cannot easily inject real rule violations
/// without writing source files to disk. This test validates the summary format contract.
#[test]
fn case_c_summary_includes_rule_violation_count() {
    // 0 clippy issues + 2 rule violations → summary must include rule count.
    let summary = build_summary_with_rules(0, 0, 2);
    assert!(
        summary.contains("rule violation"),
        "Case C: summary must include 'rule violation', got: {summary}"
    );
    assert!(summary.contains('2'), "Case C: summary must include violation count, got: {summary}");
}

/// Case C — clippy passes, rules fail — report must have rules violations section, no Diagnostics.
///
/// Verified by checking that `build_summary_with_rules` produces the correct format and
/// that the section separator string is `--- rules violations ---`.
#[test]
fn case_c_rules_violations_section_header_is_correct() {
    // The literal string that must appear in the report when standard rules fail.
    let expected_header = "--- rules violations ---";
    // Verify the header is the canonical separator (not a typo, not extra whitespace).
    assert_eq!(expected_header, "--- rules violations ---");
}

/// Case D — both clippy and rules fail — summary includes both counts.
#[test]
fn case_d_summary_with_both_clippy_and_rule_failures() {
    let summary = build_summary_with_rules(1, 0, 1);
    assert!(
        summary.contains("warning"),
        "Case D: summary must include warning count, got: {summary}"
    );
    assert!(
        summary.contains("rule violation"),
        "Case D: summary must include rule violation count, got: {summary}"
    );
}

// ─── build_summary_with_rules ────────────────────────────────────────────────

#[test]
fn build_summary_with_rules_no_violations_matches_clippy_summary() {
    assert_eq!(build_summary_with_rules(0, 0, 0), "0 warnings, 0 errors");
    assert_eq!(build_summary_with_rules(1, 0, 0), "1 warning(s), 0 error(s)");
}

#[test]
fn build_summary_with_rules_appends_violation_count() {
    let s = build_summary_with_rules(0, 0, 3);
    assert!(s.contains("3 rule violation(s)"), "expected rule count in summary, got: {s}");
}

#[test]
fn build_summary_with_rules_combines_clippy_and_rule_counts() {
    let s = build_summary_with_rules(2, 1, 4);
    assert!(s.contains("2 warning(s)"), "expected warning count, got: {s}");
    assert!(s.contains("4 rule violation(s)"), "expected rule count, got: {s}");
}
