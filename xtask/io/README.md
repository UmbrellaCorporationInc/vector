# `io`

Backend-agnostic I/O abstraction for the Forge runtime. Provides two subsystems:

- **Filesystem** — object-safe traits ([`Directory`], [`File`]) with in-memory and disk backends.
- **Shell** — deterministic synchronous process execution via `CommandBuilder`, `Execution`, `InputSource`, and the `command!` macro.

File resolution and structural boundaries (`FileLookup`, `Library`, `RootLookup`) live in the
`runtime/library` crate, which depends on this one.

---

## Architecture

```
io
├── Directory / File          ← backend-agnostic trait layer
│   ├── MemoryDir             — HashMap + Arc<Mutex>, zero disk I/O (tests)
│   ├── DiskDir               — delegates to std::fs (production)
│   └── ZipDir                — read-only virtual mount of .zip archives
│
└── Shell
    ├── CommandBuilder        ← fluent builder: .arg() .workdir() .env() .stdin() .run()
    ├── Execution             ← process handle: output: Reader + async wait()
    ├── InputSource           ← stdin routing: Null | Stdio | Execution(Box<Execution>)
    └── command!              ← ergonomic macro shorthand
```

---

## Filesystem API

### Selecting a backend

```rust
use io::MemoryDir;
use io::DiskDir;

// In-memory (tests, sandboxed environments)
let dir = MemoryDir::new();

// Disk (production) — all paths resolved relative to root
let dir = DiskDir::new("/var/forge/workspace");

// ZIP (virtual mount) — read-only access to .zip/.babel/.doc artifacts
let dir = ZipDir::new(std::fs::File::open("artifact.babel")?)?;
```

### File operations

```rust
let file = dir.get_file("docs/readme.txt")?;

// Write (full replace)
file.write_text("hello forge")?;
file.write_bytes(&[0xDE, 0xAD, 0xBE, 0xEF])?;

// Streaming write — returns a Writer (Box<dyn std::io::Write + Send>)
// Useful for emitters and serializers that write incrementally.
// The writer truncates the file on open; content is committed on flush/drop.
let mut writer = file.write_writer()?; // io::Writer
writer.write_all(b"hello")?;
writer.flush()?;

// Read
let text  = file.read_text()?;
let bytes = file.read_bytes()?;

// Streaming read
let reader = file.read_reader()?; // Box<dyn Read + Send + Sync>

// Metadata
let ts = file.last_modified()?;         // UnixTimestamp (u64, seconds since epoch)
if let Some(path) = file.path() { /* physical path, DiskDir only */ }

// Type alias for the streaming writer handle
// pub type io::Writer = Box<dyn std::io::Write + Send>;

// Delete / existence
file.delete()?;
if file.exists() { ... }
```

### Directory operations

```rust
// Create directory tree — idempotent
dir.create_dir("sub/nested")?;

// Get a file handle relative to this directory
let f = dir.get_file("src/main.rs")?;
f.write_text("...")?;

// Recursively list all files — returns relative paths using '/' as separator
for entry in dir.list_files()? {
    let path = entry?; // e.g. "src/main.rs", "docs/readme.txt"
    println!("{path}");
}
```

---

## Shell API

### Running a command

`run()` is synchronous and returns an `Execution` handle immediately after spawning
the OS process. `stdout` and `stderr` are merged into a single `output: Reader` at
the OS level (ADR 0020).

```rust
use io::CommandBuilder;
use std::io::Read as _;

let mut exec = CommandBuilder::new("cargo")
    .arg("build")
    .arg("--release")
    .workdir("/var/lib/forge/build_zone")
    .env("RUST_LOG", "debug")
    .run()?;                         // sync — spawns the process

// Read merged stdout + stderr
let mut output = String::new();
exec.output.read_to_string(&mut output)?;

// Drain remaining output and wait for the exit code
let code = exec.wait().await?;       // async — reaps the process
```

### `Execution` handle

```rust
pub struct Execution {
    pub output: Reader,  // merged stdout + stderr — Box<dyn Read + Send + Sync>
    // child and exit_code are private
}

impl Execution {
    // Creates a static handle backed by a pre-computed byte stream (no OS process).
    pub fn new(exit_code: i32, output: Reader) -> Self;

    // Drains output (background thread) then waits for the child to exit.
    // Consumes self — the handle cannot be reused after this call.
    pub async fn wait(self) -> Result<i32, FsError>;
}
```

`Execution` employs `kill_on_drop` semantics: if the handle is dropped before
`wait()` is called, the child process is automatically terminated.

