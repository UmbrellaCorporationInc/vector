//! Quality-test command: workspace tests + optional coverage report.

use std::env;
use std::time::Instant;

use chrono::Utc;
use io::CommandBuilder;

/// Options for the quality-test command.
///
/// # DTO(quality test execution configuration with coverage and reporting options)
pub struct QualityTestConfig<'a> {
    pub write_report: bool,
    pub include_ignore: bool,
    /// Skip coverage.
    pub no_coverage: bool,
    pub verbose: bool,
    /// Coverage threshold %. Files below this appear in the summary table.
    pub coverage_threshold: u8,
    /// Show the full per-file table regardless of threshold.
    pub complete_coverage_summary: bool,
    /// After coverage succeeds, also generate an HTML report and open it in the browser.
    pub open_browser: bool,
    /// Scope tests and coverage to a single package (mirrors `cargo test -p` /
    /// `cargo llvm-cov --package`). All other flags remain independent.
    pub package: Option<&'a str>,
}

/// Report string + pass/fail status.
pub type TestReport = (String, bool);

/// Pair of (status_tag, table_output).
type TestStatusLine = (&'static str, String);

/// Pair of (crate_name, test_type).
type CrateTestMetadata = (String, String);

/// Build the test+coverage report string and return it together with pass/fail.
/// Optionally writes the report to `quality-test-report.txt`.
///
/// Run workspace tests with code coverage metrics and triple-consumable output.
/// Returns the report content and a boolean indicating whether all tests and coverage goals passed.
pub async fn execute(config: &QualityTestConfig<'_>) -> TestReport {
    let start = Instant::now();
    let workspace = env::current_dir()
        .unwrap_or_else(|_| unreachable!("current_dir() cannot fail in normal operation"));
    let workspace_str = workspace.display().to_string();
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let report_title = match config.package {
        Some(pkg) => format!("=== Quality Test Report (package: {pkg}) ==="),
        None => "=== Quality Test Report ===".to_string(),
    };

    // CommandBuilder merges stdout+stderr into a single stream, equivalent to 2>&1.
    let mut builder: CommandBuilder = CommandBuilder::new("xtask-cargo-tests", "cargo");
    builder = match config.package {
        Some(pkg) => builder.args(["llvm-cov", "--all-features", "--package", pkg]),
        None => builder.args(["llvm-cov", "--all-features", "--workspace"]),
    };
    builder = builder.args([
        "--ignore-filename-regex",
        r"_test\.rs$|src/main\.rs$|src\\main\.rs$",
        "--lcov",
        "--output-path",
        "lcov.info",
    ]);
    builder = match config.include_ignore {
        true => builder.args(["--", "--include-ignored"]),
        false => builder,
    };

    let (test_output, passed) = match builder.workdir(&workspace).run() {
        Err(e) => {
            let report = format!(
                "{report_title}\nTimestamp: {timestamp}\nWorkspace: {workspace_str}\nStatus: FAIL\nSummary: failed to spawn cargo test: {e}\n"
            );
            if config.write_report {
                let report_path = workspace.join("quality-test-report.txt");
                let _ = std::fs::write(&report_path, &report);
            }
            return (report, false);
        }
        Ok(mut exec) => {
            let mut buf = String::new();
            let _ = std::io::Read::read_to_string(&mut exec.output, &mut buf);
            let exit_code = exec.wait().await.unwrap_or(1);
            (buf, exit_code == 0)
        }
    };

    let test_duration_secs = start.elapsed().as_secs_f64();
    let (status, summary) = parse_test_output(&test_output, "", passed);

    let mut report = if config.verbose {
        format!(
            r#"{report_title}
Timestamp: {timestamp}
Workspace: {workspace_str}
Test Duration: {test_duration_secs:.2}s
Status: {status}
Summary: {summary}

--- Raw Output ---
{test_output}
"#
        )
    } else if passed {
        format!(
            r#"{report_title}
Timestamp: {timestamp}
Workspace: {workspace_str}
Test Duration: {test_duration_secs:.2}s
Status: {status}
Summary: {summary}
"#
        )
    } else {
        let failure_output = extract_failure_output(&test_output, "");
        format!(
            r#"{report_title}
Timestamp: {timestamp}
Workspace: {workspace_str}
Test Duration: {test_duration_secs:.2}s
Status: {status}
Summary: {summary}

--- Failures ---
{failure_output}
"#
        )
    };

    let run_coverage = passed && !config.no_coverage;
    let threshold = config.coverage_threshold;

    if run_coverage {
        let cov_start = Instant::now();
        // lcov.info was already written by the first cargo llvm-cov invocation above.
        // Read it directly — no second process needed.
        let lcov_path = workspace.join("lcov.info");
        let lcov_str = std::fs::read_to_string(&lcov_path).unwrap_or_default();
        let cov_duration_secs = cov_start.elapsed().as_secs_f64();

        if lcov_str.is_empty() {
            report
                .push_str("\n\n=== Coverage Report ===\nSkipped: lcov.info not found or empty.\n");
        } else {
            let cov_table =
                extract_lcov_coverage_table(&lcov_str, threshold, config.complete_coverage_summary);
            report.push_str(&format!(
                r#"

=== Coverage Report (per-file: function, line, branch) ===
Duration: {cov_duration_secs:.2}s
Threshold: {threshold}% (N/A = - = 100%)

{cov_table}
"#
            ));
        }

        if config.open_browser {
            let mut html_args: Vec<&str> = if let Some(pkg) = config.package {
                vec!["llvm-cov", "--all-features", "--package", pkg]
            } else {
                vec!["llvm-cov", "--all-features", "--workspace"]
            };
            html_args.extend([
                "--html",
                "--open",
                "--no-clean",
                "--ignore-filename-regex",
                r"_test\.rs$|src/main\.rs$|src\\main\.rs$",
            ]);
            if let Ok(exec) = CommandBuilder::new("xtask-cargo-llvm-cov-html", "cargo")
                .args(html_args.iter().copied())
                .workdir(&workspace)
                .run()
            {
                let _ = exec.wait().await;
            }
        }
    }

    if config.write_report {
        let report_path = workspace.join("quality-test-report.txt");
        if let Err(e) = std::fs::write(&report_path, &report) {
            eprintln!("Warning: could not write report to {}: {}", report_path.display(), e);
        }
    }

    (report, passed)
}

/// Command line entry point.
pub async fn run(config: QualityTestConfig<'_>) -> i32 {
    let (report, passed) = execute(&config).await;
    print!("{report}");
    if passed { 0 } else { 1 }
}

// --- Test summary table ---

struct TestSummaryRow {
    crate_name: String,
    test_type: String,
    passed: u32,
    failed: u32,
    ignored: u32,
    measured: u32,
    filtered: u32,
}

fn parse_test_output(stdout: &str, stderr: &str, success: bool) -> TestStatusLine {
    let combined = format!("{stdout}\n{stderr}");
    let rows = collect_test_summaries(&combined);
    let table = build_summary_table(&rows);
    let status = if success { "PASS" } else { "FAIL" };
    (status, table)
}

fn collect_test_summaries(output: &str) -> Vec<TestSummaryRow> {
    let mut rows = Vec::new();
    let mut current_crate = String::new();
    let mut current_type = String::new();

    for line in output.lines() {
        if line.contains("Running ") {
            if let Some((crate_name, test_type)) = parse_running_line(line) {
                current_crate = crate_name;
                current_type = test_type;
            }
        } else if line.contains("test result:")
            && let Some(c) = parse_test_result_line(line)
        {
            rows.push(TestSummaryRow {
                crate_name: current_crate.clone(),
                test_type: current_type.clone(),
                passed: c.passed,
                failed: c.failed,
                ignored: c.ignored,
                measured: c.measured,
                filtered: c.filtered,
            });
        }
    }
    rows
}

fn parse_running_line(line: &str) -> Option<CrateTestMetadata> {
    let run = line.trim().trim_start_matches("Running").trim();
    let in_paren = run.find('(').and_then(|i| run[i + 1..].strip_suffix(')'))?;
    let exe = in_paren.split(['/', '\\']).next_back()?;
    let crate_part = exe.strip_suffix(".exe").unwrap_or(exe);
    let crate_name = crate_part
        .rsplit_once('-')
        .and_then(|(name, hash)| {
            if hash.chars().all(|c| c.is_ascii_hexdigit()) && hash.len() >= 12 {
                Some(name.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| crate_part.to_string());

    let before_paren = run.split('(').next()?.trim();
    let type_label = if before_paren.starts_with("unittests") {
        if before_paren.contains("src\\lib.rs") || before_paren.contains("src/lib.rs") {
            "lib"
        } else if before_paren.contains("src\\main.rs") || before_paren.contains("src/main.rs") {
            "bin"
        } else {
            "unit"
        }
    } else if before_paren.starts_with("tests")
        || before_paren.contains("tests\\")
        || before_paren.contains("tests/")
    {
        "integration"
    } else {
        "test"
    };
    Some((crate_name, type_label.to_string()))
}

/// Counts of test results from a `cargo test` summary line.
struct TestCounts {
    passed: u32,
    failed: u32,
    ignored: u32,
    measured: u32,
    filtered: u32,
}

fn parse_test_result_line(line: &str) -> Option<TestCounts> {
    let after = line.trim().trim_start_matches("test result:").trim();
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ignored = 0u32;
    let mut measured = 0u32;
    let mut filtered = 0u32;
    for part in after.split(';') {
        let part = part.trim();
        let num =
            part.split_whitespace().find(|w| w.parse::<u32>().is_ok()).and_then(|w| w.parse().ok());
        if let Some(n) = num {
            if part.contains("passed") {
                passed = n;
            } else if part.contains("failed") {
                failed = n;
            } else if part.contains("ignored") {
                ignored = n;
            } else if part.contains("measured") {
                measured = n;
            } else if part.contains("filtered") {
                filtered = n;
            }
        }
    }
    Some(TestCounts { passed, failed, ignored, measured, filtered })
}

fn build_summary_table(rows: &[TestSummaryRow]) -> String {
    let mut out = String::new();
    if rows.is_empty() {
        return "No test results.".to_string();
    }

    let header = "| Crate      | Type        | Passed | Failed | Ignored | Measured | Filtered |";
    let sep = "|------------|-------------|--------|--------|---------|----------|----------|";
    out.push('\n');
    out.push_str(header);
    out.push('\n');
    out.push_str(sep);
    out.push('\n');

    for r in rows {
        if r.passed == 0 && r.failed == 0 && r.ignored == 0 && r.measured == 0 && r.filtered == 0 {
            continue;
        }
        out.push_str(&format!(
            "| {:<10} | {:<11} | {:>6} | {:>6} | {:>7} | {:>8} | {:>8} |\n",
            truncate_str(&r.crate_name, 10),
            truncate_str(&r.test_type, 11),
            r.passed,
            r.failed,
            r.ignored,
            r.measured,
            r.filtered
        ));
    }

    let total_passed: u32 = rows.iter().map(|r| r.passed).sum();
    let total_failed: u32 = rows.iter().map(|r| r.failed).sum();
    let total_ignored: u32 = rows.iter().map(|r| r.ignored).sum();
    let total_measured: u32 = rows.iter().map(|r| r.measured).sum();
    let total_filtered: u32 = rows.iter().map(|r| r.filtered).sum();
    out.push_str(
        "|------------|-------------|--------|--------|---------|----------|----------|\n",
    );
    out.push_str(&format!(
        "| {:<10} | {:<11} | {:>6} | {:>6} | {:>7} | {:>8} | {:>8} |\n",
        "TOTAL", "", total_passed, total_failed, total_ignored, total_measured, total_filtered
    ));
    out
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}..", &s[..max.saturating_sub(2)]) }
}

fn extract_failure_output(stdout: &str, stderr: &str) -> String {
    let mut out = String::new();
    if let Some(idx) = stdout.find("failures:") {
        out.push_str(stdout[idx..].trim());
        out.push('\n');
    } else {
        let failed_lines: Vec<&str> = stdout
            .lines()
            .filter(|l| {
                l.contains("FAILED")
                    || l.contains("panicked")
                    || l.contains("assertion")
                    || l.contains("failures:")
            })
            .collect();
        if !failed_lines.is_empty() {
            out.push_str(&failed_lines.join("\n"));
            out.push('\n');
        }
    }
    let stderr_trimmed = stderr.trim();
    if !stderr_trimmed.is_empty() {
        if !out.is_empty() {
            out.push_str("\n--- stderr ---\n");
        }
        out.push_str(stderr_trimmed);
    }
    if out.is_empty() {
        out.push_str("(no failure details captured)");
    }
    out
}

// --- Coverage table ---

struct CoverageRow {
    filename: String,
    func_pct: Option<f64>,
    line_pct: Option<f64>,
    branch_pct: Option<f64>,
}

/// One parsed block from an lcov.info file (SF: … end_of_record).
struct LcovRecord {
    filename: String,
    fn_found: u64,
    fn_hit: u64,
    line_found: u64,
    line_hit: u64,
    br_found: u64,
    br_hit: u64,
}

/// Parse an lcov.info string into per-file records.
fn parse_lcov(lcov_str: &str) -> Vec<LcovRecord> {
    let mut records: Vec<LcovRecord> = Vec::new();
    let mut current: Option<LcovRecord> = None;

    for raw in lcov_str.lines() {
        let line = raw.trim();
        if let Some(path) = line.strip_prefix("SF:") {
            current = Some(LcovRecord {
                filename: path.to_string(),
                fn_found: 0,
                fn_hit: 0,
                line_found: 0,
                line_hit: 0,
                br_found: 0,
                br_hit: 0,
            });
        } else if line == "end_of_record" {
            if let Some(rec) = current.take() {
                records.push(rec);
            }
        } else if let Some(rec) = current.as_mut() {
            if let Some(v) = line.strip_prefix("FNF:") {
                rec.fn_found = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("FNH:") {
                rec.fn_hit = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("LF:") {
                rec.line_found = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("LH:") {
                rec.line_hit = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("BRF:") {
                rec.br_found = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("BRH:") {
                rec.br_hit = v.parse().unwrap_or(0);
            }
        }
    }
    records
}

fn lcov_pct(hit: u64, found: u64) -> Option<f64> {
    if found == 0 { None } else { Some(hit as f64 / found as f64 * 100.0) }
}

fn extract_lcov_coverage_table(lcov_str: &str, threshold: u8, complete: bool) -> String {
    let thresh_f64 = f64::from(threshold);
    let mut out = String::new();

    let records = parse_lcov(lcov_str);
    if records.is_empty() {
        return "No coverage data in lcov.info.".to_string();
    }

    let mut rows: Vec<CoverageRow> = records
        .iter()
        .map(|r| CoverageRow {
            filename: r.filename.clone(),
            func_pct: lcov_pct(r.fn_hit, r.fn_found),
            line_pct: lcov_pct(r.line_hit, r.line_found),
            branch_pct: lcov_pct(r.br_hit, r.br_found),
        })
        .collect();

    rows.sort_by(|a, b| {
        let a_below = any_below_threshold(a, thresh_f64);
        let b_below = any_below_threshold(b, thresh_f64);
        match (a_below, b_below) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_min = min_applicable(a);
                let b_min = min_applicable(b);
                a_min.partial_cmp(&b_min).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });

    let header = "| File                         | Func%   | Line%   | Branch%  |";
    let sep = "|------------------------------|---------|---------|----------|";
    out.push_str(header);
    out.push('\n');
    out.push_str(sep);
    out.push('\n');

    let mut shown = 0usize;
    for r in &rows {
        if !complete && !any_below_threshold(r, thresh_f64) {
            continue;
        }
        shown += 1;
        let short = r
            .filename
            .find("forge\\")
            .map(|i| &r.filename[i + 6..])
            .or_else(|| r.filename.find("forge/").map(|i| &r.filename[i + 6..]))
            .unwrap_or(&r.filename);
        let short = truncate_path(short, 28);
        out.push_str(&format!(
            "| {:<28} | {:>7} | {:>7} | {:>8} |\n",
            short,
            fmt_pct(r.func_pct),
            fmt_pct(r.line_pct),
            fmt_pct(r.branch_pct),
        ));
    }

    if !complete && shown == 0 {
        out.push_str("| (no files below threshold)                    |\n");
    }
    out.push_str(sep);
    out.push('\n');

    // Totals: aggregate across all records.
    let total_fn_found: u64 = records.iter().map(|r| r.fn_found).sum();
    let total_fn_hit: u64 = records.iter().map(|r| r.fn_hit).sum();
    let total_lf: u64 = records.iter().map(|r| r.line_found).sum();
    let total_lh: u64 = records.iter().map(|r| r.line_hit).sum();
    let total_brf: u64 = records.iter().map(|r| r.br_found).sum();
    let total_brh: u64 = records.iter().map(|r| r.br_hit).sum();
    out.push_str(&format!(
        "| {:<28} | {:>7} | {:>7} | {:>8} |\n",
        "TOTAL",
        fmt_pct(lcov_pct(total_fn_hit, total_fn_found)),
        fmt_pct(lcov_pct(total_lh, total_lf)),
        fmt_pct(lcov_pct(total_brh, total_brf)),
    ));
    out.push_str("\nLegend: - = N/A (100%).");
    out
}

fn fmt_pct(pct: Option<f64>) -> String {
    match pct {
        None => "-".to_string(),
        Some(v) => format!("{:.1}%", v),
    }
}

fn any_below_threshold(r: &CoverageRow, thresh: f64) -> bool {
    [r.func_pct, r.line_pct, r.branch_pct].into_iter().flatten().any(|p| p < thresh)
}

fn min_applicable(r: &CoverageRow) -> f64 {
    [r.func_pct, r.line_pct, r.branch_pct].into_iter().flatten().fold(f64::MAX, |a, b| a.min(b))
}

fn truncate_path(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let parts: Vec<&str> = s.split(['/', '\\']).collect();
    for n in [3, 2, 1] {
        if parts.len() >= n {
            let short: String = parts[parts.len() - n..].join("\\");
            if short.len() <= max {
                return short;
            }
        }
    }
    let last = parts.last().unwrap_or(&"");
    if last.len() <= max {
        last.to_string()
    } else {
        format!("..{}", &last[last.len().saturating_sub(max - 2)..])
    }
}

#[cfg(test)]
#[path = "quality_test_runner_test.rs"]
mod tests;
