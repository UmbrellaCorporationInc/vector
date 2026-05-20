# `runtime-io`

`runtime-io` provides asynchronous input/output primitives and stream adapters for the `vector` runtime. It builds strictly upon the `Sender<T>` and `Receiver<T>` channel contracts defined in `runtime-core`.

## Features

- **File IO**: Streaming binary file readers (`FileReader`) and writers (`FileWriter`) with explicit memory buffer allocation.
- **Memory Buffers**: In-memory message queues (`MemReader`, `MemWriter`) for fast testing and buffer swapping.
- **Text Processing**: UTF-8 aware streaming adapters (`TextReader`, `TextWriter`) that enforce multibyte character boundaries across chunked binary streams.
- **Path API**: A strict path boundary (`IoPath`, `PathResolver`) enforcing sandbox limitations and path normalization to prevent traversal attacks.
- **Shell Commands**: Spec-first command planning (`CommandBuilder`, `CommandSpec`), explicit execution (`CommandExecutor`, `ProcessCommandExecutor`), and typed process handles (`CommandHandle`, `CommandInput`, `CommandOutput`).
- **Helpers**: Full-file read/write adapters for `Bytes` and `String` built transparently over the streaming API.

## Usage

```rust
use runtime_io::{FileReader, TextReader};
use runtime_core::Receiver;

// Read a file chunk by chunk with an 8KB buffer
let mut reader = FileReader::open("data.txt", 8192).await?;

// Transparently wrap the byte stream in a UTF-8 character-aware text adapter
let mut text_reader = TextReader::new(reader, 8192);

while let Some(chunk) = text_reader.recv().await {
    println!("Received text chunk: {}", chunk);
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
```

`CommandBuilder` performs no process side effects. Execution starts only when one executor spawns a `CommandSpec`.

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
