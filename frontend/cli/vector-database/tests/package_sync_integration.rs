//! Integration tests for the `package sync` CLI command.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::panic
)]

use runtime_io::{CommandBuilder, CommandExecutor, ProcessCommandExecutor};

type CmdOutput = (String, String);

fn bin_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::current_exe().expect("cannot locate test executable");
    p.pop();
    if p.file_name().is_some_and(|n| n == "deps") {
        p.pop();
    }
    p.push(if cfg!(windows) { format!("{name}.exe") } else { name.to_string() });
    p
}

async fn run_cmd(cmd: &str, args: &[&str], dir: &std::path::Path) -> CmdOutput {
    let executor = ProcessCommandExecutor::default();
    let spec = CommandBuilder::new(cmd)
        .args(args.iter().map(|s| (*s).to_string()))
        .current_dir(dir)
        .build()
        .expect("failed to build command spec");
    let mut handle = executor.spawn(spec).await.expect("failed to spawn command");
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    handle
        .stream_output(
            &mut |b| stdout_bytes.extend_from_slice(b),
            &mut |b| stderr_bytes.extend_from_slice(b),
        )
        .await;
    let exit = handle.wait().await.expect("failed to wait for command");
    let stderr = String::from_utf8_lossy(&stderr_bytes).into_owned();
    assert!(exit.success, "command {cmd} {args:?} failed:\n{stderr}");
    (String::from_utf8_lossy(&stdout_bytes).into_owned(), stderr)
}

#[tokio::test]
async fn test_package_sync_cli_integration_with_pre_messages() {
    let bin = bin_path("vector-database").to_string_lossy().into_owned();

    // 1. Setup root directory
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    std::fs::create_dir_all(root.join(".vector")).unwrap();

    // 2. Setup mock git source repository
    let git_source_dir = temp.path().join("source_git_repo");
    std::fs::create_dir_all(git_source_dir.join(".vector")).unwrap();
    std::fs::create_dir_all(git_source_dir.join("doc")).unwrap();
    std::fs::write(git_source_dir.join(".vector/keep"), b"").unwrap();
    std::fs::write(git_source_dir.join("doc/readme.md"), b"Hello Git package").unwrap();

    run_cmd("git", &["init"], &git_source_dir).await;
    run_cmd("git", &["checkout", "-b", "main"], &git_source_dir).await;
    run_cmd("git", &["config", "user.name", "test"], &git_source_dir).await;
    run_cmd("git", &["config", "user.email", "test@example.com"], &git_source_dir).await;
    run_cmd("git", &["add", "."], &git_source_dir).await;
    run_cmd("git", &["commit", "-m", "initial commit"], &git_source_dir).await;

    // 3. Setup mock file source repository
    let file_source_dir = temp.path().join("source_file_repo");
    std::fs::create_dir_all(file_source_dir.join(".vector")).unwrap();
    std::fs::create_dir_all(file_source_dir.join("doc")).unwrap();
    std::fs::write(file_source_dir.join(".vector/keep"), b"").unwrap();
    std::fs::write(file_source_dir.join("doc/readme.md"), b"Hello File package").unwrap();

    // 4. Create packages.yaml manifest
    let manifest_yaml = format!(
        "pkg_file:\n  type: file\n  url: {}\npkg_git:\n  type: git\n  url: {}\n  tag: branch:main\n",
        file_source_dir.to_string_lossy().replace('\\', "/"),
        git_source_dir.to_string_lossy().replace('\\', "/")
    );
    std::fs::write(root.join(".vector/packages.yaml"), manifest_yaml).unwrap();

    // 5. Run the precompiled binary — avoids Cargo build lock deadlock
    let (stdout_str, stderr_str) = run_cmd(&bin, &["package", "sync"], root).await;

    println!("--- CLI stdout ---\n{stdout_str}");
    println!("--- CLI stderr ---\n{stderr_str}");

    assert!(
        stdout_str.contains("copying package pkg_file from url"),
        "expected copying pre-message in: {stdout_str}"
    );
    assert!(
        stdout_str.contains("cloning package pkg_git from url"),
        "expected cloning pre-message in: {stdout_str}"
    );

    let synced_file_doc = root.join(".vector-database/packages/pkg_file/doc/readme.md");
    let synced_git_doc = root.join(".vector-database/packages/pkg_git/doc/readme.md");
    assert!(synced_file_doc.exists(), "pkg_file doc should exist");
    assert!(synced_git_doc.exists(), "pkg_git doc should exist");

    // 6. Test refresh behavior
    std::fs::write(git_source_dir.join("doc/readme.md"), b"Hello Git package updated").unwrap();
    run_cmd("git", &["add", "."], &git_source_dir).await;
    run_cmd("git", &["commit", "-m", "update doc"], &git_source_dir).await;

    let (stdout_ref_str, stderr_ref_str) = run_cmd(&bin, &["package", "sync"], root).await;

    println!("--- CLI second run stdout ---\n{stdout_ref_str}");
    println!("--- CLI second run stderr ---\n{stderr_ref_str}");

    assert!(
        stdout_ref_str.contains("fetching package pkg_git from url"),
        "expected fetching pre-message in: {stdout_ref_str}"
    );
    assert!(
        stdout_ref_str.contains("copying package pkg_file from url"),
        "expected copying pre-message in: {stdout_ref_str}"
    );

    let git_doc_content = std::fs::read_to_string(synced_git_doc).unwrap();
    assert_eq!(git_doc_content, "Hello Git package updated");
}
