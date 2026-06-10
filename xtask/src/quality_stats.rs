//! Quality-stats command: full quality pipeline + statistics section.
//!
//! Default: appends tokei + cargo tree after the quality pipeline.
//! `--module-structure <crate>`: runs ONLY `cargo modules structure -p <crate>` as the stats section.

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use io::CommandBuilder;

/// Report string + pass/fail status.
pub type StatsReport = (String, bool);

/// Run the full quality pipeline, then append a Statistics section.
/// Optional `module_structure` bypasses full quality and only dumps structure for one crate.
pub async fn execute(write_report: bool, module_structure_crate: Option<&str>) -> StatsReport {
    let workspace = env::current_dir()
        .unwrap_or_else(|_| unreachable!("current_dir() cannot fail in normal operation"));

    // ── Quality pipeline ─────────────────────────────────────────────────────
    let (quality_report, quality_passed) = crate::quality::execute(false, false).await;

    // ── Statistics section ────────────────────────────────────────────────────
    let stats_section = build_stats_section(&workspace, module_structure_crate).await;

    let report = format!("{quality_report}{stats_section}");

    if write_report {
        let report_path = workspace.join("quality-stats-report.txt");
        if let Err(e) = std::fs::write(&report_path, &report) {
            eprintln!("Warning: could not write report to {}: {}", report_path.display(), e);
        }
    }

    (report, quality_passed)
}

/// Run quality + stats pipeline. Returns exit code 0 if all quality steps passed.
pub async fn run(write_report: bool, module_structure_crate: Option<&str>) -> i32 {
    let (report, passed) = execute(write_report, module_structure_crate).await;
    print!("{report}");
    if passed { 0 } else { 1 }
}

/// Build the statistics section string.
///
/// - `module_structure_crate` set → only runs `cargo modules structure -p <crate>`.
/// - `None` → runs tokei + cargo tree (default).
pub async fn build_stats_section(
    workspace: &PathBuf,
    module_structure_crate: Option<&str>,
) -> String {
    let mut out = String::from("=== Statistics ===\n");

    if let Some(krate) = module_structure_crate {
        out.push_str(
            &run_stat_tool(
                workspace,
                &format!("Module Structure ({krate})"),
                "cargo",
                &["modules", "structure", "-p", krate],
                "cargo install cargo-modules",
            )
            .await,
        );
    } else {
        // Default: tokei + dependency graph
        out.push_str(
            &run_stat_tool(workspace, "Lines of Code (tokei)", "tokei", &[], "cargo install tokei")
                .await,
        );
        out.push('\n');
        out.push_str(
            &run_stat_tool(
                workspace,
                "Dependency Graph (cargo tree)",
                "cargo",
                &["tree", "--workspace"],
                "built-in (cargo tree)",
            )
            .await,
        );
    }

    out
}

/// Run a single statistics tool and return a formatted section.
/// If the tool is not found or fails, a note is included instead of the output.
async fn run_stat_tool(
    workspace: &PathBuf,
    label: &str,
    bin: &str,
    args: &[&str],
    install_hint: &str,
) -> String {
    let start = Instant::now();

    // Stable semantic intention keys used by `runtime/io` for deterministic stubbing in tests.
    let semantic_key = if bin == "tokei" {
        "xtask-tokei-stats".to_string()
    } else if bin == "cargo" {
        match args {
            ["modules", "structure", ..] => "xtask-cargo-modules-structure".to_string(),
            ["tree", ..] => "xtask-cargo-tree".to_string(),
            ["--version"] => "xtask-cargo-version".to_string(),
            _ => "xtask-cargo-generic".to_string(),
        }
    } else {
        // For unknown tools, keep keys unique per binary to avoid accidental stub
        // interception across parallel tests.
        format!("xtask-tool-{bin}")
    };

    let mut exec = match CommandBuilder::new(semantic_key, bin)
        .args(args.iter().copied())
        .workdir(workspace)
        .run()
    {
        Err(e) => {
            let msg = e.to_string();
            return if msg.contains("os error 2")
                || msg.contains("No such file")
                || msg.contains("cannot find the file")
                || msg.contains("program not found")
            {
                format!("--- {label} ---\nNot available: install with `{install_hint}`\n")
            } else {
                format!("--- {label} ---\nError: {msg}\n")
            };
        }
        Ok(e) => e,
    };

    let mut combined = String::new();
    let _ = std::io::Read::read_to_string(&mut exec.output, &mut combined);
    let exit_code = exec.wait().await.unwrap_or(1);
    let duration = start.elapsed().as_secs_f64();

    if exit_code == 0 {
        let content = combined.trim().to_string();
        format!("--- {label} ({duration:.2}s) ---\n{content}\n")
    } else {
        let detail = combined.trim().to_string();
        format!("--- {label} ({duration:.2}s) ---\nFailed:\n{detail}\n")
    }
}

#[cfg(test)]
#[path = "quality_stats_test.rs"]
mod tests;
