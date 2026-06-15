#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use runtime_io::{CommandExit, CommandHandle, IoError, MockCommandHandleBuilder};
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
