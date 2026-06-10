#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;

// ─── execute (stub-based) ─────────────────────────────────────────────────────

fn passing_test_output() -> &'static str {
    "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)\n\
     test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s\n"
}

fn default_config() -> QualityTestConfig<'static> {
    QualityTestConfig {
        write_report: false,
        include_ignore: false,
        no_coverage: true,
        verbose: false,
        coverage_threshold: 90,
        complete_coverage_summary: false,
        open_browser: false,
        package: None,
    }
}

#[tokio::test]
async fn execute_pass_when_stub_returns_ok_test_output() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let (report, passed) = execute(&default_config()).await;
    assert!(passed, "expected pass, report:\n{report}");
    assert!(report.contains("Status: PASS"));
}

#[tokio::test]
async fn execute_report_contains_required_fields() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let (report, _) = execute(&default_config()).await;
    assert!(report.contains("=== Quality Test Report ==="));
    assert!(report.contains("Timestamp:"));
    assert!(report.contains("Workspace:"));
    assert!(report.contains("Test Duration:"));
}

#[tokio::test]
async fn execute_verbose_includes_raw_output() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let config = QualityTestConfig { verbose: true, ..default_config() };
    let (report, _) = execute(&config).await;
    assert!(report.contains("--- Raw Output ---"));
}

// ─── truncate_str ────────────────────────────────────────────────────────────

#[test]
fn truncate_str_shorter_than_max() {
    assert_eq!(truncate_str("hello", 10), "hello");
}

#[test]
fn truncate_str_exact_max() {
    assert_eq!(truncate_str("hello", 5), "hello");
}

#[test]
fn truncate_str_longer_than_max() {
    assert_eq!(truncate_str("hello world", 7), "hello..");
}

// ─── truncate_path ───────────────────────────────────────────────────────────

#[test]
fn truncate_path_short_fits_unchanged() {
    assert_eq!(truncate_path("src/lib.rs", 28), "src/lib.rs");
}

#[test]
fn truncate_path_long_uses_trailing_segments() {
    let path = "runtime/language/src/compiler.rs";
    let result = truncate_path(path, 28);
    assert!(result.len() <= 28, "result was: {result}");
    assert!(result.ends_with("compiler.rs"));
}

#[test]
fn truncate_path_very_long_filename_truncates() {
    let path = "averylongfilenamethatexceedsmaxcharacters.rs";
    let result = truncate_path(path, 10);
    assert!(result.len() <= 10, "result was: {result}");
    assert!(result.starts_with(".."));
}

// ─── fmt_pct ─────────────────────────────────────────────────────────────────

#[test]
fn fmt_pct_none_shows_dash() {
    assert_eq!(fmt_pct(None), "-");
}

#[test]
fn fmt_pct_value_formatted() {
    assert_eq!(fmt_pct(Some(75.5)), "75.5%");
}

#[test]
fn fmt_pct_hundred() {
    assert_eq!(fmt_pct(Some(100.0)), "100.0%");
}

#[test]
fn fmt_pct_zero() {
    assert_eq!(fmt_pct(Some(0.0)), "0.0%");
}

// ─── any_below_threshold ─────────────────────────────────────────────────────

#[test]
fn any_below_threshold_all_above_returns_false() {
    let row = CoverageRow {
        filename: "f".into(),
        func_pct: Some(95.0),
        line_pct: Some(92.0),
        branch_pct: None,
    };
    assert!(!any_below_threshold(&row, 90.0));
}

#[test]
fn any_below_threshold_one_below_returns_true() {
    let row = CoverageRow {
        filename: "f".into(),
        func_pct: Some(95.0),
        line_pct: Some(85.0),
        branch_pct: None,
    };
    assert!(any_below_threshold(&row, 90.0));
}

#[test]
fn any_below_threshold_all_none_returns_false() {
    let row =
        CoverageRow { filename: "f".into(), func_pct: None, line_pct: None, branch_pct: None };
    assert!(!any_below_threshold(&row, 90.0));
}

