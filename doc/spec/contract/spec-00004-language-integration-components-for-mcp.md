---
id: spec-00004-language-integration-components-for-mcp
type: spec
code: "00004"
slug: language-integration-components-for-mcp
title: Language Integration Components for MCP
description: Defines the contract for language-specific tool names, runtime integration crates, and shared tree-sitter analysis support used when integrating a programming language into the MCP.
category: contract
created: 2026-05-05
updated: 2026-05-08
authors: []
tags:
  - mcp
  - runtime
  - language
  - contract
related:
  - spec-00001-repository-directory-structure
  - spec-00002-runtime-core-crate
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00004: Language Integration Components for MCP"
---

# SPEC 00004: Language Integration Components for MCP

## 1. Purpose

This spec defines the contract for the components required to integrate a programming language into the `vector` MCP ecosystem.

This spec follows [[spec-00001-repository-directory-structure]], follows [[spec-00002-runtime-core-crate]], and supports [[rfc-00001-thin-mcp-facade-over-runtime-libraries]].

## 2. Definition

Each supported programming language must define one language integration unit identified by `<lang>`.

For each `<lang>`, the integration contract contains six component families:

- one testing tool named `<lang>_test_suite`
- one linting tool named `<lang>_lint`
- one formatting tool named `<lang>_format`
- one security audit tool named `<lang>_audit`
- one runtime integration crate named `lang_<lang>`
- one shared language analysis dependency on the cross-language `runtime/lang/` crate

Tool naming contract:

- `<lang>_test_suite` is the canonical testing tool name for the language
- `<lang>_lint` is the canonical linting tool name for the language
- `<lang>_format` is the canonical formatting tool name for the language
- `<lang>_audit` is the canonical security audit tool name for the language
- canonical tool names must be stable across MCP, CLI adapters, automation, and documentation
- aliases may exist for compatibility, but all governance documents must reference the canonical names

Tool implementation contract:

- a tool named `<lang>_test_suite` may be implemented as a Rust crate, a command written in the target language, or a thin adapter over an existing language-native command
- a tool named `<lang>_lint` may be implemented as a Rust crate, a command written in the target language, or a thin adapter over an existing language-native command
- a tool named `<lang>_format` may be implemented as a Rust crate, a command written in the target language, or a thin adapter over an existing language-native command
- a tool named `<lang>_audit` may be implemented as a Rust crate, a command written in the target language, or a thin adapter over an existing language-native command
- the implementation technology of a tool does not change its canonical contract name
- MCP-facing integrations must treat the testing, linting, formatting, and audit tool families as command-capable execution targets, not as crates by default

Subcommand parallelization contract:

- every CLI tool in the integration contract must expose its independently executable steps as named subcommands
- subcommands that have no data dependency on each other must be safe to invoke in parallel by an agent spawning multiple subprocesses
- the minimum required subcommands for `<lang>_audit` are `deps`, `secrets`, and `sast`; implementations may define additional subcommands beyond these three
- `deps` must validate dependency vulnerabilities against an advisory source
- `secrets` must scan source code for exposed sensitive credentials and secrets
- `sast` must perform static analysis security checks on the language source code
- the subcommand set for `<lang>_audit` is open: implementations may extend it without violating this contract
- tools whose steps are inherently sequential and have no parallelizable decomposition are exempt from this requirement

Runtime integration crate contract:

- `runtime/lang_<lang>/` is the canonical repository location when the integration is implemented as a reusable runtime crate
- the `lang_<lang>` crate owns reusable plugins, adapters, contracts, and orchestration logic specific to one language
- `lang_<lang>` exists to support MCP tools, CLI commands, and any other future frontend that needs reusable language-specific behavior
- `lang_<lang>` may invoke `<lang>_test_suite`, `<lang>_lint`, `<lang>_format`, `<lang>_audit`, or other language-native commands through accepted runtime execution contracts
- `lang_<lang>` must not force every language tool to be reimplemented as a Rust crate

Shared analysis crate contract:

- `runtime/lang/` is the canonical repository location for the shared language analysis crate
- the `lang` crate owns cross-language parsing and code analysis capabilities built on `tree-sitter`
- `lang` may expose reusable syntax tree traversal, language registration, query helpers, and normalization utilities
- `lang` must remain cross-language and must not absorb ownership of one language's testing, linting, or plugin policy
- `lang_<lang>` crates may depend on `lang`

Ownership boundaries:

