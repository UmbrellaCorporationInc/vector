#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_io::{
    CommandBuilder, CommandExit, CommandHandle, IoError, MockCommandHandleBuilder,
    ProcessCommandExecutor,
};
use runtime_packages::types::{PackageManifest, save_manifest};
use std::collections::VecDeque;
use std::future::{Future, ready};
use std::sync::Mutex;

#[derive(Debug, Clone)]
struct MockCommand {
    command: String,
    args: Vec<String>,
    current_dir: Option<std::path::PathBuf>,
}

struct MockExecutor {
    responses: Mutex<VecDeque<Result<CommandHandle, IoError>>>,
    recorded: Mutex<Vec<MockCommand>>,
    create_dirs: bool,
}

impl MockExecutor {
    fn new(responses: Vec<Result<CommandHandle, IoError>>, create_dirs: bool) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from(responses)),
            recorded: Mutex::new(Vec::new()),
            create_dirs,
        }
    }

    fn recorded_commands(&self) -> Vec<MockCommand> {
        self.recorded.lock().expect("recorded lock").clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = Result<CommandHandle, IoError>> + Send {
        self.recorded.lock().expect("recorded lock").push(MockCommand {
            command: spec.command().to_owned(),
            args: spec.args().to_vec(),
            current_dir: spec.current_dir().map(std::path::Path::to_path_buf),
        });

        if self.create_dirs {
            for arg in spec.args() {
                if arg.contains("packages") {
                    let path = std::path::PathBuf::from(arg);
                    let _ = std::fs::create_dir_all(path.join("doc"));
                    let _ = std::fs::create_dir_all(path.join(".vector"));
                }
            }
        }

        let result = self
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err(IoError::Process("mock executor exhausted".into())));
        ready(result)
    }
}

fn success_handle() -> CommandHandle {
    MockCommandHandleBuilder::new(CommandExit::new(true, Some(0))).build().0
}

#[tokio::test]
async fn test_package_sync_empty_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let manifest = PackageManifest::parse("{}").unwrap();
    save_manifest(&IoPath::new(root), &manifest).await.unwrap();

    let executor = MockExecutor::new(vec![], false);
    run(&executor, root).await.expect("empty manifest should succeed");

    assert!(executor.recorded_commands().is_empty());
}

#[tokio::test]
async fn test_package_sync_executes_expected_actions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let manifest_yaml = r"
pkg_git:
  type: git
  url: https://github.com/org/pkg_git.git
  tag: branch:develop
pkg_file:
  type: file
  url: ./local/pkg_file
";
    let manifest = PackageManifest::parse(manifest_yaml).unwrap();
    save_manifest(&IoPath::new(root), &manifest).await.unwrap();

    let executor = MockExecutor::new(
        vec![Ok(success_handle()), Ok(success_handle()), Ok(success_handle())],
        true,
    );

    run(&executor, root).await.expect("sync should succeed");

    let cmds = executor.recorded_commands();
    assert_eq!(cmds.len(), 3);

    // Command 1: copy (pkg_file alphabetically comes first)
    if cfg!(windows) {
        assert_eq!(cmds[0].command, "xcopy");
        assert_eq!(cmds[0].args[2], "/E");
    } else {
        assert_eq!(cmds[0].command, "cp");
        assert_eq!(cmds[0].args[0], "-R");
    }

    // Command 2: git clone
    assert_eq!(cmds[1].command, "git");
    assert_eq!(cmds[1].args[0], "clone");
    assert_eq!(cmds[1].args[1], "https://github.com/org/pkg_git.git");
    assert_eq!(cmds[1].current_dir, None);

    // Command 3: git checkout
    assert_eq!(cmds[2].command, "git");
    assert_eq!(cmds[2].args[0], "checkout");
    assert_eq!(cmds[2].args[1], "develop");
    assert!(cmds[2].current_dir.is_some());
    assert!(cmds[2].current_dir.as_ref().unwrap().ends_with("pkg_git"));
}

#[tokio::test]
async fn test_package_sync_invalid_contract_fails_and_cleans_up() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let manifest_yaml = r"
pkg_git:
  type: git
  url: https://github.com/org/pkg_git.git
  tag: branch:develop
";
    let manifest = PackageManifest::parse(manifest_yaml).unwrap();
    save_manifest(&IoPath::new(root), &manifest).await.unwrap();

    let executor = MockExecutor::new(vec![Ok(success_handle()), Ok(success_handle())], false);

    // We do NOT create doc/ and .vector/ under pkg_git target.
    // The command executes, but validation should fail and directory should be deleted.
    let result = run(&executor, root).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("does not satisfy the minimum repository contract"));

    let pkg_git_dir = root.join(".vector-database").join("packages").join("pkg_git");
    assert!(!pkg_git_dir.exists(), "directory should have been deleted/cleaned up");
}

