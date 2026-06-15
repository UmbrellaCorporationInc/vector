# Vector

**Velocity Engine for Code, Tooling, Operations, and Release**

Vector is a unified development control surface that supports the full software delivery lifecycle through MCP (Model Context Protocol) tools, CLI commands, and editor extensions. Its initial focus is documentation governance — creating, validating, and organizing governed documentation vaults.

## What it does

- **Documentation governance** — create, update, validate, and organize governed docs through MCP tools
- **MCP server** — stdio-based server exposing tools to any MCP-compatible client (Claude Code, etc.)
- **Extensible runtime** — small core that grows through plugins, with a transport-agnostic design that supports MCP, CLI, and future frontends

## Tech stack

- **Rust** (Edition 2024, MSRV 1.95.0)
- **Async runtime**: Tokio
- **Protocol**: MCP via the `rmcp` SDK
- **Transport**: stdio

## Workspace layout

```
vector/
├── mcp/
│   └── vector/          # MCP server — tool registration and protocol handling
├── runtime/
│   ├── core/            # Fundamental traits, types, operations, events
│   ├── channel/         # Tokio-backed channels with cancellation
│   ├── io/              # File, memory, path, text, and shell I/O boundaries
│   ├── doc/             # Documentation governance operations
│   ├── project/         # Project bootstrap and plugin operations
│   └── language/        # Language operations and prompt resolution
├── frontend/
│   ├── cli/
│   │   └── get-vector/  # Operator CLI — install and update mcp-vector
│   └── vscode/          # VS Code extension
└── doc/                 # Governed documentation vault
    ├── adr/             # Architecture Decision Records
    ├── ai-rule/         # Operational rules for AI agents
    ├── design/          # System and component designs
    ├── rfc/             # Requests for Comments
    ├── spec/            # Technical specifications
    └── task/            # Project task tracking
```

## Installation

### Protocol Buffers compiler (`protoc`)

Some workspace builds now require the Protocol Buffers compiler `protoc`
through the LanceDB dependency graph.

Install `protoc` with the package manager for your platform:

```sh
# Linux
apt install -y protobuf-compiler
protoc --version

# macOS
brew install protobuf
protoc --version

# Windows
winget install protobuf
protoc --version
```

Ensure `protoc --version` reports a sufficiently recent compiler after
installation.

### MCP server (`mcp-vector`)

Install the latest release directly from the repository:

```sh
cargo install --git https://github.com/UmbrellaCorporationInc/vector mcp-vector
```

To install a specific version:

```sh
cargo install --git https://github.com/UmbrellaCorporationInc/vector --tag v0.1.1 mcp-vector
```

**Prerequisites:** Rust 1.95.0 or later. Install from [rustup.rs](https://rustup.rs).

### Operator CLI (`get-vector`)

`get-vector` is a companion CLI for operators managing a local `mcp-vector` installation. It installs or updates the binary from the repository HEAD via `cargo install --git --force`:

```sh
cargo install --git https://github.com/UmbrellaCorporationInc/vector get-vector
```

Once installed, run the update command:

```sh
get-vector update-mcp-vector
```

This always installs the latest `mcp-vector` from the repository. Version-aware reconciliation (skip when already current) is planned for a future release.

### VS Code extension

Install the **Vector** extension from the VS Code marketplace, or search for `vector` in the Extensions panel.

## Configuration

### MCP client

Add the server to your MCP client configuration (e.g., `.mcp.json`):

```json
{
  "mcpServers": {
    "vector": {
      "type": "stdio",
      "command": "mcp-vector"
    }
  }
}
```

Then start your MCP client. Vector exposes its tools through the MCP protocol over stdio.

## Building from source

```sh
git clone https://github.com/UmbrellaCorporationInc/vector
cd vector
cargo build --release
```

The `mcp-vector` binary is produced at `target/release/mcp-vector`.

## Design principles

- **Decentralization** — documentation lives close to teams and codebases, not in a central silo
- **Performance** — Rust systems layer with efficient local execution
- **Extensibility** — small core, plugin-based growth
- **Configurability** — no single workflow imposed; projects define their own ways of working