#[test]
fn any_below_threshold_exactly_at_threshold_returns_false() {
    let row = CoverageRow {
        filename: "f".into(),
        func_pct: Some(90.0),
        line_pct: Some(90.0),
        branch_pct: None,
    };
    assert!(!any_below_threshold(&row, 90.0));
}

// ─── min_applicable ──────────────────────────────────────────────────────────

#[test]
fn min_applicable_returns_minimum_value() {
    let row = CoverageRow {
        filename: "f".into(),
        func_pct: Some(90.0),
        line_pct: Some(70.0),
        branch_pct: None,
    };
    assert_eq!(min_applicable(&row), 70.0);
}

#[test]
fn min_applicable_all_none_returns_f64_max() {
    let row =
        CoverageRow { filename: "f".into(), func_pct: None, line_pct: None, branch_pct: None };
    assert_eq!(min_applicable(&row), f64::MAX);
}

// ─── lcov_pct ────────────────────────────────────────────────────────────────

#[test]
fn lcov_pct_zero_found_returns_none() {
    assert_eq!(lcov_pct(0, 0), None);
}

#[test]
fn lcov_pct_all_hit_returns_100() {
    assert_eq!(lcov_pct(10, 10), Some(100.0));
}

#[test]
fn lcov_pct_half_hit_returns_50() {
    assert_eq!(lcov_pct(5, 10), Some(50.0));
}

// ─── parse_running_line ──────────────────────────────────────────────────────

#[test]
fn parse_running_line_lib_windows_path() {
    let line = "Running unittests src\\lib.rs (target\\debug\\deps\\forge-abc123def45678)";
    let (name, ty) = parse_running_line(line).unwrap();
    assert_eq!(name, "forge");
    assert_eq!(ty, "lib");
}

#[test]
fn parse_running_line_lib_unix_path() {
    let line = "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)";
    let (name, ty) = parse_running_line(line).unwrap();
    assert_eq!(name, "mylib");
    assert_eq!(ty, "lib");
}

#[test]
fn parse_running_line_bin_main() {
    let line = "Running unittests src\\main.rs (target\\debug\\deps\\mycli-abc123def45678)";
    let (name, ty) = parse_running_line(line).unwrap();
    assert_eq!(name, "mycli");
    assert_eq!(ty, "bin");
}

#[test]
fn parse_running_line_integration_test() {
    let line = "Running tests\\integration.rs (target\\debug\\deps\\cli-abc123def45678)";
    let (name, ty) = parse_running_line(line).unwrap();
    assert_eq!(name, "cli");
    assert_eq!(ty, "integration");
}

#[test]
fn parse_running_line_invalid_returns_none() {
    assert!(parse_running_line("not a running line").is_none());
}

// ─── parse_test_result_line ──────────────────────────────────────────────────

#[test]
fn parse_test_result_line_all_passed() {
    let line = "test result: ok. 5 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.05s";
    let counts = parse_test_result_line(line).unwrap();
    assert_eq!(
        (counts.passed, counts.failed, counts.ignored, counts.measured, counts.filtered),
        (5, 0, 1, 0, 0)
    );
}

#[test]
fn parse_test_result_line_with_failures() {
    let line = "test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out;";
    let counts = parse_test_result_line(line).unwrap();
    assert_eq!((counts.passed, counts.failed, counts.ignored), (3, 2, 0));
}

// ─── build_summary_table ─────────────────────────────────────────────────────

#[test]
fn build_summary_table_empty_returns_message() {
    assert_eq!(build_summary_table(&[]), "No test results.");
}

#[test]
fn build_summary_table_row_appears_in_output() {
    let rows = vec![TestSummaryRow {
        crate_name: "mylib".into(),
        test_type: "lib".into(),
        passed: 10,
        failed: 0,
        ignored: 1,
        measured: 0,
        filtered: 0,
    }];
    let table = build_summary_table(&rows);
    assert!(table.contains("mylib"));
    assert!(table.contains("10"));
    assert!(table.contains("TOTAL"));
}

