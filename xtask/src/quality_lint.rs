//! Quality-lint command: workspace clippy plus optional Markdown integrity checks.

use std::env;
use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use io::CommandBuilder;

const MARKDOWN_MOJIBAKE_MARKERS: &[&str] =
    &["Ã", "Â", "â€™", "â€œ", "â€", "â€“", "â€”", "â€¦", "ðŸ", "�"];

/// Report string + pass/fail status.
pub type LintReport = (String, bool);

/// Pair of (warnings, errors).
pub type DiagnosticCounts = (usize, usize);

/// Run workspace linting with triple-consumable output.
///
/// When `package` is `Some(p)`, scopes clippy to `-p <p>`; otherwise uses `--workspace`.
/// When `markdown` is `true`, runs only Markdown UTF-8 / mojibake validation.
/// When `future` is `true`, also evaluates future rules (informational only; does not affect
/// `passed`).
///
/// Returns the report content and a boolean indicating whether all lints passed.
pub async fn execute(
    write_report: bool,
    package: Option<&str>,
    markdown: bool,
    future: bool,
) -> LintReport {
    let workspace = env::current_dir()
        .unwrap_or_else(|_| unreachable!("current_dir() cannot fail in normal operation"));
    let workspace_str = workspace.display().to_string();
    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let (report, passed) = if markdown {
        execute_markdown_lint(&workspace, &workspace_str, &timestamp)
    } else {
        execute_rust_lint(&workspace, &workspace_str, &timestamp, package, future).await
    };

    if write_report {
        let report_path = workspace.join("quality-lint-report.txt");
        if let Err(e) = std::fs::write(&report_path, &report) {
            eprintln!("Warning: could not write report to {}: {}", report_path.display(), e);
        }
    }

    (report, passed)
}

/// Run the linting and write report if requested.
///
/// When `package` is `Some(p)`, scopes clippy to `-p <p>`; otherwise uses `--workspace`.
/// When `markdown` is `true`, runs only Markdown UTF-8 / mojibake validation.
/// When `future` is `true`, also evaluates future rules (informational only; does not affect
/// exit code).
///
/// Returns 0 if clear, 1 if issues found.
pub async fn run(write_report: bool, package: Option<&str>, markdown: bool, future: bool) -> i32 {
    let (report, passed) = execute(write_report, package, markdown, future).await;
    print!("{report}");
    i32::from(!passed)
}

async fn execute_rust_lint(
    workspace: &Path,
    workspace_str: &str,
    timestamp: &str,
    package: Option<&str>,
    future: bool,
) -> LintReport {
    let start = Instant::now();

    let mut clippy_args = vec!["clippy"];
    if let Some(p) = package {
        clippy_args.extend_from_slice(&["-p", p]);
    } else {
        clippy_args.push("--workspace");
    }
    clippy_args.extend_from_slice(&["--all-targets", "--all-features", "--", "-D", "warnings"]);

    let mut exec = match CommandBuilder::new("xtask-cargo-lint", "cargo")
        .args(clippy_args.iter().copied())
        .workdir(workspace)
        .run()
    {
        Ok(e) => e,
        Err(e) => {
            let duration_secs = start.elapsed().as_secs_f64();
            let report = format!(
                "=== Quality Lint Report ===\nTimestamp: {timestamp}\nWorkspace: {workspace_str}\nLint Duration: {duration_secs:.2}s\nStatus: FAIL\nSummary: failed to spawn clippy: {e}\n"
            );
            return (report, false);
        }
    };

    let mut combined = String::new();
    let _ = std::io::Read::read_to_string(&mut exec.output, &mut combined);
    let clippy_exit = exec.wait().await.unwrap_or(1);
    let duration_secs = start.elapsed().as_secs_f64();
    let clippy_passed = clippy_exit == 0;

    // Run project-local rules regardless of clippy outcome (§3.2 of ADR 0086).
    let all_violations = crate::lint_rules::run(workspace, future);
    let standard_violations: Vec<_> =
        all_violations.iter().filter(|v| !v.rule_id.starts_with("RULE-F")).collect();
    let future_violations: Vec<_> =
        all_violations.iter().filter(|v| v.rule_id.starts_with("RULE-F")).collect();

    let rules_passed = standard_violations.is_empty();
    let passed = clippy_passed && rules_passed;

    let (warnings, errors) = count_diagnostics(&combined);
    let status = if passed { "PASS" } else { "FAIL" };
    let summary = build_summary_with_rules(warnings, errors, standard_violations.len());

    let mut report = format!(
        "=== Quality Lint Report ===\nTimestamp: {timestamp}\nWorkspace: {workspace_str}\nLint Duration: {duration_secs:.2}s\nStatus: {status}\nSummary: {summary}\n"
    );

    // Append clippy diagnostics section when clippy failed (Cases B and D).
    if !clippy_passed {
        let diagnostics = extract_diagnostics(&combined);
        report.push_str(&format!("\n--- Diagnostics ---\n{diagnostics}\n"));
    }

    // Append rules violations section when standard rules failed (Cases C and D).
    if !rules_passed {
        report.push_str("\n--- rules violations ---\n");
        for v in &standard_violations {
            report.push_str(&format_violation(workspace, v));
        }
    }

    // Append future violations section when --future is set and RULE-F* fired (informational).
    if future && !future_violations.is_empty() {
        report.push_str("\n--- future violations ---\n");
        for v in &future_violations {
            report.push_str(&format_violation(workspace, v));
        }
    }

    (report, passed)
}

