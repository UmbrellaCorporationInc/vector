#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn passing_test_output() -> &'static str {
    "Running unittests src/lib.rs (target/debug/deps/mylib-abc123)\n\
     test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s\n"
}

// ─── dispatch: happy paths ────────────────────────────────────────────────────

#[tokio::test]
async fn dispatch_quality_test_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    assert_eq!(dispatch(["xtask", "quality-test", "--no-coverage"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_lint_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    assert_eq!(dispatch(["xtask", "quality-lint"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_lint_markdown_returns_zero_on_pass() {
    assert_eq!(dispatch(["xtask", "quality-lint", "--markdown"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_returns_zero_on_pass() {
    let _g_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _g_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _g_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let _g_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    assert_eq!(dispatch(["xtask", "quality"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_stats_returns_zero_on_pass() {
    let _g_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _g_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _g_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let _g_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let _g_tokei = io::stub_shell("xtask-tokei-stats", 0, "Lines of Code: 100\n");
    let _g_tree = io::stub_shell("xtask-cargo-tree", 0, "workspace deps\n");
    assert_eq!(dispatch(["xtask", "quality-stats"], |e| e.exit()).await, 0);
}

// ─── dispatch: -p / --package flag ───────────────────────────────────────────

#[tokio::test]
async fn dispatch_quality_test_with_package_flag_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    assert_eq!(
        dispatch(["xtask", "quality-test", "-p", "my_crate", "--no-coverage"], |e| e.exit()).await,
        0
    );
}

#[tokio::test]
async fn dispatch_quality_test_with_package_long_flag_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    assert_eq!(
        dispatch(["xtask", "quality-test", "--package", "my_crate", "--no-coverage"], |e| e.exit())
            .await,
        0
    );
}

// ─── dispatch: quality-lint -p / --package flag ───────────────────────────────

#[tokio::test]
async fn dispatch_quality_lint_with_package_flag_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    assert_eq!(dispatch(["xtask", "quality-lint", "-p", "my_crate"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_lint_with_package_long_flag_returns_zero_on_pass() {
    let _g = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    assert_eq!(dispatch(["xtask", "quality-lint", "--package", "my_crate"], |e| e.exit()).await, 0);
}

#[tokio::test]
async fn dispatch_quality_lint_rejects_markdown_with_package() {
    let code =
        dispatch(["xtask", "quality-lint", "--markdown", "--package", "my_crate"], |_| 2).await;
    assert_eq!(code, 2);
}

// ─── dispatch: quality --format flag ─────────────────────────────────────────

#[tokio::test]
async fn dispatch_quality_with_format_flag_returns_zero_on_pass() {
    let _g_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _g_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _g_tests = io::stub_shell("xtask-cargo-tests", 0, passing_test_output());
    let _g_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    assert_eq!(dispatch(["xtask", "quality", "--format"], |e| e.exit()).await, 0);
}

// ─── dispatch: parse error path ───────────────────────────────────────────────

#[tokio::test]
async fn dispatch_returns_on_parse_error_with_custom_handler() {
    let code = dispatch(["xtask", "unknown-command"], |_| 2).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn dispatch_vault_research_rejects_missing_subfolder_on_reserve() {
    let code = dispatch(["xtask", "vault", "reserve", "research", "Example Research"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_roadmap_rejects_missing_subfolder_on_reserve() {
    let code = dispatch(["xtask", "vault", "reserve", "roadmap", "Example Roadmap"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_returns_on_parse_error_for_unknown_vault_subcommand() {
    let code = dispatch(["xtask", "vault", "unknown"], |_| 2).await;
    assert_eq!(code, 2);
}

#[tokio::test]
async fn dispatch_vault_guide_without_id_is_accepted_by_parser() {
    let code = dispatch(["xtask", "vault", "guide"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_roadmap_without_id_is_accepted_by_parser() {
    let code = dispatch(["xtask", "vault", "roadmap"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_query_without_expression_returns_zero() {
    let code = dispatch(["xtask", "vault", "query"], |_| 2).await;
    assert_eq!(code, 0);
}

#[tokio::test]
async fn dispatch_vault_query_with_expression_fails_without_workspace_vault_yaml() {
    // Valid parse, but `load_config` needs `.cargo/.xtask/vault.yaml` from cwd (often the `xtask/` crate when testing).
    let code = dispatch(["xtask", "vault", "query", "{type='task'}"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_query_with_query_id_fails_without_named_query_file() {
    let code = dispatch(["xtask", "vault", "query", "--query-id", "demo"], |_| 2).await;
    assert_eq!(code, 1);
}

#[tokio::test]
async fn dispatch_vault_query_malformed_expression_returns_one() {
    let code = dispatch(["xtask", "vault", "query", "{"], |_| 2).await;
    assert_eq!(code, 1);
}
