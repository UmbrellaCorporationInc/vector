//! Shell module for deterministic async process execution.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::{FsError, Reader};

/// Ergonomic shorthand for constructing a [`CommandBuilder`].
///
/// # Examples
/// ```ignore
/// let out = command!("echo", "hello").run().await?;
/// let out = command!("echo", "hello").pipe(command!("grep", "hello")).run().await?;
/// ```
#[macro_export]
macro_rules! command {
    ($key:expr, $cmd:expr $(, $arg:expr)*) => {
        $crate::shell::CommandBuilder::new($key, $cmd)$(.arg($arg))*
    };
}

/// Configures the standard input source for a spawned process.
#[non_exhaustive]
pub enum InputSource {
    /// Discards stdin entirely (equivalent to redirecting from `/dev/null`).
    ///
    /// **Default.** Use for all non-interactive commands. Prevents the child
    /// from blocking indefinitely if the parent's stdin is a terminal or pipe.
    Null,
    /// Inherits the parent process's stdin verbatim.
    ///
    /// Use for interactive commands that require real user input (e.g. `fzf`, `ssh`).
    Stdio,
    /// Pipes the output of a completed [`Execution`] as this command's stdin.
    ///
    /// Enables shell-style `cmd_a | cmd_b` pipelines without shell interpolation.
    /// The previous execution's `output` reader is pumped through a background
    /// thread into an OS pipe, so the child receives a proper file descriptor.
    Execution(Box<Execution>),
}

/// Handle to a running or completed process.
///
/// Exposes the merged stdout/stderr stream via the public [`output`](Self::output) field.
/// Obtain one from [`CommandBuilder::run`] for real processes or from
/// [`Execution::new`] for mock/static byte streams.
///
/// # DTO(shell execution handle with merged output stream and exit code)
pub struct Execution {
    child: Option<tokio::process::Child>,
    exit_code: Option<i32>,
    /// Merged stdout + stderr stream. Reads block until the process produces output or exits.
    pub output: Reader,
}

impl Execution {
    /// Creates a static [`Execution`] backed by a pre-computed byte stream.
    ///
    /// No OS process is spawned. `output` is returned verbatim; `exit_code` is
    /// returned by [`wait`](Self::wait) without an OS wait call.
    #[must_use]
    pub fn new(exit_code: i32, output: Reader) -> Self {
        Self { child: None, exit_code: Some(exit_code), output }
    }

    /// Waits for the process to finish and returns its exit code.
    ///
    /// Automatically drains any remaining data from [`output`](Self::output)
    /// in a background thread before waiting, preventing pipe-saturation deadlocks.
    /// Consuming `self` ensures the process handle is definitively released.
    ///
    /// # Side Effects
    /// Spawns a blocking drain task. On completion, the underlying OS process is
    /// reaped and all associated file descriptors are closed.
    ///
    /// # Errors
    /// Returns [`FsError`] if the drain task panics, or if the OS wait call fails.
    pub async fn wait(mut self) -> Result<i32, FsError> {
        let mut output = self.output;
        let child = self.child.take();
        let fallback_exit = self.exit_code;

        // Drain the output
        tokio::task::spawn_blocking(move || {
            let mut sink = std::io::sink();
            std::io::copy(&mut output, &mut sink)
        })
        .await
        .map_err(|e| FsError::Io(format!("Drain task panicked: {e}")))?
        .map_err(|e| FsError::Io(format!("Failed to drain output: {e}")))?;

        // Resolve the error code
        if let Some(mut process) = child {
            process
                .wait()
                .await
                .map_err(|e| FsError::Io(format!("Failed to wait: {e}")))
                .map(|status| status.code().unwrap_or(1))
        } else {
            // Architecturally impossible: `child` is `None` only when this `Execution` was
            // constructed via `Execution::new(exit_code, …)`, which always supplies a
            // `Some(exit_code)`. A `None` exit_code paired with a `None` child cannot
            // arise through any public `Execution` or `CommandBuilder` call site.
            Ok(fallback_exit.unwrap_or_else(|| {
                unreachable!("Execution::wait: no child process and no static exit code — impossible by construction")
            }))
        }
    }
}

/// An executable shell command builder that enforces explicit execution boundaries.
pub struct CommandBuilder {
    /// Semantic intention key used for deterministic test stubbing.
    key: String,
    command: OsString,
    args: Vec<OsString>,
    workdir: Option<PathBuf>,
    env: HashMap<OsString, OsString>,
    clear_env: bool,
    stdin: InputSource,
}

impl CommandBuilder {
    /// Constructs a new `CommandBuilder`.
    ///
    /// `key` is a stable semantic intention used for deterministic test stubbing.
    /// `command` is the concrete executable/binary to spawn.
    #[must_use]
    pub fn new<K, S>(key: K, command: S) -> Self
    where
        K: Into<String>,
        S: Into<OsString>,
    {
        Self {
            key: key.into(),
            command: command.into(),
            args: Vec::new(),
            workdir: None,
            env: HashMap::new(),
            clear_env: false,
            stdin: InputSource::Null,
        }
    }

