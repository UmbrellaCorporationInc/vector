---
id: rfc-00009-runtime-io-file-access-and-shell-command-execution
type: rfc
code: "00009"
slug: runtime-io-file-access-and-shell-command-execution
title: Runtime IO File Access and Shell Command Execution
description: Defines the runtime-io crate boundary for reader and writer primitives over files, memory, paths, text adaptation, and shell command execution.
status: implemented
created: 2026-05-03
updated: 2026-05-04
authors: []
tags:
  - runtime
  - io
  - file
  - path
  - shell
related:
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00006-runtime-core-control-observability-and-encoding-primitives
supersedes: []
superseded_by: null
aliases:
  - "RFC 00009: Runtime IO File Access and Shell Command Execution"
---

# RFC 00009: Runtime IO File Access and Shell Command Execution

## 1. Problem

The workspace has `runtime-core` for transport-agnostic async primitives and `runtime-channel` for the standard channel backend, but it still lacks one runtime-owned crate for practical IO boundaries.

Two concrete gaps remain unresolved:

- there is no standard reader and writer model for files and in-memory buffers
- there is no standard shell command execution API that reuses those same reader and writer boundaries

Without one accepted `runtime-io` crate, higher-level crates will improvise different shapes for file access, UTF-8 conversion, path handling, and process IO. That drift would create inconsistent buffering behavior, duplicate text-decoding logic, and incompatible process interaction patterns.

The project needs one crate that turns concrete IO resources into standard runtime boundaries while reusing `runtime-core` primitives instead of introducing unrelated streaming models.

This RFC follows [[rfc-00002-runtime-core-v1-boundary-and-async-first-contracts]], [[rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations]], and [[rfc-00006-runtime-core-control-observability-and-encoding-primitives]].

## 2. Proposal

Create a new crate at `runtime/io` with the published crate name `runtime-io`.

`runtime-io` owns the following API families:

- file-backed reader and writer implementations
- memory-backed reader and writer implementations
- text adapters over readers and writers
- path utilities
- shell command planning and execution through specs, executors, and handles

This RFC accepts one core design decision:

- byte readers implement `Receiver<Bytes>`
- byte writers implement `Sender<Bytes>`

`runtime-io` therefore depends on `runtime-core` and reuses its established async message boundaries instead of creating a separate streaming abstraction family.

### Ownership boundary

`runtime-io` owns:

- concrete IO implementations that read bytes from files
- concrete IO implementations that write bytes to files
- concrete IO implementations that read bytes from memory
- concrete IO implementations that write bytes to memory
- text adapters that convert byte readers and writers into UTF-8 text readers and writers
- path manipulation helpers over Rust paths
- shell command preparation, execution, and process IO handles

`runtime-io` does not own:

- repository-specific file workflows
- structured text parsing
- line-oriented protocol policy
- shell parsing from one composite command string
- retry policy
- scheduling policy
- process orchestration policy beyond one running process handle
- non-UTF-8 text codecs

### Reader and Writer Boundaries

Shape intent:

- readers emit values
- writers consume values
- byte streaming reuses `runtime-core` channel semantics
- file IO, memory IO, and process IO share one streaming model

This RFC is intentionally byte-oriented at the base boundary.

- byte readers emit `Bytes`
- byte writers consume `Bytes`
- text is layered on top through dedicated adapters

This avoids mixing UTF-8 framing concerns into the lowest-level reader and writer contracts.

This RFC does not define `Reader` or `Writer` as new aliases or new traits. The accepted boundary is expressed directly through the existing `runtime-core` contracts:

- `FileReader`, `MemReader`, and process output readers implement `Receiver<Bytes>`
- `FileWriter`, `MemWriter`, and process input writers implement `Sender<Bytes>`

### File-backed implementations

`FileReader` reads bytes from a file and exposes them through the accepted `Receiver<Bytes>` boundary.