### `InputSource` — stdin routing

```rust
pub enum InputSource {
    Null,                      // default — discard stdin (/dev/null equivalent)
    Stdio,                     // inherit parent stdin (interactive commands)
    Execution(Box<Execution>), // pipe a previous Execution's output as stdin
}
```

### Piping commands

```rust
use io::{CommandBuilder, InputSource};
use std::io::Read as _;

// echo forge_marker | grep forge_marker
let source = CommandBuilder::shell_command("echo forge_marker").run()?;

let mut piped = CommandBuilder::new("grep")
    .arg("forge_marker")
    .stdin(InputSource::Execution(Box::new(source)))
    .run()?;

let mut out = String::new();
piped.output.read_to_string(&mut out)?;
let code = piped.wait().await?;
```

### `command!` macro — ergonomic shorthand

```rust
use io::command;

// Unix: echo macro_ok
let exec = command!("echo", "macro_ok").run()?;
let code = exec.wait().await?;
```

### Mocking — virtual execution

`Execution::new` creates a static handle with a pre-computed byte stream.
Pass it to `CommandBuilder::mocked` to short-circuit `run()` without spawning
a real OS process.

```rust
use io::{CommandBuilder, Execution};
use std::io::{Cursor, Read as _};

let mock = Execution::new(0, Box::new(Cursor::new(b"virtual output")));

let mut exec = CommandBuilder::new("any_binary")
    .mocked(mock)
    .run()?;        // returns immediately — no process spawned

let mut buf = String::new();
exec.output.read_to_string(&mut buf)?;
assert_eq!(buf, "virtual output");

let code = exec.wait().await?;
assert_eq!(code, 0);
```

### Platform-aware shell commands

```rust
use io::CommandBuilder;

// Uses `cmd /c` on Windows, `sh -c` on Unix
let exec = CommandBuilder::shell_command("echo $FORGE_HOME").run()?;
let code = exec.wait().await?;
```

---

## Feature Flags

| Flag | When to enable |
|---|---|
| `test-utils` | Exposes [`stub_shell`] and [`StubShellGuard`] for mocking shell execution in tests. Add to `[dev-dependencies]` only. |

### `test-utils` — mocking shell commands in tests

Crates that use `CommandBuilder` can enable `test-utils` in their `[dev-dependencies]`
to mock shell execution without spawning real OS processes:

```toml
[dev-dependencies]
io = { workspace = true, features = ["test-utils"] }
```

```rust
use io::stub_shell;

#[test]
fn my_test() {
    let _guard = stub_shell("my-command", 0, "expected output");
    // Any CommandBuilder::run() call for "my-command" returns a mocked Execution
    // backed by "expected output" with exit code 0 instead of spawning a real process.
    // The guard removes FORGE_STUB_SHELL env vars automatically on drop.
}
```

`stub_shell` holds a process-wide mutex for its lifetime, serializing all shell
mock calls across parallel test threads.

---

## Error Handling

All fallible operations return `Result<_, FsError>`.

| Variant | When emitted |
|---|---|
| `FsError::NotFound(path)` | File or directory does not exist |
| `FsError::PermissionDenied(path)` | Malicious path traversal or OS permission error |
| `FsError::Io(message)` | Unclassified I/O failure (spawn error, pipe break, drain panic) |

---

## Backend Details

### `MemoryDir`

All data is stored in a `HashMap<String, FileData>` (content + Unix timestamp) protected by a `Mutex`.
Cloning derived handles is `O(1)`. `create_dir` is a no-op as the store is key-based.
`list_files` filters store keys by the handle's prefix and returns relative subpaths.
`last_modified` returns the timestamp stamped on the last `write_text` or `write_bytes` call.

### `DiskDir`

Delegates to `std::fs`. All paths are resolved relative to the `root` supplied at
construction. `File::path()` returns `Some(PathBuf)` for this backend.

### `ZipDir`

Provides a virtual mount of a ZIP archive. All file accesses are read-only (`FsError::PermissionDenied` on write).
To ensure thread-safety and high-concurrency without global locks, `ZipDir` uses a **Source Factory** pattern:
each `read_reader()` call clones the underlying source and spawns an independent archive handle (ADR 0098).
`last_modified` returns the timestamp from the ZIP Central Directory.

---

## Dependencies

| Crate | Role |
|---|---|
| `tokio` | Async runtime for process spawning and drain task scheduling |
| `os_pipe` | OS-level pipe creation for merging stdout/stderr and stdin routing |
| `thiserror` | Error derive macros for `FsError` |
