---
id: rfc-00010-runtime-io-command-spec-and-executor-separation
type: rfc
code: "00010"
slug: runtime-io-command-spec-and-executor-separation
title: Runtime IO Command Spec and Executor Separation
description: Separates command specification from command execution in runtime-io to support interchangeable real and mock executors.
status: implemented
created: 2026-05-03
updated: 2026-05-04
authors: []
tags:
  - runtime
  - io
  - shell
  - architecture
related:
  - rfc-00009-runtime-io-file-access-and-shell-command-execution
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
aliases:
  - "RFC 00010: Runtime IO Command Spec and Executor Separation"
---

# RFC 00010: Runtime IO Command Spec and Executor Separation

## 1. Problem

[[rfc-00009-runtime-io-file-access-and-shell-command-execution]] currently describes shell command support in `runtime-io` as one builder that both prepares the command and starts real process execution through `spawn`.

That shape leaves one architectural gap unresolved:

- command description and command side effects are coupled into the same API boundary

This coupling creates predictable friction:

- callers cannot reuse one command description across different execution strategies
- tests that need deterministic process behavior must either fake the full builder surface or launch real commands
- the crate cannot provide a standard mock executor without retrofitting the public API later
- higher-level crates cannot separate command planning from command execution policy

The project needs one stable command model that supports at least two execution modes:

- a real executor backed by operating-system processes
- a mock executor for tests, simulations, and deterministic validation

## 2. Proposal

Keep shell command support in `runtime-io`, but split it into two explicit layers:

- command specification
- command execution

### Accepted design decision

`CommandBuilder` must build a data-only command specification.

`CommandBuilder` must not spawn real processes directly.

Execution must move into a separate command executor contract.

### Command specification

Define one runtime-owned command specification type.

Illustrative direction:

```rust
pub struct CommandSpec {
    /* private fields */
}

pub struct CommandBuilder {
    /* private fields */
}

impl CommandBuilder {
    pub fn new(command: impl Into<String>) -> Self;
    pub fn arg(self, argument: impl Into<String>) -> Self;
    pub fn args(self, arguments: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn current_dir(self, path: impl AsRef<std::path::Path>) -> Self;
    pub fn env(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn build(self) -> RuntimeResult<CommandSpec>;
}
```

Accepted responsibilities of `CommandSpec`:

- store the executable command
- store ordered arguments
- store optional working directory
- store environment mutations or additions

Accepted non-responsibilities of `CommandSpec`:

- no process creation
- no shell parsing
- no execution policy
- no retry policy
- no scheduling policy
- no test-only fake behavior embedded in the spec

### Command executor contract

Define one execution contract that receives a `CommandSpec` and returns a running command handle.

Illustrative direction:

```rust
pub trait CommandExecutor {
    fn spawn(
        &self,
        spec: CommandSpec,
    ) -> impl Future<Output = RuntimeResult<CommandHandle>> + Send;
}
```

Accepted responsibilities of `CommandExecutor`:

- validate or normalize execution-time concerns required by the runtime implementation
- create the running process or mock process
- return the accepted `CommandHandle`

The executor contract is the execution boundary. The builder and spec do not own side effects.

### Standard implementations

`runtime-io` may provide multiple executor implementations, but this RFC requires at least the following conceptual roles:

- one real executor for operating-system process execution
- one mock-friendly executor boundary that allows deterministic testing

Illustrative direction:

```rust
pub struct ProcessCommandExecutor {
    /* private fields */
}

impl CommandExecutor for ProcessCommandExecutor {
    async fn spawn(&self, spec: CommandSpec) -> RuntimeResult<CommandHandle> {
        /* ... */
    }
}
```

The mock support may be a crate-internal executor, a public test utility, or direct mock `CommandHandle` construction support. The public architecture must allow deterministic tests without changing the accepted command model later.

### Relationship with [[rfc-00009-runtime-io-file-access-and-shell-command-execution]]

This RFC narrows and updates the command portion of [[rfc-00009-runtime-io-file-access-and-shell-command-execution]].

After this RFC is accepted:

- `CommandBuilder::spawn` is no longer the accepted direction
- `CommandBuilder::build` becomes the accepted direction
- `CommandExecutor::spawn` becomes the accepted execution boundary
- `CommandHandle`, `CommandOutput`, and `CommandInput` remain the running-process boundary after execution begins

This RFC does not change the accepted file, memory, text, or path portions of [[rfc-00009-runtime-io-file-access-and-shell-command-execution]].

### Command handle boundary

The accepted running-command boundary remains unchanged from [[rfc-00009-runtime-io-file-access-and-shell-command-execution]] once execution starts.

