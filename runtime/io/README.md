# `runtime-io`

`runtime-io` provides asynchronous input/output primitives and stream adapters for the `vector` runtime. It builds strictly upon the `Sender<T>` and `Receiver<T>` channel contracts defined in `runtime-core`.

## Features

- **File IO**: Streaming binary file readers (`FileReader`) and writers (`FileWriter`) with explicit memory buffer allocation.
- **Memory Buffers**: In-memory message queues (`MemReader`, `MemWriter`) for fast testing and buffer swapping.
- **Text Processing**: UTF-8 aware streaming adapters (`TextReader`, `TextWriter`) that enforce multibyte character boundaries across chunked binary streams.
- **Path API**: A strict path boundary (`IoPath`, `PathResolver`) enforcing sandbox limitations and path normalization to prevent traversal attacks.
- **Directory Traversal**: Deterministic directory listing and recursive traversal over `IoPath` roots with generic entry metadata.
- **Shell Commands**: Spec-first command planning (`CommandBuilder`, `CommandSpec`), explicit execution (`CommandExecutor`, `ProcessCommandExecutor`), and typed process handles (`CommandHandle`, `CommandInput`, `CommandOutput`).
- **Helpers**: Full-file read/write adapters for `Bytes` and `String` built transparently over the streaming API.

## Usage

```rust
use runtime_core::Receiver;
use runtime_io::{FileReader, IoPath, TextReader};

// Read a file chunk by chunk with an 8KB buffer
let reader = FileReader::open(&IoPath::new("data.txt"), 8192).await?;

// Transparently wrap the byte stream in a UTF-8 character-aware text adapter
let mut text_reader = TextReader::new(reader, 8192);

while let Some(chunk) = text_reader.recv().await? {
    println!("Received text chunk: {}", chunk);
}
```

Higher-level crates should combine directory traversal with the existing file
readers by passing each file entry path directly into `read_file_bytes`,
`read_file_text`, or `FileReader::open`. Domain filtering stays outside
`runtime-io`; for example, a Markdown discovery crate can filter extensions
before reading candidate files:

```rust
use runtime_io::{read_file_text, traverse_directory, IoPath};

let root = IoPath::new("doc");
let entries = traverse_directory(&root).await?;

for entry in entries.iter().filter(|entry| entry.is_file()) {
    if matches!(
        entry.path().as_path().extension().and_then(|extension| extension.to_str()),
        Some("md" | "markdown")
    ) {
        let content = read_file_text(entry.path()).await?;
        println!("{} bytes", content.len());
    }
}
```

## Command Planning and Execution

Shell commands are split into planning and execution.

```rust
use runtime_io::{CommandBuilder, CommandExecutor, ProcessCommandExecutor};

let spec = CommandBuilder::new("git")
    .arg("status")
    .build()?;

let executor = ProcessCommandExecutor;
let mut handle = executor.spawn(spec).await?;

// Stream stdout and stderr concurrently until both are exhausted.
handle.stream_output(
    |bytes| print!("{}", String::from_utf8_lossy(bytes)),
    |bytes| eprint!("{}", String::from_utf8_lossy(bytes)),
).await;

let exit = handle.wait().await?;
```

`CommandBuilder` performs no process side effects. Execution starts only when one executor spawns a `CommandSpec`.

`CommandHandle::stream_output` drains stdout and stderr concurrently via `tokio::select!`, forwarding each chunk to the provided callbacks until both streams are exhausted. Call `wait` afterwards to obtain the exit status.

For deterministic tests that only need the running-command boundary, `runtime-io` also exposes `MockCommandHandleBuilder`, which builds a mock `CommandHandle` without launching a real process.

## Domain Aliases

`runtime-io` exposes `Writer<T>` and `Reader<T>` as named sub-traits over the
`Sender<T>` and `Receiver<T>` contracts from `runtime-core`. Every concrete
`Sender<T>` or `Receiver<T>` satisfies the corresponding alias automatically —
no changes to implementing types are required.

```rust
use runtime_io::{Writer, Reader};

fn write_value<W: Writer<u32>>(w: &mut W) { ... }
fn read_value<R: Reader<u32>>(r: &mut R) { ... }
```

## Contracts and Constraints

1. **No External Parsers**: This crate does not parse structured data (like JSON, YAML) nor shell command syntax. It only manages primitive streams.
2. **Channel Parity**: All stream operations strictly implement either `Sender<Bytes>` or `Receiver<Bytes>` (`String` for Text wrappers), ensuring total compatibility with the runtime channel infrastructure.
3. **No Unbounded Buffering**: Streaming boundaries explicitly define byte caps per transaction to avoid OOM vulnerabilities during file or process streams.

## License

MIT