- `mcp/vector/` may expose tools backed by `lang_<lang>`, `<lang>_test_suite`, `<lang>_lint`, `<lang>_format`, `<lang>_audit`, or combinations of them
- reusable language-specific behavior belongs in `runtime/lang_<lang>/` when that behavior is needed by more than one transport or command surface
- reusable cross-language parsing behavior belongs in `runtime/lang/`
- transport-specific request handling must remain outside `runtime/lang/` and `runtime/lang_<lang>/`

Current repository note:

- this repository snapshot on 2026-05-05 does not contain `frontend/`
- therefore this spec does not require `<lang>_test_suite`, `<lang>_lint`, or `<lang>_format` to currently live under a repository CLI package
- if a future `frontend/cli/` package exposes these tools, that package must still preserve the canonical tool names defined by this spec

## 3. Invariants

- Every supported language must define exactly one canonical testing tool name: `<lang>_test_suite`.
- Every supported language must define exactly one canonical linting tool name: `<lang>_lint`.
- Every supported language must define exactly one canonical formatting tool name: `<lang>_format`.
- Every supported language must define exactly one canonical security audit tool name: `<lang>_audit`.
- The canonical names of testing, linting, formatting, and audit tools must use the same `<lang>` token as the corresponding `lang_<lang>` crate.
- Tool names must use lowercase snake_case.
- Runtime integration crates for languages must use the `lang_<lang>` naming pattern.
- Shared language analysis must live in `runtime/lang/` and must not be split into duplicated per-language parsing foundations.
- `runtime/lang/` must use `tree-sitter` as its parsing foundation when it provides syntax analysis behavior.
- `runtime/lang/` must remain language-agnostic and must not own per-language lint or test execution policy.
- `runtime/lang/` must remain language-agnostic and must not own per-language formatting execution policy.
- `runtime/lang/` must remain language-agnostic and must not own per-language audit execution policy.
- `runtime/lang_<lang>/` must remain language-specific and must not become a catch-all home for cross-language parsing infrastructure.
- No spec, RFC, CLI, or MCP document may assume that `<lang>_test_suite`, `<lang>_lint`, `<lang>_format`, or `<lang>_audit` are always Rust crates.
- When a tool is implemented as a wrapper around a language-native command, the wrapper must preserve the canonical tool contract name at the integration boundary.
- Every CLI tool in the integration contract must expose independently executable steps as named subcommands when those steps can be executed in parallel.
- `<lang>_audit` must expose at minimum the subcommands `deps`, `secrets`, and `sast` as independently invocable execution targets.
- The subcommand set of `<lang>_audit` is open; implementations may define additional subcommands beyond the minimum required set without violating this contract.
- Tools whose execution steps are inherently sequential with no parallelizable decomposition are exempt from the subcommand exposure requirement.

## 4. Examples

Repository layout example with one integrated language:

```text
.
|-- mcp/
|   `-- vector/
|       |-- Cargo.toml
|       `-- src/
`-- runtime/
    |-- lang/
    |   |-- Cargo.toml
    |   `-- src/
    `-- lang_rust/
        |-- Cargo.toml
        `-- src/
```

Valid naming examples:

- `rust_test_suite`
- `rust_lint`
- `rust_format`
- `rust_audit`
- `runtime/lang_rust/`
- `runtime/lang/`

Valid implementation examples:

- `rust_test_suite` is a Rust crate that wraps `cargo test` orchestration and reporting.
- `python_lint` is a Python command that runs the accepted Python lint pipeline.
- `rust_format` is a Rust command that wraps the accepted Rust formatting pipeline.
- `rust_audit` is a tool that exposes `rust_audit deps`, `rust_audit secrets`, and `rust_audit sast` as independently invocable subcommands; an agent may spawn all three in parallel.
- `python_audit` is a Python command that wraps `pip-audit` for `deps`, `trufflesecurity` for `secrets`, and `semgrep` for `sast`; it exposes each as a subcommand.
- `lang_rust` provides reusable Rust plugins used by MCP tools and CLI commands.
- `runtime/lang/` provides `tree-sitter`-based parsing helpers shared by `lang_rust` and future language crates.

Invalid examples:

- `runtime/rust_lang/`
- `runtime/tree_sitter_rust/` as the canonical shared analysis crate
- assuming `python_lint` must be implemented as a Rust crate
- assuming `python_audit` must be implemented as a Rust crate
- placing cross-language `tree-sitter` infrastructure inside `lang_rust`
- documenting a testing tool as `rust-test-suite` instead of `rust_test_suite`
- documenting an audit tool as `rust-audit` or `rust_security` instead of `rust_audit`
- implementing `rust_audit` without exposing `deps`, `secrets`, and `sast` as subcommands
- merging the three audit subcommand steps into a single non-decomposable execution path that prevents agent parallelization

## 5. Open Questions

- None.
