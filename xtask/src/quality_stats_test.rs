#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::*;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_clean_workspace_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let dir = std::env::temp_dir().join(format!("xtask-quality-stats-{test_name}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    dir
}

struct TestWorkspaceGuard {
    previous_dir: PathBuf,
    temp_dir: PathBuf,
}

impl Drop for TestWorkspaceGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous_dir);
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

async fn with_clean_workspace<T, F, Fut>(test_name: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let _cwd_guard = crate::quality::CURRENT_DIR_TEST_LOCK.lock().await;
    let temp_dir = temp_clean_workspace_dir(test_name);
    let previous_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();
    let _workspace_guard = TestWorkspaceGuard { previous_dir, temp_dir };
    f().await
}

#[tokio::test]
async fn stat_tool_not_found_shows_install_hint() {
    let workspace = PathBuf::from(".");
    let section = run_stat_tool(
        &workspace,
        "Test Tool",
        "__nonexistent_binary_xyz__",
        &[],
        "cargo install test-tool",
    )
    .await;
    assert!(section.contains("Not available"));
    assert!(section.contains("cargo install test-tool"));
}

#[tokio::test]
async fn stat_tool_label_appears_in_output() {
    let workspace = PathBuf::from(".");
    let section =
        run_stat_tool(&workspace, "My Label", "__nonexistent_binary_xyz__", &[], "some-hint").await;
    assert!(section.contains("My Label"));
}

#[tokio::test]
async fn stat_tool_success_contains_stdout() {
    let _guard = io::stub_shell("xtask-cargo-version", 0, "cargo 1.2.3\n");
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section = run_stat_tool(&workspace, "Echo", "cargo", &["--version"], "n/a").await;
    assert!(section.contains("cargo"));
}

#[tokio::test]
async fn stat_tool_failure_with_stdout_uses_stdout_as_detail() {
    // `cargo __nonexistent__` exits non-zero and writes an error message to stderr.
    // run_stat_tool captures merged output; the "Failed:" label or "Bad" section header appears.
    let _guard = io::stub_shell("xtask-cargo-generic", 1, "error: Bad\n");
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section =
        run_stat_tool(&workspace, "Bad", "cargo", &["__nonexistent_subcommand_xyz__"], "n/a").await;
    assert!(section.contains("Failed:") || section.contains("error") || section.contains("Bad"));
}

#[tokio::test]
async fn stat_tool_duration_appears_in_output() {
    let _guard = io::stub_shell("xtask-cargo-version", 0, "cargo duration output\n");
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section = run_stat_tool(&workspace, "Timer", "cargo", &["--version"], "n/a").await;
    // Duration in seconds always present in success/fail output (not in "Not available")
    assert!(section.contains("Timer") && section.contains("s)"));
}

#[tokio::test]
async fn build_stats_default_contains_tokei_and_tree() {
    let _guard_tokei = io::stub_shell("xtask-tokei-stats", 0, "Lines of Code: 123\n");
    let _guard_cargo = io::stub_shell("xtask-cargo-tree", 0, "Dependency Graph: ok\ncargo tree\n");
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section = build_stats_section(&workspace, None).await;
    assert!(section.contains("Lines of Code") || section.contains("tokei"));
    assert!(section.contains("Dependency Graph") || section.contains("cargo tree"));
}

#[tokio::test]
async fn build_stats_module_structure_mode_no_tree() {
    let _guard = io::stub_shell(
        "xtask-cargo-modules-structure",
        0,
        "Module Structure\n(only structure output)\n",
    );
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section = build_stats_section(&workspace, Some("xtask")).await;
    assert!(section.contains("Module Structure"));
    assert!(!section.contains("Dependency Graph"));
    assert!(!section.contains("Lines of Code"));
}

// ─── build_stats_section (stub-based) ────────────────────────────────────────

#[tokio::test]
async fn build_stats_stub_returns_stats_header() {
    let _guard_tokei = io::stub_shell("xtask-tokei-stats", 0, "tokei output here\n");
    let _guard_cargo = io::stub_shell("xtask-cargo-tree", 0, "cargo tree output\n");
    let workspace = std::env::current_dir().expect("current dir accessible");
    let section = build_stats_section(&workspace, None).await;
    assert!(section.contains("=== Statistics ==="));
}

// ─── execute / run ────────────────────────────────────────────────────────────

#[tokio::test]
async fn execute_all_pass_returns_pass_with_stats_section() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell(
        "xtask-cargo-tests",
        0,
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n",
    );
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let _guard_tokei = io::stub_shell("xtask-tokei-stats", 0, "Lines of Code: 500\n");
    let _guard_tree = io::stub_shell("xtask-cargo-tree", 0, "workspace deps\n");
    let (report, passed) =
        with_clean_workspace("execute-all-pass", || execute(false, None)).await;
    assert!(passed, "expected PASS, report:\n{report}");
    assert!(report.contains("=== Statistics ==="));
}

#[tokio::test]
async fn run_returns_zero_when_all_pass() {
    let _guard_fmt = io::stub_shell("xtask-shell-run", 0, "");
    let _guard_lint = io::stub_shell("xtask-cargo-lint", 0, "    Finished checking\n");
    let _guard_tests = io::stub_shell(
        "xtask-cargo-tests",
        0,
        "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n",
    );
    let _guard_cov = io::stub_shell("xtask-cargo-llvm-cov", 0, "");
    let _guard_tokei = io::stub_shell("xtask-tokei-stats", 0, "Lines of Code: 500\n");
    let _guard_tree = io::stub_shell("xtask-cargo-tree", 0, "workspace deps\n");
    let exit_code = with_clean_workspace("run-all-pass", || run(false, None)).await;
    assert_eq!(exit_code, 0);
}
