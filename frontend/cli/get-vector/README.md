# `get-vector`

## 1. Objective

`get-vector` is the operator CLI for managing the local Vector tool installation. It provides a single command surface to install or update the base `mcp-vector` and `vector-database` binaries from the vector repository without requiring manual `cargo install` invocations. Optional RAG support is installed separately through `get-vector install rag`.

## 2. Boundaries

### In scope

- Installing and updating the local `mcp-vector` and `vector-database` binaries
- Installing optional local RAG support through the `vector-rag` companion CLI
- Using `runtime-io` execution primitives (`CommandBuilder`, `CommandExecutor`, `ProcessCommandExecutor`) as the shell boundary

### Out of scope

- Version comparison or install-state detection (deferred to a future release)
- MCP protocol interaction
- Any mutation of MCP server behavior or configuration

### Dependencies

| Dependency     | Role                                                        |
|----------------|-------------------------------------------------------------|
| `runtime-io`   | `CommandBuilder`, `CommandExecutor`, `ProcessCommandExecutor` for shell execution |
| `runtime-core` | Core async primitives                                       |
| `tokio`        | Async runtime                                               |
| `terminal_size`| Dynamic terminal size querying for output formatting        |
| `thiserror`    | Error enum derivation                                       |

## 3. Commands

### `update-mcp-vector`

Runs `cargo install --git <repo> --force mcp-vector` and `cargo install --git <repo> --force vector-database` to install or update the base local Vector tools from the repository HEAD.

This command does not install RAG support. Run `get-vector install rag` when the optional `vector-rag` companion CLI is required for `vector-database rag ...` commands.

V1 always performs a full install. No version comparison is done — this avoids a chicken-and-egg problem where an outdated CLI binary would incorrectly skip reinstallation after a workspace version bump.

Cargo's stdout and stderr are streamed live to the terminal as the install runs, so the command does not appear blocked during the (often lengthy) compilation step.

**Exit behavior:**

| Condition                       | Exit code |
|---------------------------------|-----------|
| Install succeeded               | `0`       |
| `cargo install` exited non-zero | `1`       |
| Failed to spawn `cargo`         | `1`       |

### `install rag`

Runs `cargo install --git <repo> --force vector-rag` to install or update the optional local RAG runtime support from the repository HEAD.

`vector-rag` owns the heavy RAG runtime dependencies, including LanceDB, DataFusion, FastEmbed, ONNX Runtime, and tokenizer dependencies. Keeping this install behind an explicit command prevents the base `update-mcp-vector` flow from compiling those dependencies unless RAG support is requested.

**Exit behavior:**

| Condition                       | Exit code |
|---------------------------------|-----------|
| Install succeeded               | `0`       |
| `cargo install` exited non-zero | `1`       |
| Failed to spawn `cargo`         | `1`       |

**UX limitations (V1):**

- Output is raw Cargo text; no custom progress bar or structured formatting is applied.
- stdout and stderr from Cargo are interleaved via concurrent polling — their relative order may differ slightly from a sequential terminal session when both streams have data simultaneously.
- Version-aware skip logic (avoid reinstalling when already up to date) is deferred; the command always performs a full compile-and-install cycle.

### `--version`, `-V`

Displays the version of the `get-vector` CLI (derived from the cargo package version) to help verify alignment with the workspace and MCP version.

### `--help`, `-h`

Displays usage text for the CLI and an ASCII box containing the `cargo install` command to update the CLI itself.

## 4. Usage

```sh
# Install get-vector
cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector

# Display help information
get-vector --help
# or
get-vector -h

# Display version
get-vector --version
# or
get-vector -V

# Install or update mcp-vector
get-vector update-mcp-vector

# Install or update optional RAG support
get-vector install rag
```

## 5. Distribution

`get-vector` is distributed via `cargo install --git` alongside `mcp-vector`, `vector-database`, and `vector-rag`. These crates are not published to crates.io because they depend on internal `runtime-*` path crates that cannot be resolved by the crates.io registry. See `spec-00008-mcp-vector-release-process` for the full release workflow.
