//! Command implementation to sync packages defined in the manifest.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use runtime_core::operation::FlowOperation;
use runtime_io::path::IoPath;
use runtime_io::{CommandBuilder, CommandExecutor, CommandSpec};
use runtime_packages::operations::sync_packages::{
    SyncCommandType, SyncPackagesInput, SyncPackagesOp, SyncPackagesOutput,
};
use runtime_packages::types::load_manifest;

#[derive(Default)]
struct CollectSyncSender {
    output: Option<SyncPackagesOutput>,
}

impl runtime_core::channel::Sender<SyncPackagesOutput> for CollectSyncSender {
    async fn send(&mut self, value: SyncPackagesOutput) -> runtime_core::RuntimeResult<()> {
        self.output = Some(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<SyncPackagesOutput> for CollectSyncSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Runs the package synchronization process.
///
/// # Errors
///
/// Returns an error message if any part of the synchronization planning or execution fails.
#[allow(clippy::too_many_lines)]
pub async fn run<E>(executor: &E, root_dir: &std::path::Path) -> Result<(), String>
where
    E: CommandExecutor + Sync,
{
    let io_root = IoPath::new(root_dir);
    let manifest =
        load_manifest(&io_root).await.map_err(|e| format!("failed to load manifest: {e}"))?;

    let input = SyncPackagesInput::new(io_root.clone());
    let mut sender = CollectSyncSender::default();
    SyncPackagesOp::new()
        .run(input, &mut sender)
        .await
        .map_err(|e| format!("sync-packages planning failed: {e}"))?;

    let output = sender.output.ok_or_else(|| "sync-packages did not produce output".to_string())?;
    let packages_dir = root_dir.join(".vector-database").join("packages");

    for action in output.actions {
        let entry = manifest
            .packages
            .get(&action.name)
            .ok_or_else(|| format!("package '{}' not found in manifest", action.name))?;

        // Print pre-message before each command execution
        match action.command_type {
            SyncCommandType::Clone => {
                println!("cloning package {} from url {}", action.name, entry.url);
            }
            SyncCommandType::Fetch => {
                println!("fetching package {} from url {}", action.name, entry.url);
            }
            SyncCommandType::Copy => {
                println!("copying package {} from url {}", action.name, entry.url);
            }
            _ => {}
        }

        let target_dir = packages_dir.join(&action.name);
        match action.command_type {
            SyncCommandType::Clone => {
                if let Some(parent) = target_dir.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("failed to create packages directory: {e}"))?;
                }

                let spec = CommandBuilder::new("git")
                    .arg("clone")
                    .arg(&entry.url)
                    .arg(target_dir.to_string_lossy().to_string())
                    .build()
                    .map_err(|e| format!("failed to build git clone spec: {e}"))?;
                execute_command(executor, spec).await?;

                let target = entry.tag.as_ref().map_or("main", |tag_val| {
                    tag_val.strip_prefix("branch:").map_or(tag_val.as_str(), |branch| branch.trim())
                });

                let spec = CommandBuilder::new("git")
                    .arg("checkout")
                    .arg(target)
                    .current_dir(&target_dir)
                    .build()
                    .map_err(|e| format!("failed to build git checkout spec: {e}"))?;
                execute_command(executor, spec).await?;
            }
            SyncCommandType::Fetch => {
                let spec = CommandBuilder::new("git")
                    .arg("fetch")
                    .current_dir(&target_dir)
                    .build()
                    .map_err(|e| format!("failed to build git fetch spec: {e}"))?;
                execute_command(executor, spec).await?;

                let target = entry.tag.as_ref().map_or("main", |tag_val| {
                    tag_val.strip_prefix("branch:").map_or(tag_val.as_str(), |branch| branch.trim())
                });

                let spec = CommandBuilder::new("git")
                    .arg("checkout")
                    .arg(target)
                    .current_dir(&target_dir)
                    .build()
                    .map_err(|e| format!("failed to build git checkout spec: {e}"))?;
                execute_command(executor, spec).await?;

                if entry.tag.as_ref().is_some_and(|tag_val| tag_val.starts_with("branch:")) {
                    let remote_ref = format!("origin/{target}");
                    let spec = CommandBuilder::new("git")
                        .arg("reset")
                        .arg("--hard")
                        .arg(&remote_ref)
                        .current_dir(&target_dir)
                        .build()
                        .map_err(|e| format!("failed to build git reset spec: {e}"))?;
                    execute_command(executor, spec).await?;
                }
            }
            SyncCommandType::Copy => {
                let src_path = if std::path::Path::new(&entry.url).is_absolute() {
                    std::path::PathBuf::from(&entry.url)
                } else {
                    root_dir.join(&entry.url)
                };

                if target_dir.exists() {
                    std::fs::remove_dir_all(&target_dir)
                        .map_err(|e| format!("failed to clear target directory: {e}"))?;
                }

                let spec = if cfg!(windows) {
                    CommandBuilder::new("xcopy")
                        .arg(src_path.to_string_lossy().to_string())
                        .arg(target_dir.to_string_lossy().to_string())
                        .arg("/E")
                        .arg("/Y")
                        .arg("/I")
                        .build()
                        .map_err(|e| format!("failed to build xcopy spec: {e}"))?
                } else {
                    if let Some(parent) = target_dir.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("failed to create packages directory: {e}"))?;
                    }
                    CommandBuilder::new("cp")
                        .arg("-R")
                        .arg(src_path.to_string_lossy().to_string())
                        .arg(target_dir.to_string_lossy().to_string())
                        .build()
                        .map_err(|e| format!("failed to build cp spec: {e}"))?
                };
                execute_command(executor, spec).await?;
            }
            _ => return Err("unsupported sync command type".to_string()),
        }
    }

    Ok(())
}

async fn execute_command<E>(executor: &E, spec: CommandSpec) -> Result<(), String>
where
    E: CommandExecutor + Sync,
{
    use std::io::Write;
    let mut handle =
        executor.spawn(spec).await.map_err(|e| format!("command spawn failed: {e}"))?;

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();

    let mut on_stdout = |b: &[u8]| {
        let _ = stdout.write_all(b);
        let _ = stdout.flush();
    };
    let mut on_stderr = |b: &[u8]| {
        let _ = stderr.write_all(b);
        let _ = stderr.flush();
    };

    handle.stream_output(&mut on_stdout, &mut on_stderr).await;

    let exit = handle.wait().await.map_err(|e| format!("failed waiting for process: {e}"))?;
    if exit.success {
        Ok(())
    } else {
        Err(format!("command failed with exit code {:?}", exit.code))
    }
}

#[cfg(test)]
#[path = "package_sync_test.rs"]
mod tests;
