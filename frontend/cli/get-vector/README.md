# `get-vector`

## 1. Objective

`get-vector` is the operator CLI for managing the local `mcp-vector` installation. It provides a single command surface to install or update the `mcp-vector` binary from the vector repository without requiring a manual `cargo install` invocation.

## 2. Boundaries

### In scope

- Installing and updating the local `mcp-vector` binary
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

Runs `cargo install --git <repo> --force mcp-vector` to install or update the local `mcp-vector` binary from the repository HEAD.

V1 always performs a full install. No version comparison is done — this avoids a chicken-and-egg problem where an outdated CLI binary would incorrectly skip reinstallation after a workspace version bump.

Cargo's stdout and stderr are streamed live to the terminal as the install runs, so the command does not appear blocked during the (often lengthy) compilation step.

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

## 4. Usage

```sh
# Install get-vector
cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector

# Install or update mcp-vector
get-vector update-mcp-vector
```

## 5. Distribution

`get-vector` is distributed via `cargo install --git` alongside `mcp-vector`. Neither crate is published to crates.io because both depend on internal `runtime-*` path crates that cannot be resolved by the crates.io registry. See `spec-00008-mcp-vector-release-process` for the full release workflow.
