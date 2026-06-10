//! CLI argument model and top-level dispatcher for xtask.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Forge workspace build and quality automation")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Resolve and reserve numbered vault documents
    Vault {
        #[command(subcommand)]
        command: crate::vault::VaultCommand,
    },
    /// Run workspace tests with triple-consumable output (human, script, agent)
    QualityTest {
        /// Write report to quality-test-report.txt
        #[arg(long)]
        report: bool,
        /// Include ignored tests (runs cargo test -- --include-ignored)
        #[arg(long = "include-ignore")]
        include_ignore: bool,
        /// Skip coverage run (by default, coverage is generated after tests pass)
        #[arg(long)]
        no_coverage: bool,
        /// Show full test output. Default: only show failures (saves tokens for agents)
        #[arg(long)]
        verbose: bool,
        /// Coverage threshold (%). Only show files below this. Default: 70
        #[arg(long, default_value = "70")]
        coverage_threshold: u8,
        /// Show full per-file coverage table. Default: only files with any metric < threshold + TOTAL
        #[arg(long)]
        complete_coverage_summary: bool,
        /// After coverage succeeds, generate an HTML report and open it in the browser.
        #[arg(long)]
        open: bool,
        /// Scope tests and coverage to a single package (mirrors `cargo test -p` / `cargo llvm-cov --package`).
        /// All other flags remain independent.
        #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
        package: Option<String>,
    },
    /// Run cargo clippy --workspace --all-targets --all-features -- -D warnings with
    /// triple-consumable output (human, script, agent)
    QualityLint {
        /// Write report to quality-lint-report.txt
        #[arg(long)]
        report: bool,
        /// Run Markdown UTF-8 / mojibake validation instead of Rust clippy lint
        #[arg(long, conflicts_with = "package")]
        markdown: bool,
        /// Scope lint to a single package (mirrors `cargo clippy -p`)
        #[arg(short = 'p', long = "package", value_name = "PACKAGE")]
        package: Option<String>,
        /// Also evaluate future rules (informational only; does not affect exit code)
        #[arg(long)]
        future: bool,
    },
    /// Run the full quality pipeline: fmt check → clippy → tests+coverage.
    /// Exits 0 only if all three steps pass.
    Quality {
        /// Write combined report to quality-report.txt
        #[arg(long)]
        report: bool,
        /// Apply `cargo fmt --all` before running the pipeline (default: check only)
        #[arg(long)]
        format: bool,
    },
    /// Run the full quality pipeline, then append a Statistics section.
    /// Default stats: tokei + cargo tree.
    /// Use --module-structure for targeted crate analysis.
    /// Exits 0 only if the quality pipeline passes.
    QualityStats {
        /// Write combined report to quality-stats-report.txt
        #[arg(long)]
        report: bool,
        /// Run ONLY `cargo modules structure -p <CRATE>` as the stats section.
        #[arg(long, value_name = "CRATE")]
        module_structure: Option<String>,
    },
}

pub(crate) async fn dispatch<I, T, F>(args: I, on_parse_error: F) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
    F: FnOnce(clap::Error) -> i32,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => return on_parse_error(e),
    };
    match cli.command {
        Commands::Vault { command } => crate::vault::run(command),
        Commands::QualityTest {
            report,
            include_ignore,
            no_coverage,
            verbose,
            coverage_threshold,
            complete_coverage_summary,
            open,
            package,
        } => {
            crate::quality_test_runner::run(crate::quality_test_runner::QualityTestConfig {
                write_report: report,
                include_ignore,
                no_coverage,
                verbose,
                coverage_threshold,
                complete_coverage_summary,
                open_browser: open,
                package: package.as_deref(),
            })
            .await
        }
        Commands::QualityLint { report, markdown, package, future } => {
            crate::quality_lint::run(report, package.as_deref(), markdown, future).await
        }
        Commands::Quality { report, format } => crate::quality::run(report, format).await,
        Commands::QualityStats { report, module_structure } => {
            crate::quality_stats::run(report, module_structure.as_deref()).await
        }
    }
}

#[cfg(test)]
#[path = "main_dispatcher_test.rs"]
mod tests;