/// Format a single `RuleViolation` as an output line.
fn format_violation(workspace: &Path, v: &crate::lint_rules::RuleViolation) -> String {
    let rel =
        v.file.strip_prefix(workspace).unwrap_or(&v.file).display().to_string().replace('\\', "/");
    let loc = match (v.line, v.column) {
        (Some(l), Some(c)) => format!(":{l}:{c}"),
        (Some(l), None) => format!(":{l}"),
        _ => String::new(),
    };
    format!("[{}] {}  {}{}\n", v.rule_id, v.message, rel, loc)
}

fn execute_markdown_lint(workspace: &Path, workspace_str: &str, timestamp: &str) -> LintReport {
    let start = Instant::now();
    let findings = lint_markdown_files(workspace);
    let duration_secs = start.elapsed().as_secs_f64();
    let passed = findings.is_empty();
    let status = if passed { "PASS" } else { "FAIL" };
    let summary =
        if passed { "0 finding(s)".to_string() } else { format!("{} finding(s)", findings.len()) };

    let report = if passed {
        format!(
            r"=== Quality Lint Report ===
Timestamp: {timestamp}
Workspace: {workspace_str}
Lint Duration: {duration_secs:.2}s
Mode: markdown
Status: {status}
Summary: {summary}
"
        )
    } else {
        format!(
            r"=== Quality Lint Report ===
Timestamp: {timestamp}
Workspace: {workspace_str}
Lint Duration: {duration_secs:.2}s
Mode: markdown
Status: {status}
Summary: {summary}

--- Diagnostics ---
{}
",
            findings.join("\n")
        )
    };

    (report, passed)
}

// --- Parsing helpers ---

/// Count `warning:` and `error` diagnostic lines in clippy output.
/// Excludes summary lines like "warning: X warning(s) emitted".
#[must_use]
pub fn count_diagnostics(output: &str) -> DiagnosticCounts {
    let mut warnings = 0usize;
    let mut errors = 0usize;
    for line in output.lines() {
        let trimmed = line.trim();
        if is_warning_header(trimmed) {
            warnings += 1;
        } else if is_error_header(trimmed) {
            errors += 1;
        }
    }
    (warnings, errors)
}

/// Build the human-readable summary string (clippy counts only).
#[must_use]
pub fn build_summary(warnings: usize, errors: usize, passed: bool) -> String {
    if passed {
        "0 warnings, 0 errors".to_string()
    } else {
        format!("{warnings} warning(s), {errors} error(s)")
    }
}