`FileWriter` writes bytes to a file through the accepted `Sender<Bytes>` boundary.

Accepted responsibilities:

- `FileReader` opens a file path and produces byte chunks
- `FileWriter` opens a file path and consumes byte chunks
- both implementations are buffered
- buffer size is configured at construction time
- both implementations must implement `Drop`

Illustrative direction:

```rust
pub struct FileReader { /* private fields */ }

impl FileReader {
    pub async fn open(path: impl AsRef<std::path::Path>, buffer_size: usize) -> RuntimeResult<Self>;
}

impl Receiver<Bytes> for FileReader { /* ... */ }

impl Drop for FileReader { /* ... */ }

pub struct FileWriter { /* private fields */ }

impl FileWriter {
    pub async fn create(path: impl AsRef<std::path::Path>, buffer_size: usize) -> RuntimeResult<Self>;
}

impl Sender<Bytes> for FileWriter { /* ... */ }

impl Drop for FileWriter { /* ... */ }
```

Drop intent:

- release the owned file handle
- flush or finalize any pending buffered state required by the chosen implementation
- avoid leaking background tasks or open descriptors

### Memory-backed implementations

`MemReader` and `MemWriter` provide the same runtime boundary over in-memory storage.

Accepted responsibilities:

- `MemReader` emits bytes from a memory-backed source
- `MemWriter` collects bytes into a memory-backed sink
- both implementations follow the same reader and writer contracts as file-backed IO

Illustrative direction:

```rust
pub struct MemReader { /* private fields */ }

impl MemReader {
    pub fn new(bytes: impl Into<Bytes>, buffer_size: usize) -> Self;
}

impl Receiver<Bytes> for MemReader { /* ... */ }

pub struct MemWriter { /* private fields */ }

impl MemWriter {
    pub fn new(buffer_size: usize) -> Self;
    pub fn into_bytes(self) -> Bytes;
}

impl Sender<Bytes> for MemWriter { /* ... */ }
```

`MemReader` and `MemWriter` do not need `Drop` as part of the accepted contract unless their final implementation owns additional external resources. The required `Drop` contract in this RFC applies to `FileReader`, `FileWriter`, and `CommandHandle`.

### Text adapters

`TextReader` and `TextWriter` adapt byte-oriented `Receiver<Bytes>` and `Sender<Bytes>` implementations into UTF-8 text boundaries.

Accepted responsibilities:

- `TextReader` adapts a byte `Receiver<Bytes>` into text output
- `TextWriter` adapts a byte `Sender<Bytes>` into text input
- both adapters use `Encoding` from `runtime-core`
- both adapters are buffered
- buffer size is configured at construction time

Illustrative direction:

```rust
pub struct TextReader<R> { /* private fields */ }
where
    R: Receiver<Bytes>;

impl<R> TextReader<R>
where
    R: Receiver<Bytes>,
{
    pub fn new(reader: R, buffer_size: usize) -> Self;
}

impl<R> Receiver<String> for TextReader<R>
where
    R: Receiver<Bytes>,
{
    /* ... */
}

pub struct TextWriter<W> { /* private fields */ }
where
    W: Sender<Bytes>;

impl<W> TextWriter<W>
where
    W: Sender<Bytes>,
{
    pub fn new(writer: W, buffer_size: usize) -> Self;
}

impl<W> Sender<String> for TextWriter<W>
where
    W: Sender<Bytes>,
{
    /* ... */
}
```

Buffering rules:

- `TextReader` and `TextWriter` must buffer around UTF-8 boundaries
- they must not emit invalid partial UTF-8 sequences as text
- they must preserve incomplete trailing bytes until a valid UTF-8 boundary can be formed or decoding fails
- buffer size is caller-controlled

Critical boundary note:

The UTF-8 alignment requirement belongs to `TextReader` and `TextWriter`, not to raw byte `FileReader` and `FileWriter`. Raw byte readers and writers are allowed to operate on arbitrary chunk boundaries because bytes have no UTF-8 validity requirement. The buffering discipline needed to avoid incomplete UTF-8 consumption applies only at the text adapter layer.