#[test]
fn build_summary_table_all_zero_row_is_skipped() {
    let rows = vec![TestSummaryRow {
        crate_name: "empty".into(),
        test_type: "lib".into(),
        passed: 0,
        failed: 0,
        ignored: 0,
        measured: 0,
        filtered: 0,
    }];
    let table = build_summary_table(&rows);
    assert!(!table.contains("empty"), "zero-only row should be skipped");
    assert!(table.contains("TOTAL"));
}

#[test]
fn build_summary_table_totals_are_summed() {
    let rows = vec![
        TestSummaryRow {
            crate_name: "a".into(),
            test_type: "lib".into(),
            passed: 4,
            failed: 1,
            ignored: 0,
            measured: 0,
            filtered: 0,
        },
        TestSummaryRow {
            crate_name: "b".into(),
            test_type: "lib".into(),
            passed: 6,
            failed: 0,
            ignored: 2,
            measured: 0,
            filtered: 0,
        },
    ];
    let table = build_summary_table(&rows);
    // TOTAL passed = 10, failed = 1, ignored = 2
    assert!(table.contains("10"));
    assert!(table.contains("TOTAL"));
}

// ─── collect_test_summaries ──────────────────────────────────────────────────

#[test]
fn collect_test_summaries_parses_single_suite() {
    let output = "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)\n\
                  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
    let rows = collect_test_summaries(output);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].passed, 4);
    assert_eq!(rows[0].crate_name, "mylib");
}

#[test]
fn collect_test_summaries_no_running_line_skips() {
    let output = "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
    // No Running line — still parses the result but crate_name will be empty
    let rows = collect_test_summaries(output);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].passed, 2);
}

// ─── extract_failure_output ──────────────────────────────────────────────────

#[test]
fn extract_failure_output_with_failures_section() {
    let stdout = "some output\nfailures:\n  test_foo\n\nfailures:\n  test_foo failed";
    let result = extract_failure_output(stdout, "");
    assert!(result.contains("failures:"));
}

#[test]
fn extract_failure_output_with_failed_lines_fallback() {
    let stdout = "test test_bar ... FAILED\nother output";
    let result = extract_failure_output(stdout, "");
    assert!(result.contains("FAILED"));
}

#[test]
fn extract_failure_output_empty_returns_no_details() {
    let result = extract_failure_output("", "");
    assert_eq!(result, "(no failure details captured)");
}

#[test]
fn extract_failure_output_stderr_included() {
    let result = extract_failure_output("", "error from stderr");
    assert!(result.contains("error from stderr"));
}

// ─── extract_lcov_coverage_table ─────────────────────────────────────────────

fn lcov_file(filename: &str, fnf: u32, fnh: u32, lf: u32, lh: u32, brf: u32, brh: u32) -> String {
    format!(
        "SF:{filename}\nFNF:{fnf}\nFNH:{fnh}\nLF:{lf}\nLH:{lh}\nBRF:{brf}\nBRH:{brh}\nend_of_record\n"
    )
}

#[test]
fn extract_lcov_coverage_table_empty_input_returns_message() {
    let result = extract_lcov_coverage_table("", 90, false);
    assert!(result.contains("No coverage data"));
}

#[test]
fn extract_lcov_coverage_table_shows_file_below_threshold() {
    // 7/10 functions = 70%
    let lcov = lcov_file("/workspace/forge/runtime/io/src/lib.rs", 10, 7, 100, 70, 0, 0);
    let result = extract_lcov_coverage_table(&lcov, 90, false);
    assert!(result.contains("70.0%"), "expected 70.0% in:\n{result}");
    assert!(result.contains("lib.rs"));
}

#[test]
fn extract_lcov_coverage_table_hides_file_above_threshold() {
    // 10/10 = 100%
    let lcov = lcov_file("/workspace/forge/runtime/io/src/lib.rs", 10, 10, 100, 100, 0, 0);
    let result = extract_lcov_coverage_table(&lcov, 90, false);
    assert!(
        result.contains("no files below threshold") || !result.contains("lib.rs\n"),
        "file above threshold should be hidden:\n{result}"
    );
}