/// Build the summary string including rule violation count.
///
/// When `rule_violations` is non-zero, appends ` — N rule violation(s)` to the clippy summary.
#[must_use]
pub fn build_summary_with_rules(warnings: usize, errors: usize, rule_violations: usize) -> String {
    let clippy_part = build_summary(warnings, errors, warnings == 0 && errors == 0);
    if rule_violations == 0 {
        clippy_part
    } else {
        format!("{clippy_part} \u{2014} {rule_violations} rule violation(s)")
    }
}

/// Extract the full diagnostic blocks (warning/error with their context lines).
/// Returns a trimmed string suitable for report output.
#[must_use]
pub fn extract_diagnostics(output: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;

    for line in output.lines() {
        let trimmed = line.trim();
        if is_warning_header(trimmed) || is_error_header(trimmed) {
            if !out.is_empty() {
                out.push('\n');
            }
            in_block = true;
        }
        if in_block {
            out.push_str(line);
            out.push('\n');
            // End block on blank line after we've collected at least the header
            if trimmed.is_empty() {
                in_block = false;
            }
        }
    }

    if out.is_empty() {
        "(no diagnostics captured)".to_string()
    } else {
        out.trim_end().to_string()
    }
}

/// Returns true if the line is a `warning: <msg>` diagnostic header (not a summary line).
#[must_use]
fn is_warning_header(line: &str) -> bool {
    if !line.starts_with("warning:") {
        return false;
    }
    // Exclude summary lines: "warning: X warning(s) emitted"
    let rest = line["warning:".len()..].trim();
    !rest.starts_with(|c: char| c.is_ascii_digit())
}

/// Returns true if the line is an `error[...]: <msg>` or `error: <msg>` diagnostic header,
/// excluding "error: aborting due to ..." summary lines.
#[must_use]
fn is_error_header(line: &str) -> bool {
    if line.starts_with("error[") {
        return true;
    }
    if let Some(rest) = line.strip_prefix("error:") {
        let rest = rest.trim();
        // Exclude "error: aborting due to ..." and "error: could not compile ..."
        return !rest.starts_with("aborting") && !rest.starts_with("could not compile");
    }
    false
}

/// Scan all tracked Markdown-relevant paths under the workspace for invalid UTF-8
/// and common mojibake markers.
#[must_use]
pub fn lint_markdown_files(workspace: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    walk_markdown_tree(workspace, workspace, &mut findings);
    findings
}

fn walk_markdown_tree(root: &Path, dir: &Path, findings: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            findings
                .push(format!("{}: could not read directory: {e}", relative_display(root, dir)));
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                findings.push(format!(
                    "{}: could not enumerate directory entry: {e}",
                    relative_display(root, dir)
                ));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(e) => {
                findings.push(format!(
                    "{}: could not read file type: {e}",
                    relative_display(root, &path)
                ));
                continue;
            }
        };
        if file_type.is_dir() {
            if should_skip_dir(&path) {
                continue;
            }
            walk_markdown_tree(root, &path, findings);
        } else if file_type.is_file()
            && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        {
            findings.extend(lint_markdown_file(root, &path));
        }
    }
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target"))
}

fn lint_markdown_file(root: &Path, path: &Path) -> Vec<String> {
    let relative = relative_display(root, path);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return vec![format!("{relative}: could not read file: {e}")],
    };
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => return vec![format!("{relative}: invalid UTF-8 ({e})")],
    };

    let mut findings = Vec::new();
    let mut in_fenced_code_block = false;
    for (line_idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fenced_code_block = !in_fenced_code_block;
            continue;
        }
        if in_fenced_code_block {
            continue;
        }
        if let Some(marker) =
            MARKDOWN_MOJIBAKE_MARKERS.iter().find(|marker| line.contains(**marker))
        {
            findings.push(format!(
                "{relative}:{}: suspicious mojibake marker `{}`",
                line_idx + 1,
                marker
            ));
        }
    }
    findings
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).display().to_string().replace('\\', "/")
}

#[cfg(test)]
#[path = "quality_lint_test.rs"]
mod tests;