This matters because otherwise the byte layer would accidentally inherit text policy and become harder to reuse for binary content and process IO.

### Path API

`runtime-io` exposes a path manipulation API that wraps or composes Rust path handling and becomes the standard path input for file-backed IO.

Accepted responsibilities:

- construct paths
- join paths
- inspect path segments and file names
- normalize caller-visible path usage rules as needed by the crate
- provide a stable path object that is accepted by `FileReader` and `FileWriter`

Accepted direction:

```rust
pub struct IoPath { /* private fields */ }

impl IoPath {
    pub fn new(path: impl AsRef<std::path::Path>) -> Self;
    pub fn join(&self, segment: impl AsRef<std::path::Path>) -> Self;
    pub fn as_path(&self) -> &std::path::Path;
}
```

Shape intent:

- reuse Rust path semantics instead of inventing a string-based path model
- keep a runtime-owned path entry point for the rest of the crate
- let `FileReader` and `FileWriter` accept `IoPath` directly or anything that can become one

### File convenience API

In addition to streaming implementations, `runtime-io` may expose convenience helpers for full text and byte reads and writes, but those helpers are adapters over the accepted reader and writer model rather than an independent API family.

Accepted direction:

- read full file content as `Bytes`
- write full file content from `Bytes`
- read full file content as `String` through `TextReader`
- write full file content from `String` through `TextWriter`

This keeps streaming as the foundational contract and treats full-buffer helpers as convenience only.

### Shell command API

Shell command support is spec-first.

`CommandBuilder` prepares one command specification.

Accepted builder responsibilities:

- configure the executable command
- configure ordered arguments
- configure an optional working directory
- configure environment variables
- build one data-only command specification

Accepted builder direction:

```rust
pub struct CommandBuilder { /* private fields */ }

pub struct CommandSpec { /* private fields */ }

impl CommandBuilder {
    pub fn new(command: impl Into<String>) -> Self;
    pub fn arg(self, argument: impl Into<String>) -> Self;
    pub fn args(self, arguments: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn current_dir(self, path: impl AsRef<std::path::Path>) -> Self;
    pub fn env(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn build(self) -> RuntimeResult<CommandSpec>;
}
```

`CommandBuilder` does not accept stdin configuration. Stdin is exposed only after execution begins through the returned handle.

Execution belongs to a separate executor boundary.

```rust
pub trait CommandExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = RuntimeResult<CommandHandle>> + Send;
}
```

`runtime-io` provides one operating-system-backed implementation through `ProcessCommandExecutor`.

`CommandHandle` owns the running process boundary after execution starts.

Accepted handle responsibilities:

- expose stdout as `CommandOutput`
- expose stderr as `CommandOutput`
- expose stdin as `CommandInput`
- wait for process completion
- implement `Drop`

Accepted direction:

```rust
pub struct CommandOutput { /* private fields */ }

impl Receiver<Bytes> for CommandOutput { /* ... */ }

pub struct CommandInput { /* private fields */ }

impl Sender<Bytes> for CommandInput { /* ... */ }

pub struct CommandHandle { /* private fields */ }

impl CommandHandle {
    pub fn stdout(&mut self) -> &mut CommandOutput;
    pub fn stderr(&mut self) -> &mut CommandOutput;
    pub fn stdin(&mut self) -> &mut CommandInput;
    pub async fn wait(self) -> RuntimeResult<CommandExit>;
}

impl Drop for CommandHandle { /* ... */ }
```

Shape intent:

- process planning is separate from process creation
- process creation is separate from process interaction
- `CommandOutput` implements `Receiver<Bytes>`
- `CommandInput` implements `Sender<Bytes>`
- process stdio reuses the same streaming contracts as files and memory
- callers can build text adapters over process streams if they need UTF-8 text interaction
- deterministic tests may build mock `CommandHandle` values directly without launching a real process