    /// Appends an argument to the execution vector.
    #[must_use]
    pub fn arg<S: Into<OsString>>(mut self, arg: S) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Appends multiple arguments to the execution vector.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Explicitly locks down the working directory of the process.
    #[must_use]
    pub fn workdir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.workdir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Injects an environment variable into the isolated process boundary.
    #[must_use]
    pub fn env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env.insert(key.into(), val.into());
        self
    }

    /// Injects multiple environment variables into the isolated process boundary.
    #[must_use]
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        self.env.extend(vars.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Whether to clear the host environment before executing.
    #[must_use]
    pub const fn clear_env(mut self, clear: bool) -> Self {
        self.clear_env = clear;
        self
    }

    /// Sets the stdin source for the spawned process.
    ///
    /// Defaults to [`InputSource::Null`]. Use [`InputSource::Stdio`] for interactive
    /// commands or [`InputSource::Execution`] to pipe from a previous command.
    #[must_use]
    pub fn stdin(mut self, source: InputSource) -> Self {
        self.stdin = source;
        self
    }

    /// Constructs a platform-aware shell command from a raw command string.
    ///
    /// On Windows, wraps `cmd /c <cmd>`. On Unix, wraps `sh -c <cmd>`.
    /// Use this instead of manually branching on `cfg!(windows)` at call sites.
    #[must_use]
    pub fn shell_command(key: &str, cmd: &str) -> Self {
        if cfg!(windows) {
            Self::new(key, "cmd").args(["/c", cmd])
        } else {
            Self::new(key, "sh").args(["-c", cmd])
        }
    }

    /// This method merges `stdout` and `stderr` into a single interleaved stream
    /// at the OS level, ensuring deterministic logging and preventing deadlocks
    /// caused by unread error buffers.
    ///
    /// # Side Effects
    /// Spawns an asynchronous OS process. The process is automatically terminated
    /// if the returned handle is dropped before calling [`Execution::wait`].
    ///
    /// # Errors
    /// Returns [`FsError`] if the binary cannot be found or the OS fails to
    /// allocate process resources.
    pub fn run(mut self) -> Result<Execution, FsError> {
        #[cfg(any(test, feature = "test-utils"))]
        {
            use base64::Engine;
            let normalized = normalize_stub_key(&self.key);
            let output_key = format!("FORGE_STUB_SHELL_{normalized}");
            if let Ok(encoded_val) = std::env::var(&output_key) {
                let exit_code_key = format!("FORGE_STUB_SHELL_EXIT_CODE_{normalized}");
                let exit_code = std::env::var(&exit_code_key)
                    .ok()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                let decoded = base64::engine::general_purpose::STANDARD
                    .decode(encoded_val)
                    .map_err(|e| FsError::Io(format!("Failed to decode stub env: {e}")))?;
                return Ok(Execution::new(exit_code, Box::new(std::io::Cursor::new(decoded))));
            }
        }

        let (reader, writer) = os_pipe::pipe()
            .map_err(|e| FsError::Io(format!("Failed to create output pipe: {e}")))?;
        let writer_stderr = writer
            .try_clone()
            .map_err(|e| FsError::Io(format!("Failed to clone pipe handle: {e}")))?;

        // Replace stdin field so `self` remains fully valid for `build_tokio_cmd`.
        let stdin_source = std::mem::replace(&mut self.stdin, InputSource::Null);
        let stdin_stdio = resolve_stdin(stdin_source)?;

        let mut cmd = build_tokio_cmd(&self, stdin_stdio);
        cmd.stdout(writer);
        cmd.stderr(writer_stderr);

        let child = cmd.spawn().map_err(|e| {
            FsError::Io(format!("Failed to spawn '{}': {e}", self.command.to_string_lossy()))
        })?;

        Ok(Execution { child: Some(child), exit_code: None, output: Box::new(reader) })
    }
}

/// Converts an [`InputSource`] into a [`Stdio`] handle suitable for passing to the child process.
///
/// For [`InputSource::Execution`], spawns a background thread that pumps the previous
/// execution's output into an OS pipe, returning the read end as the child's stdin.
fn resolve_stdin(source: InputSource) -> Result<Stdio, FsError> {
    match source {
        InputSource::Null => Ok(Stdio::null()),
        InputSource::Stdio => Ok(Stdio::inherit()),
        InputSource::Execution(exec) => {
            let (pipe_reader, pipe_writer) = os_pipe::pipe()
                .map_err(|e| FsError::Io(format!("Failed to create stdin pipe: {e}")))?;
            std::thread::spawn(move || {
                // `exec` (boxed) is moved here so `exec.child` stays alive for
                // the duration of the copy. Dropping it before reading completes
                // would send SIGTERM via kill_on_drop and may close the write
                // end of the OS pipe before all buffered data is consumed.
                let mut exec = exec;
                let mut writer = pipe_writer;
                // Copy errors are intentionally ignored: a broken pipe means the
                // downstream command has already exited, which is a normal shutdown
                // path. The exit code of that command is the authoritative signal.
                let _ = std::io::copy(&mut exec.output, &mut writer);
                // Dropping pipe_writer signals EOF to the next command's stdin.
                // Dropping exec reclaims the source child process.
            });
            Ok(pipe_reader.into())
        }
    }
}