#[test]
fn extract_lcov_coverage_table_complete_flag_shows_all_files() {
    let lcov = lcov_file("/workspace/forge/runtime/io/src/lib.rs", 10, 10, 100, 100, 0, 0);
    let result = extract_lcov_coverage_table(&lcov, 90, true);
    assert!(result.contains("lib.rs"), "complete flag should show all files:\n{result}");
    assert!(result.contains("100.0%"));
}

#[test]
fn extract_lcov_coverage_table_always_has_total_row() {
    let lcov = lcov_file("/workspace/forge/runtime/io/src/lib.rs", 10, 8, 100, 80, 0, 0);
    let result = extract_lcov_coverage_table(&lcov, 90, false);
    assert!(result.contains("TOTAL"));
}

#[test]
fn extract_lcov_coverage_table_zero_branches_shows_dash() {
    // BRF=0 → branch% = None → "-"
    let lcov = lcov_file("/workspace/forge/runtime/io/src/lib.rs", 10, 8, 100, 80, 0, 0);
    let result = extract_lcov_coverage_table(&lcov, 90, false);
    assert!(result.contains('-'), "zero BRF should show dash:\n{result}");
}

#[test]
fn extract_lcov_coverage_table_multi_file_totals_aggregated() {
    let mut lcov = lcov_file("/workspace/forge/runtime/io/src/a.rs", 4, 4, 20, 20, 0, 0);
    lcov.push_str(&lcov_file("/workspace/forge/runtime/io/src/b.rs", 6, 3, 30, 15, 0, 0));
    let result = extract_lcov_coverage_table(&lcov, 90, true);
    // Total FNH=7, FNF=10 → 70%
    assert!(result.contains("TOTAL"));
    assert!(result.contains("70.0%"), "expected 70.0% total func in:\n{result}");
}

// ─── parse_test_output ───────────────────────────────────────────────────────

#[test]
fn parse_test_output_success_returns_pass() {
    let stdout = "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)\n\
                  test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;";
    let (status, table) = parse_test_output(stdout, "", true);
    assert_eq!(status, "PASS");
    assert!(table.contains("mylib") || table.contains("3"));
}

#[test]
fn parse_test_output_failure_returns_fail() {
    let stdout = "Running unittests src/lib.rs (target/debug/deps/mylib-abc123def45678)\n\
                  test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out;";
    let (status, _) = parse_test_output(stdout, "", false);
    assert_eq!(status, "FAIL");
}

// ─── parse_running_line (additional branches) ────────────────────────────────

#[test]
fn parse_running_line_unit_type_no_lib_or_main() {
    // "unittests" prefix but path is neither src/lib.rs nor src/main.rs → "unit"
    let line = "Running unittests src/other.rs (target/debug/deps/mymod-abc123def45678)";
    let (_, ty) = parse_running_line(line).unwrap();
    assert_eq!(ty, "unit");
}

#[test]
fn parse_running_line_no_dash_uses_full_name() {
    // Executable has no dash → crate_part used as-is
    let line = "Running unittests src/lib.rs (target/debug/deps/mylib)";
    let (name, _) = parse_running_line(line).unwrap();
    assert_eq!(name, "mylib");
}

#[test]
fn parse_running_line_short_hash_uses_full_name() {
    // Hash exists but is fewer than 12 chars → not treated as hash, full name kept
    let line = "Running unittests src/lib.rs (target/debug/deps/my-lib-abc12)";
    let (name, _) = parse_running_line(line).unwrap();
    // rsplit_once finds last dash, hash "abc12" is < 12 chars → unwrap_or_else
    assert_eq!(name, "my-lib-abc12");
}

#[test]
fn parse_running_line_non_hex_hash_uses_full_name() {
    // Hash contains non-hex chars → unwrap_or_else path
    let line = "Running unittests src/lib.rs (target/debug/deps/mylib-zzzzzzzzzzzz)";
    let (name, _) = parse_running_line(line).unwrap();
    assert_eq!(name, "mylib-zzzzzzzzzzzz");
}

// ─── extract_failure_output (additional branches) ────────────────────────────