Drop intent:

- release process-owned handles
- request termination for a still-running process as best-effort cleanup
- avoid leaking process resources when the handle goes out of scope

### Error boundary

`runtime-io` must return typed library errors.

The accepted error surface includes at least:

- file open failure
- file read failure
- file write failure
- invalid UTF-8 during text decoding
- path-related failure when required by the chosen API
- process spawn failure
- process stream failure
- process wait failure

This RFC does not force immediate unification into `RuntimeError`, but it does require compatibility with the project's typed runtime error style and with `Encoding`-based UTF-8 handling.

### Dependency boundary

`runtime-io` may depend on:

- Rust `std`
- `runtime-core`
- the standard async runtime backend already used by runtime crates
- `thiserror`

No shell parser, line protocol parser, repository-specific helper library, or alternative text codec dependency belongs in the accepted v1 boundary.

## 3. Alternatives Considered

- **Create independent reader and writer traits unrelated to `runtime-core`:** Discarded because the project already accepted transport-agnostic async channel primitives and duplicating another streaming abstraction family would fragment the runtime boundary.
- **Make file and process streaming separate models:** Discarded because one shared `Receiver<Bytes>` and `Sender<Bytes>` shape is the main reuse advantage of introducing `runtime-io`.
- **Push UTF-8 alignment down into raw byte readers and writers:** Discarded because byte-oriented IO must stay valid for binary content and arbitrary process output, and UTF-8 framing belongs only at the text adapter layer.
- **Put path helpers outside `runtime-io`:** Discarded because file-backed IO needs one stable path object boundary instead of ad hoc direct `PathBuf` handling at every call site.
- **Keep stdin configuration on `CommandBuilder`:** Discarded because the chosen direction is to keep process input as part of the returned runtime handle rather than part of launch configuration.
- **Expose shell commands as one shell-form string:** Discarded because command parsing and escaping semantics differ by platform and would create ambiguous runtime behavior.
- **Require `Drop` on every in-memory implementation:** Discarded because memory-backed readers and writers do not inherently own external resources and should not be forced into artificial lifecycle semantics unless the implementation actually needs them.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Reusing `Receiver<Bytes>` and `Sender<Bytes>` keeps `runtime-io` aligned with `runtime-core` instead of creating another async streaming model. | IO APIs now inherit the constraints and ergonomics of the core channel contracts, including any generic complexity they carry. |
| One reader and writer shape works for files, memory, and processes. | The same abstraction may feel lower-level than a domain-specific file or process API at some call sites. |
| Separating raw byte IO from `TextReader` and `TextWriter` keeps binary and text concerns disciplined. | Text consumers need an explicit adapter instead of reading `String` directly from every source. |
| Caller-controlled buffer sizes let the crate support small interactive streams and larger throughput-oriented flows. | Buffer sizing becomes an API decision at every construction point and may be misconfigured by callers. |
| `Drop` on file and process handles makes resource ownership explicit. | `Drop` semantics must be carefully documented so callers understand what is guaranteed and what is best-effort cleanup only. |
| A runtime-owned path type creates one stable crate entry point for file-backed IO. | If the wrapper adds little value, it can become a thin indirection over `std::path::PathBuf` that still needs long-term maintenance. |
| `CommandHandle` reusing readers and writers makes process IO composition straightforward. | Process APIs raise extra lifecycle questions around when streams close, what `wait` consumes, and how drop interacts with a running child process. |

## 5. Acceptance Criteria