/// Shared helper — configures a [`tokio::process::Command`] from a [`CommandBuilder`].
fn build_tokio_cmd(builder: &CommandBuilder, stdin: Stdio) -> TokioCommand {
    // Keep `key` referenced so `CommandBuilder.key` is not considered dead code
    // in non-stubbing builds (the stubbing logic reads it behind cfg gates).
    let _ = &builder.key;
    let mut cmd = TokioCommand::new(&builder.command);
    cmd.kill_on_drop(true);
    cmd.args(&builder.args);
    if builder.clear_env {
        cmd.env_clear();
    }
    cmd.envs(&builder.env);
    if let Some(wd) = &builder.workdir {
        cmd.current_dir(wd);
    }
    cmd.stdin(stdin);
    cmd
}

// ── test-utils feature ────────────────────────────────────────────────────────

/// Normalises a semantic key into an env-key suffix.
///
/// Converts to uppercase and replaces every non-alphanumeric ASCII character
/// with `_`. For example `"__nonexistent__"` → `"__NONEXISTENT__"` and
/// `"cargo"` → `"CARGO"`.
#[cfg(any(test, feature = "test-utils"))]
fn normalize_stub_key(name: &str) -> String {
    name.to_uppercase().chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}

/// Returns the per-key `'static` mutex for `key`, creating it on first access.
///
/// Each distinct semantic key gets its own mutex so concurrent stubs for
/// different intentions never block each other, while concurrent stubs for
/// the *same* key are correctly serialised.
#[cfg(any(test, feature = "test-utils"))]
fn key_mutex(key: &str) -> &'static std::sync::Mutex<()> {
    use std::collections::HashMap;
    static MAP: std::sync::OnceLock<
        std::sync::Mutex<HashMap<String, &'static std::sync::Mutex<()>>>,
    > = std::sync::OnceLock::new();
    let map_lock = MAP.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    let mut map = map_lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(m) = map.get(key) {
        return m;
    }
    // Leak a freshly allocated mutex so we can hand out `'static` guards.
    // Memory cost is trivial (one mutex per distinct binary name, test-only).
    let leaked: &'static std::sync::Mutex<()> = Box::leak(Box::new(std::sync::Mutex::new(())));
    map.insert(key.to_string(), leaked);
    leaked
}

/// RAII guard returned by [`stub_shell`].
///
/// Keeps `FORGE_STUB_SHELL_{CMD}` set for its entire lifetime and removes it on
/// [`drop`]. Holds the per-binary mutex so concurrent stubs for the *same*
/// binary are serialised, while stubs for *different* binaries run in parallel.
#[cfg(any(test, feature = "test-utils"))]
pub struct StubShellGuard {
    output_key: String,
    exit_code_key: String,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Drop for StubShellGuard {
    fn drop(&mut self) {
        // SAFETY: serialised by the per-binary `_lock` — no other thread holds
        // a StubShellGuard for this key at the same time.
        unsafe {
            std::env::remove_var(&self.output_key);
            std::env::remove_var(&self.exit_code_key);
        };
    }
}

/// Stubs [`CommandBuilder::run`] for a given semantic intention key.
///
/// While the returned [`StubShellGuard`] is live, any `CommandBuilder` whose
/// semantic key matches `semantic_key` will return `output` and the provided
/// `exit_code` instead of spawning a real process.
///
/// Concurrent stubs for *different* semantic keys are fully independent and
/// never block each other. Concurrent stubs for the *same* semantic key are
/// serialised — the second call blocks until the first guard is dropped.
///
/// # Side Effects
/// Sets `FORGE_STUB_SHELL_<NORMALIZED_KEY>` and
/// `FORGE_STUB_SHELL_EXIT_CODE_<NORMALIZED_KEY>` for the duration of the guard.
#[cfg(any(test, feature = "test-utils"))]
#[must_use]
pub fn stub_shell(semantic_key: &str, exit_code: i32, output: &str) -> StubShellGuard {
    use base64::Engine as _;
    let normalized = normalize_stub_key(semantic_key);
    let output_key = format!("FORGE_STUB_SHELL_{normalized}");
    let exit_code_key = format!("FORGE_STUB_SHELL_EXIT_CODE_{normalized}");
    let lock = key_mutex(&normalized).lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let encoded = base64::engine::general_purpose::STANDARD.encode(output);
    // SAFETY: serialised by the per-key lock.
    unsafe {
        std::env::set_var(&output_key, &encoded);
        std::env::set_var(&exit_code_key, exit_code.to_string());
    };
    StubShellGuard { output_key, exit_code_key, _lock: lock }
}

#[cfg(test)]
#[path = "shell_test.rs"]
mod tests;
