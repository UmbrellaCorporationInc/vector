#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_quality_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("xtask-quality-{test_name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn fmt_section_pass_no_raw() {
    let section = build_fmt_section(true, 1.23, "", false);
    assert!(section.contains("Status: PASS"));
    assert!(section.contains("1.23s"));
    assert!(!section.contains("Diff"));
}

#[test]
fn fmt_section_fail_with_diff() {
    let section = build_fmt_section(false, 0.5, "--- src/lib.rs\n+++ src/lib.rs", false);
    assert!(section.contains("Status: FAIL"));
    assert!(section.contains("Diff"));
    assert!(section.contains("src/lib.rs"));
}

#[test]
fn fmt_section_fail_empty_raw() {
    // No diff output — still shows FAIL but no Diff block
    let section = build_fmt_section(false, 0.1, "", false);
    assert!(section.contains("Status: FAIL"));
    assert!(!section.contains("Diff"));
}

#[test]
fn fmt_section_check_uses_format_check_title() {
    let section = build_fmt_section(true, 0.5, "", false);
    assert!(section.contains("Format Check"));
    assert!(!section.contains("=== Format ==="));
}

#[test]
fn fmt_section_applied_uses_format_title() {
    let section = build_fmt_section(true, 0.5, "", true);
    assert!(section.contains("=== Format ==="));
    assert!(!section.contains("Format Check"));
}

// ─── run_shell ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn run_shell_succeeds_for_simple_command() {
    let _guard = io::stub_shell("xtask-shell-run", 0, "cargo 1.80.0\n");
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (exit_code, stdout) = run_shell(&workspace, "cargo --version 2>&1").await;
    assert_eq!(exit_code, 0);
    let stdout_str = String::from_utf8_lossy(&stdout);
    assert!(stdout_str.contains("cargo"));
}

#[tokio::test]
async fn run_shell_captures_failure_exit_code() {
    let _guard = io::stub_shell("xtask-shell-run", 1, "");
    let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (exit_code, _) = run_shell(&workspace, "sh -c 'exit 1'").await;
    assert_ne!(exit_code, 0);
}

// ─── execute ─────────────────────────────────────────────────────────────────

fn passing_tests_output() -> &'static str {
    "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)\n\
     test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s\n"
}

#[tokio::test]
async fn execute_all_pass_returns_pass_status() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let (report, passed) = execute(false, false).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("Result: PASS"));
}

#[tokio::test]
async fn execute_fmt_fail_returns_fail_with_diff() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 1, "--- src/lib.rs\n+++ src/lib.rs\n");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let (report, passed) = execute(false, false).await;
    assert!(!passed, "expected FAIL, report:\n{report}");
    assert!(report.contains("Result: FAIL"));
    assert!(report.contains("Diff"));
}

// ─── run ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn run_returns_zero_when_all_pass() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let exit_code = run(false, false).await;
    assert_eq!(exit_code, 0);
}

#[tokio::test]
async fn run_returns_one_when_fmt_fails() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 1, "--- src/lib.rs\n+++ src/lib.rs\n");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let exit_code = run(false, false).await;
    assert_eq!(exit_code, 1);
}

// ─── execute: fail paths ──────────────────────────────────────────────────────

#[tokio::test]
async fn execute_lint_fail_sets_lint_fail_status() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 1, "error[E0001]: unused import\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let (report, passed) = execute(false, false).await;
    assert!(!passed, "expected FAIL, report:\n{report}");
    assert!(report.contains("Lint:   FAIL"));
}

#[tokio::test]
async fn execute_test_fail_sets_test_fail_status() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell(
        "xtask-cargo-tests",
        1,
        "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n",
    );
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let (report, passed) = execute(false, false).await;
    assert!(!passed, "expected FAIL, report:\n{report}");
    assert!(report.contains("Tests:  FAIL"));
}

#[tokio::test]
async fn execute_write_report_true_creates_file() {
    let _cwd_guard = crate::quality::CURRENT_DIR_TEST_LOCK.lock().await;
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let temp_dir = temp_quality_dir("write-report");
    let old_dir = std::env::current_dir().expect("current dir accessible");
    std::env::set_current_dir(&temp_dir).expect("temp dir must be accessible");
    let _ = execute(true, false).await;
    let report_path = temp_dir.join("quality-report.txt");
    std::env::set_current_dir(old_dir).expect("restore current dir");
    assert!(report_path.exists(), "quality-report.txt was not created");
    let _ = fs::remove_file(&report_path);
    let _ = fs::remove_dir_all(&temp_dir);
}

// ─── execute: --format mode ───────────────────────────────────────────────────

#[tokio::test]
async fn execute_with_format_true_passes_and_uses_format_section_title() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let (report, passed) = execute(false, true).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("=== Format ==="), "expected '=== Format ===' section, got:\n{report}");
    assert!(
        !report.contains("Format Check"),
        "did not expect 'Format Check' when --format, got:\n{report}"
    );
}

#[tokio::test]
async fn run_with_format_true_returns_zero_on_pass() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_tests_output());
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let exit_code = run(false, true).await;
    assert_eq!(exit_code, 0);
}
