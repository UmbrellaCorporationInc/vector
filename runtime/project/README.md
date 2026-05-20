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

## Policy: Skip Existing

By default, the `create_project` operation will skip any file that already exists at the target path. It will continue provisioning the rest of the skeleton and report all skipped files in the output.
