//! Quality command: orchestrates fmt-check → quality-lint → quality-test.

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use io::CommandBuilder;

#[cfg(test)]
pub(crate) static CURRENT_DIR_TEST_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// String report + passed/failed status.
pub type QualityReport = (String, bool);

/// Exit code + raw stdout.
type ShellOutput = (i32, Vec<u8>);

/// Build the full quality pipeline report and return it together with pass/fail.
/// When `format` is `true`, runs `cargo fmt --all` (applies formatting) as the first step;
/// otherwise runs `cargo fmt --all -- --check` (check only, the default).
/// Optionally writes the report to `quality-report.txt`.
pub async fn execute(write_report: bool, format: bool) -> QualityReport {
    let workspace = env::current_dir()
        .unwrap_or_else(|_| unreachable!("current_dir() cannot fail in normal operation"));
    let workspace_str = workspace.display().to_string();
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let total_start = Instant::now();

    // ── Step 1: cargo fmt --all [-- --check] ─────────────────────────────────
    let fmt_cmd = if format { "cargo fmt --all 2>&1" } else { "cargo fmt --all -- --check 2>&1" };
    let fmt_start = Instant::now();
    let (fmt_status, fmt_stdout) = run_shell(&workspace, fmt_cmd).await;
    let fmt_duration = fmt_start.elapsed().as_secs_f64();
    let fmt_passed = fmt_status == 0;
    let fmt_raw = String::from_utf8_lossy(&fmt_stdout).to_string();

    // ── Step 2: quality-lint (clippy) ────────────────────────────────────────
    let lint_start = Instant::now();
    let (lint_report, lint_passed) = crate::quality_lint::execute(false, None, false, false).await;
    let lint_duration = lint_start.elapsed().as_secs_f64();

    // ── Step 3: quality-test (tests + coverage) ──────────────────────────────
    let test_start = Instant::now();
    let (test_report, test_passed) =
        crate::quality_test_runner::execute(&crate::quality_test_runner::QualityTestConfig {
            write_report: false, // we handle it here
            include_ignore: false,
            no_coverage: false,
            verbose: false,
            coverage_threshold: 90,
            complete_coverage_summary: false,
            open_browser: false,
            package: None,
        })
        .await;
    let test_duration = test_start.elapsed().as_secs_f64();

    // ── Aggregate ────────────────────────────────────────────────────────────
    let all_passed = fmt_passed && lint_passed && test_passed;
    let agg_status = if all_passed { "PASS" } else { "FAIL" };
    let fmt_status = if fmt_passed { "PASS" } else { "FAIL" };
    let lint_status = if lint_passed { "PASS" } else { "FAIL" };
    let test_status = if test_passed { "PASS" } else { "FAIL" };
    let total_duration = total_start.elapsed().as_secs_f64();

    // ── Build combined report ────────────────────────────────────────────────
    let fmt_section = build_fmt_section(fmt_passed, fmt_duration, &fmt_raw, format);
    let summary_section = format!(
        "=== Quality Summary ===\n\
         Format: {fmt_status}  ({fmt_duration:.2}s)\n\
         Lint:   {lint_status}  ({lint_duration:.2}s)\n\
         Tests:  {test_status}  ({test_duration:.2}s)\n\
         ───────────────────────\n\
         Result: {agg_status}  (total: {total_duration:.2}s)\n"
    );

    let report = format!(
        "=== Quality Report ===\n\
         Timestamp: {timestamp}\n\
         Workspace: {workspace_str}\n\
         \n\
         {fmt_section}\
         \n\
         {lint_report}\
         \n\
         {test_report}\
         \n\
         {summary_section}"
    );

    if write_report {
        let report_path = workspace.join("quality-report.txt");
        if let Err(e) = std::fs::write(&report_path, &report) {
            eprintln!("Warning: could not write report to {}: {}", report_path.display(), e);
        }
    }

    (report, all_passed)
}

/// Run the full quality pipeline: fmt [--check] → clippy → tests+coverage.
/// When `format` is `true`, applies formatting before checking; otherwise checks only.
/// Returns exit code 0 only if all three steps pass.
pub async fn run(write_report: bool, format: bool) -> i32 {
    let (report, passed) = execute(write_report, format).await;
    print!("{report}");
    if passed { 0 } else { 1 }
}

fn build_fmt_section(passed: bool, duration_secs: f64, raw: &str, applied: bool) -> String {
    let title = if applied { "=== Format ===" } else { "=== Format Check ===" };
    let status = if passed { "PASS" } else { "FAIL" };
    if passed || raw.trim().is_empty() {
        format!(
            "{title}\n\
             Duration: {duration_secs:.2}s\n\
             Status: {status}\n"
        )
    } else {
        format!(
            "{title}\n\
             Duration: {duration_secs:.2}s\n\
             Status: {status}\n\
             \n\
             --- Diff ---\n\
             {raw}\n"
        )
    }
}

async fn run_shell(workspace: &PathBuf, cmd: &str) -> ShellOutput {
    let mut exec = CommandBuilder::shell_command("xtask-shell-run", cmd)
        .workdir(workspace)
        .run()
        .unwrap_or_else(|e| unreachable!("CommandBuilder::run failed unexpectedly: {e}"));

    let mut stdout = Vec::new();
    std::io::Read::read_to_end(&mut exec.output, &mut stdout).unwrap();
    let status: i32 =
        exec.wait().await.unwrap_or_else(|e| unreachable!("process wait failed unexpectedly: {e}"));
    (status, stdout)
}

#[cfg(test)]
#[path = "quality_test.rs"]
mod tests;
