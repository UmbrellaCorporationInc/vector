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
│   └── vscode/          # VS Code extension
└── doc/                 # Governed documentation vault
    ├── adr/             # Architecture Decision Records
    ├── ai-rule/         # Operational rules for AI agents
    ├── design/          # System and component designs
    ├── rfc/             # Requests for Comments
    ├── spec/            # Technical specifications
    └── task/            # Project task tracking
```

## Getting started

### Prerequisites

- Rust 1.95.0 or later
- `cargo` in your PATH

### Build

```sh
cargo build --release
```

The main binary is `mcp-vector`, produced from the `mcp/vector` crate.

### Run as an MCP server

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

## Design principles

- **Decentralization** — documentation lives close to teams and codebases, not in a central silo
- **Performance** — Rust systems layer with efficient local execution
- **Extensibility** — small core, plugin-based growth
- **Configurability** — no single workflow imposed; projects define their own ways of working
