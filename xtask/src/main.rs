//! Xtask: build and quality automation for the Forge workspace.
//!
//! Commands: quality-test, quality-lint, quality, quality-stats

mod lint_rules;
mod main_dispatcher;
mod quality;
mod quality_lint;
mod quality_stats;
mod quality_test_runner;
mod vault;
mod vault_query;

#[tokio::main]
async fn main() {
    std::process::exit(main_dispatcher::dispatch(std::env::args_os(), |e| e.exit()).await);
}
