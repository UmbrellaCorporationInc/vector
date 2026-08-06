# runtime-project

Provisioning and bootstrapping boundary for governed projects in the VECTOR workspace.

## Ownership and Responsibility

This crate owns the **project skeleton definition**. It is responsible for:
- Embedding all baseline assets (templates, configurations, rules) at compile time.
- Provisioning these assets to a target directory using `runtime-io` boundaries.
- Implementing the "skip-existing" policy to ensure no accidental data loss during repeated bootstrapping.

## Dependency Boundary

To maintain architectural integrity and portability, `runtime-project` strictly adheres to the following boundaries:
- **No MCP SDK:** Does not depend on MCP types or SDKs. It is transport-agnostic.
- **No Shell/Git:** Does not initialize Git repositories or execute shell commands. It only handles file and directory provisioning.
- **No Direct IO:** All filesystem operations must go through `runtime-io`.

## Usage

The primary entry point is the `CreateProjectOp` plugin operation:

```rust
use runtime_project::{CreateProjectOp, CreateProjectInput};
// ... running the operation via a dispatcher or directly in tests
```

## Bootstrap Asset: `.vector/agents.yaml`

The provisioned `.vector/agents.yaml` configures agent execution and the instruction capability for the Vector VS Code extension and `mcp-vector`. Every bootstrapped project receives:

```yaml
instructions-dir: "${system-temp}/vector/instructions"
agents:
  claude:
    type: cli
    command: claude '<instruction>'
  ...
```

- `instructions-dir` is required and must begin with `${system-temp}`. The VS Code extension writes a UUID-scoped instruction file to this directory before spawning an agent terminal; `mcp-vector` reads it back through `get_instruction`. Omitting this field or using the obsolete `<file>` placeholder is a validation failure that prevents terminal creation.
- `<instruction>` is the only supported placeholder for agent commands. The frontend renders the complete MCP directive and substitutes it as one shell-safe argument.

This is a **breaking change** from the legacy `<file>` path handoff. Projects bootstrapped before this change must add `instructions-dir` and replace every `<file>` placeholder with `<instruction>` to restore agent execution.

## Policy: Skip Existing

By default, the `create_project` operation will skip any file that already exists at the target path. It will continue provisioning the rest of the skeleton and report all skipped files in the output.