#[test]
fn extract_failure_output_stdout_failures_and_stderr_combined() {
    let stdout = "failures:\n  test_foo\n\ntest result: FAILED.";
    let stderr = "thread panicked at src/lib.rs:10";
    let result = extract_failure_output(stdout, stderr);
    assert!(result.contains("failures:"));
    assert!(result.contains("--- stderr ---"));
    assert!(result.contains("src/lib.rs:10"));
}

// ─── build_summary_table (additional branches) ───────────────────────────────

#[test]
fn build_summary_table_long_crate_name_truncated() {
    let rows = vec![TestSummaryRow {
        crate_name: "a_very_long_crate_name".into(),
        test_type: "lib".into(),
        passed: 1,
        failed: 0,
        ignored: 0,
        measured: 0,
        filtered: 0,
    }];
    let table = build_summary_table(&rows);
    // The crate name is 22 chars, max is 10 → gets truncated with ".."
    assert!(table.contains("a_very_lo..") || table.contains("a_very_"));
}

// ─── extract_lcov_coverage_table (additional paths) ──────────────────────────

#[test]
fn extract_lcov_coverage_table_file_with_all_zero_counts_shows_na() {
    // FNF=0 → func% is None → "-"
    let lcov =
        "SF:/workspace/forge/src/lib.rs\nFNF:0\nFNH:0\nLF:10\nLH:10\nBRF:0\nBRH:0\nend_of_record\n";
    let result = extract_lcov_coverage_table(lcov, 90, true);
    assert!(result.contains("TOTAL"));
    // func column should be "-" since FNF=0
    assert!(result.contains('-'), "expected dash for zero-count metric:\n{result}");
}

// ─── execute with open_browser ───────────────────────────────────────────────

#[tokio::test]
async fn execute_open_browser_triggers_html_llvm_cov() {
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let _guard_html = io::stub_shell("xtask-cargo-llvm-cov-html", 0, "");
    let config = QualityTestConfig { no_coverage: false, open_browser: true, ..default_config() };
    let (report, passed) = execute(&config).await;
    assert!(passed, "expected PASS, report:\n{report}");
}

// ─── run ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn run_returns_zero_when_tests_pass() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let exit_code = run(default_config()).await;
    assert_eq!(exit_code, 0);
}

#[tokio::test]
async fn run_returns_one_when_tests_fail() {
    let _guard = io::stub_shell(
        "xtask-cargo-tests",
        1,
        "FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n",
    );
    let exit_code = run(default_config()).await;
    assert_eq!(exit_code, 1);
}

// ─── execute with no_coverage false ──────────────────────────────────────────

#[tokio::test]
async fn execute_with_no_coverage_false_includes_coverage_section() {
    // lcov.info is read from disk; in tests it will be missing/empty → "Skipped" message.
    // The coverage section header must still appear.
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let config = QualityTestConfig { no_coverage: false, ..default_config() };
    let (report, passed) = execute(&config).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("=== Coverage Report"), "coverage section missing:\n{report}");
}

// ─── execute with write_report true ──────────────────────────────────────────

#[tokio::test]
async fn execute_with_write_report_true_returns_pass() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let config = QualityTestConfig { write_report: true, ..default_config() };
    let (report, passed) = execute(&config).await;
    assert!(passed, "expected PASS, report:\n{report}");
}

// ─── execute with -p / package ───────────────────────────────────────────────

#[tokio::test]
async fn execute_package_title_contains_package_name() {
    let _guard = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let config = QualityTestConfig { package: Some("my_crate"), ..default_config() };
    let (report, _) = execute(&config).await;
    assert!(
        report.contains("package: my_crate"),
        "expected package name in report title, got:\n{report}"
    );
}

#[tokio::test]
async fn execute_package_with_coverage_includes_coverage_section() {
    let _guard_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let config =
        QualityTestConfig { no_coverage: false, package: Some("my_crate"), ..default_config() };
    let (report, passed) = execute(&config).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("=== Coverage Report"), "coverage section missing:\n{report}");
}
