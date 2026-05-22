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
| `thiserror`    | Error enum derivation                                       |

## 3. Commands

### `update-mcp-vector`

Runs `cargo install --git <repo> --force mcp-vector` to install or update the local `mcp-vector` binary from the repository HEAD.

V1 always performs a full install. No version comparison is done — this avoids a chicken-and-egg problem where an outdated CLI binary would incorrectly skip reinstallation after a workspace version bump.

**Exit behavior:**

| Condition                       | Exit code |
|---------------------------------|-----------|
| Install succeeded               | `0`       |
| `cargo install` exited non-zero | `1`       |
| Failed to spawn `cargo`         | `1`       |

## 4. Usage

```sh
# Install get-vector
cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector

# Install or update mcp-vector
get-vector update-mcp-vector
```

## 5. Distribution

`get-vector` is distributed via `cargo install --git` alongside `mcp-vector`. Neither crate is published to crates.io because both depend on internal `runtime-*` path crates that cannot be resolved by the crates.io registry. See `spec-00008-mcp-vector-release-process` for the full release workflow.