- [ ] A new crate exists at `runtime/io` with the published crate name `runtime-io`.
- [ ] `runtime-io` depends on `runtime-core` and is documented as the runtime-owned crate for path, text, file, memory, and shell IO boundaries.
- [ ] `runtime-io` defines its byte-oriented streaming contract directly over `Receiver<Bytes>` and `Sender<Bytes>`.
- [ ] `runtime-io` exposes `FileReader` as a file-backed byte reader.
- [ ] `runtime-io` exposes `FileWriter` as a file-backed byte writer.
- [ ] `FileReader` accepts a caller-supplied buffer size.
- [ ] `FileWriter` accepts a caller-supplied buffer size.
- [ ] `FileReader` implements `Drop`.
- [ ] `FileWriter` implements `Drop`.
- [ ] `runtime-io` exposes `MemReader` as a memory-backed byte reader.
- [ ] `runtime-io` exposes `MemWriter` as a memory-backed byte writer.
- [ ] `runtime-io` exposes `TextReader` as an adapter over a byte reader.
- [ ] `runtime-io` exposes `TextWriter` as an adapter over a byte writer.
- [ ] `TextReader` uses `Encoding` from `runtime-core` for UTF-8 decoding.
- [ ] `TextWriter` uses `Encoding` from `runtime-core` for UTF-8 encoding.
- [ ] `TextReader` accepts a caller-supplied buffer size.
- [ ] `TextWriter` accepts a caller-supplied buffer size.
- [ ] `TextReader` and `TextWriter` buffer across UTF-8 boundaries and do not emit invalid partial UTF-8 text fragments.
- [ ] Raw byte readers and writers remain valid for arbitrary binary content and are not forced to align chunk boundaries to UTF-8.
- [ ] `runtime-io` exposes a path API over Rust path handling.
- [ ] The path API is accepted by `FileReader` and `FileWriter`.
- [ ] `runtime-io` exposes `CommandBuilder`.
- [ ] `CommandBuilder` configures the executable command explicitly.
- [ ] `CommandBuilder` configures ordered arguments explicitly.
- [ ] `CommandBuilder` configures an optional working directory.
- [ ] `CommandBuilder` configures environment variables.
- [ ] `CommandBuilder` does not accept stdin configuration before process creation.
- [ ] `runtime-io` exposes `CommandSpec`.
- [ ] `CommandBuilder::build` returns `CommandSpec`.
- [ ] `runtime-io` exposes `CommandExecutor`.
- [ ] `CommandExecutor::spawn` returns `CommandHandle`.
- [ ] `runtime-io` exposes `CommandOutput` and `CommandInput` as concrete process IO types.
- [ ] `CommandOutput` implements `Receiver<Bytes>`.
- [ ] `CommandInput` implements `Sender<Bytes>`.
- [ ] `CommandHandle` exposes stdout as `CommandOutput`.
- [ ] `CommandHandle` exposes stderr as `CommandOutput`.
- [ ] `CommandHandle` exposes stdin as `CommandInput`.
- [ ] `CommandHandle` exposes process completion through `wait`.
- [ ] `CommandHandle` implements `Drop`.
- [ ] The public command boundary allows deterministic mock `CommandHandle` construction without changing `CommandHandle`, `CommandOutput`, or `CommandInput`.
- [ ] `runtime-io` returns typed library errors for file, text, path, and process failures.
- [ ] `runtime-io` introduces no shell parser, structured text parser, retry policy, scheduling policy, or repository-specific workflow logic.

## 6. Open Questions

- Should `Bytes` in v1 be `Vec<u8>`, a newtype, or a borrowed-capable buffer type kept internal to the crate boundary?
- Should `IoPath` be a strict wrapper type used everywhere in file APIs, or should file-backed IO continue to accept plain `AsRef<Path>` while `IoPath` remains optional convenience?
- Should `CommandHandle::wait` treat non-zero exit codes as typed errors, or should it always return a `CommandExit` value and leave exit-code policy to callers?
- What exact `Drop` guarantee should `CommandHandle` provide for a still-running process: close local handles only, request termination, or synchronously wait for shutdown?
- How much mock command support should stay in the public API beyond mock `CommandHandle` construction itself?