- `CommandOutput` remains a concrete type implementing `Receiver<Bytes>`
- `CommandInput` remains a concrete type implementing `Sender<Bytes>`
- `CommandHandle` remains the owner of stdout, stderr, stdin, `wait`, and `Drop`

This separation matters because the executor contract chooses how a handle is created, but not what a running handle means to callers.

### Ownership boundary

`runtime-io` owns:

- `CommandSpec`
- `CommandBuilder`
- `CommandExecutor`
- the real process-backed executor implementation
- `CommandHandle`, `CommandOutput`, and `CommandInput`
- mock `CommandHandle` construction support used for deterministic tests

`runtime-io` does not own:

- shell-form parsing rules
- repository-specific command policies
- workflow-specific retry or fallback policy
- test orchestration policy outside the mock executor boundary itself

## 3. Alternatives Considered

- **Keep `CommandBuilder::spawn` as the only public API:** Discarded because it couples command construction to real side effects and makes mock execution a retrofit instead of a first-class boundary.
- **Hide an executor internally but keep builder-driven spawn publicly:** Discarded because the public contract would still communicate the wrong ownership model and would not help callers that need interchangeable executors.
- **Represent the spec as plain tuples or ad hoc structs per call site:** Discarded because command configuration is a stable runtime boundary and should not be improvised by each caller.
- **Move the executor contract into `runtime-core`:** Discarded because process execution is a concrete runtime capability, not a transport-agnostic primitive.
- **Use only a mock executor in tests without changing the public API:** Discarded because tests would still depend on hidden implementation choices rather than an accepted execution contract.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Separating `CommandSpec` from execution makes command construction reusable and side-effect free. | The API grows by at least one extra public type and one extra step at call sites. |
| A dedicated `CommandExecutor` contract allows real and mock execution without later API surgery. | Callers that only need the real executor face slightly more ceremony than a direct `spawn` on the builder. |
| Higher-level crates can decide when and how to execute a prepared command. | The split introduces one more ownership boundary that must be documented clearly. |
| Tests can validate command planning independently from process execution. | If the mock executor surface is underspecified, different test helpers may still drift in behavior. |
| The command portion of `runtime-io` becomes more consistent with the contract-versus-implementation discipline used elsewhere in the runtime architecture. | The RFC must now keep `CommandSpec`, executor behavior, and handle behavior aligned across multiple documents. |

## 5. Acceptance Criteria

- [ ] `runtime-io` exposes `CommandSpec` as the data-only command specification type.
- [ ] `runtime-io` exposes `CommandBuilder` as the standard way to construct `CommandSpec`.
- [ ] `CommandBuilder` configures the executable command explicitly.
- [ ] `CommandBuilder` configures ordered arguments explicitly.
- [ ] `CommandBuilder` configures an optional working directory.
- [ ] `CommandBuilder` configures environment variables.
- [ ] `CommandBuilder::build` returns `CommandSpec`.
- [ ] `CommandBuilder` introduces no process side effects before `build`.
- [ ] `runtime-io` exposes a `CommandExecutor` execution contract.
- [ ] `CommandExecutor` accepts `CommandSpec` and returns `CommandHandle`.
- [ ] `runtime-io` provides one real executor implementation for operating-system process execution.
- [ ] The accepted architecture allows one mock executor implementation without changing the public command model.
- [ ] `CommandHandle`, `CommandOutput`, and `CommandInput` remain the running-command boundary after execution starts.
- [ ] `CommandOutput` continues to implement `Receiver<Bytes>`.
- [ ] `CommandInput` continues to implement `Sender<Bytes>`.
- [ ] The command API introduces no shell-form parsing rules, retry policy, scheduling policy, or repository-specific command workflow logic.
- [ ] [[rfc-00009-runtime-io-file-access-and-shell-command-execution]] is updated or interpreted consistently so that builder-driven direct execution is no longer the accepted command direction.

## 6. Open Questions

- Should `CommandExecutor` remain a static-dispatch contract in v1, or does the runtime eventually need a separate object-safe execution boundary?
- Should `CommandSpec` preserve environment configuration as a full map, an ordered overlay list, or another explicit representation that keeps duplicate-key behavior deterministic?
- Should the real executor be named `ProcessCommandExecutor`, `SystemCommandExecutor`, or another name that better communicates operating-system process ownership?
- Should `CommandSpec` remain fully owned, or should some fields allow shared borrowed inputs during builder construction before `build` materializes ownership?
- Should mock command support remain centered on mock `CommandHandle` construction, or should a public mock executor become necessary later?