#[tokio::test]
async fn test_package_sync_cli_integration_with_pre_messages() {
    // 1. Setup root directory
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    // Create ".vector" inside root so find_root_dir finds it
    std::fs::create_dir_all(root.join(".vector")).unwrap();

    // 2. Setup mock git source repository
    let git_source_dir = temp.path().join("source_git_repo");
    std::fs::create_dir_all(git_source_dir.join(".vector")).unwrap();
    std::fs::create_dir_all(git_source_dir.join("doc")).unwrap();
    std::fs::write(git_source_dir.join(".vector/keep"), b"").unwrap();
    std::fs::write(git_source_dir.join("doc/readme.md"), b"Hello Git package").unwrap();

    // Initialize git repository
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

    // 4. Create packages.yaml manifest in the root's .vector directory
    let manifest_yaml = format!(
        "pkg_file:\n  type: file\n  url: {}\npkg_git:\n  type: git\n  url: {}\n  tag: branch:main\n",
        file_source_dir.to_string_lossy().replace('\\', "/"),
        git_source_dir.to_string_lossy().replace('\\', "/")
    );
    std::fs::write(root.join(".vector/packages.yaml"), manifest_yaml).unwrap();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_or_else(|_| std::env::current_dir().unwrap(), std::path::PathBuf::from);
    let manifest_path_str = manifest_dir.join("Cargo.toml").to_string_lossy().into_owned();

    // 5. Run vector-database package sync CLI as a real subprocess
    let (stdout_str, stderr_str) = run_cmd(
        "cargo",
        &[
            "run",
            "--manifest-path",
            &manifest_path_str,
            "--bin",
            "vector-database",
            "--",
            "package",
            "sync",
        ],
        root,
    )
    .await;

    println!("--- CLI stdout ---");
    println!("{stdout_str}");
    println!("--- CLI stderr ---");
    println!("{stderr_str}");

    // Verify expected pre-messages are in stdout
    assert!(
        stdout_str.contains("copying package pkg_file from url"),
        "expected copying pre-message in: {stdout_str}"
    );
    assert!(
        stdout_str.contains("cloning package pkg_git from url"),
        "expected cloning pre-message in: {stdout_str}"
    );

    // Verify packages exist and satisfy contract
    let synced_file_doc = root.join(".vector-database/packages/pkg_file/doc/readme.md");
    let synced_git_doc = root.join(".vector-database/packages/pkg_git/doc/readme.md");
    assert!(synced_file_doc.exists(), "pkg_file doc should exist");
    assert!(synced_git_doc.exists(), "pkg_git doc should exist");

    // 6. Test refresh behavior (fetch)
    // Modify git source
    std::fs::write(git_source_dir.join("doc/readme.md"), b"Hello Git package updated").unwrap();
    run_cmd("git", &["add", "."], &git_source_dir).await;
    run_cmd("git", &["commit", "-m", "update doc"], &git_source_dir).await;

    // Run sync again
    let (stdout_ref_str, stderr_ref_str) = run_cmd(
        "cargo",
        &[
            "run",
            "--manifest-path",
            &manifest_path_str,
            "--bin",
            "vector-database",
            "--",
            "package",
            "sync",
        ],
        root,
    )
    .await;

    println!("--- CLI second run stdout ---");
    println!("{stdout_ref_str}");
    println!("--- CLI second run stderr ---");
    println!("{stderr_ref_str}");

    // Verify expected pre-messages for refresh in stdout
    assert!(
        stdout_ref_str.contains("fetching package pkg_git from url"),
        "expected fetching pre-message in: {stdout_ref_str}"
    );
    assert!(
        stdout_ref_str.contains("copying package pkg_file from url"),
        "expected copying pre-message in: {stdout_ref_str}"
    );

    // Verify git package was updated successfully
    let git_doc_content = std::fs::read_to_string(synced_git_doc).unwrap();
    assert_eq!(git_doc_content, "Hello Git package updated");
}

async fn run_cmd(cmd: &str, args: &[&str], dir: &std::path::Path) -> (String, String) {
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
            &mut |b| {
                stdout_bytes.extend_from_slice(b);
            },
            &mut |b| {
                stderr_bytes.extend_from_slice(b);
            },
        )
        .await;
    let exit = handle.wait().await.expect("failed to wait for command");
    let stderr_str = String::from_utf8_lossy(&stderr_bytes).into_owned();
    assert!(exit.success, "command {cmd} {args:?} failed: {stderr_str}");
    (String::from_utf8_lossy(&stdout_bytes).into_owned(), stderr_str)
}
