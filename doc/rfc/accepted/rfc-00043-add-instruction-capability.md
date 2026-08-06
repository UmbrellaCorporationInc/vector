---
id: rfc-00043-add-instruction-capability
type: rfc
code: "00043"
slug: add-instruction-capability
title: Add Instruction Capability
description: Replace temporary prompt-path handoff with a UUID-scoped instruction capability shared by the Vector VS Code extension and mcp-vector.
status: accepted
created: 2026-08-05
updated: 2026-08-05
authors: []
tags:
  - vscode
  - mcp
  - security
  - configuration
related:
  - task-00075-implement-rfc-00043-add-instruction-capability
supersedes: []
superseded_by: null
aliases:
  - "RFC 00043: Add Instruction Capability"
---

# RFC 00043: Add Instruction Capability

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00043-add-instruction-capability`
  document-type: task
  document-name: implement-rfc-00043-add-instruction-capability
```

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00043-add-instruction-capability
```

## 1. Problem

The VS Code frontend currently resolves a prompt into a temporary file and substitutes that file path into an agent command through the `<file>` placeholder. This exposes a filesystem location to the agent, couples command templates to frontend storage details, and makes it difficult for the MCP boundary to constrain reads. A compromised or malformed command can turn path handoff into an arbitrary-file-read opportunity.

The current contract also leaves configuration, lifecycle, and workspace behavior underspecified:

- Agent commands may depend on VS Code's implicit terminal working directory.
- Temporary-file ownership and cleanup are not consistently tied to terminal lifetime.
- The MCP cannot validate a caller-provided path as an opaque capability.
- TypeScript and Rust do not have an explicit portable contract for the instruction directory.
- Existing configurations cannot clearly distinguish the obsolete file-path contract from the new capability contract.

## 2. Proposal

### 2.1 Contract

Replace `<file>` with the semantic placeholder `<instruction>`. The frontend resolves the selected prompt and its inputs, creates a cryptographically random canonical UUID with `crypto.randomUUID()`, writes the resolved UTF-8 content to a controlled directory, builds the fixed MCP instruction directive, and substitutes that complete directive into the configured command.

The fixed filename is:

```text
vector-instruction-<canonical-uuid>.txt
```

The frontend owns the generic directive and renders the canonical UUID into it:

```text
Using the Vector MCP server, call get_instruction with id "<canonical-uuid>" and execute the returned instructions.
```

Agent command templates contain only `<instruction>` at the argument position. They do not repeat this generic text and do not expose an `<instruction-id>` placeholder. The UUID remains an internal value embedded by the frontend in the generated directive.

The MCP exposes:

```json
{
  "name": "get_instruction",
  "arguments": {
    "id": "93b0a984-5422-4491-9b0c-db44fdb69ea8"
  }
}
```

The caller controls only the UUID. The MCP controls the directory, filename prefix, filename suffix, normalization, validation, size limit, and encoding checks.

### 2.2 Configuration

Both the repository configuration and project bootstrap asset require a top-level property:

```yaml
instructions-dir: "${system-temp}/vector/instructions"
```

`${system-temp}` is the only supported variable. TypeScript resolves it with `os.tmpdir()`; Rust resolves it with `std::env::temp_dir()`. Both implementations append the remaining relative segments and normalize them according to the host platform.

Configuration validation must reject:

- A missing, empty, non-string, or malformed `instructions-dir`.
- Unsupported or repeated variable expressions.
- A suffix that is rooted, contains traversal, or escapes the resolved system temporary directory after normalization.
- Agent commands missing `<instruction>`.
- Agent commands containing the obsolete `<file>` or `<instruction-id>` placeholders.
- Malformed YAML or an unusable instruction directory.

This is an intentional breaking change. There is no compatibility fallback and no machine-specific absolute temporary path in versioned YAML. Configuration failures produce an actionable VS Code error toast and prevent terminal creation.

### 2.3 Frontend lifecycle

For both agent actions and agent buttons, the extension performs this sequence:

1. Resolve the selected prompt and substitute all declared input values.
2. Load and validate `.vector/agents.yaml`.
3. Resolve the configured instruction directory.
4. Generate a random UUID and derive the fixed filename.
5. Create the directory when necessary.
6. Create the file exclusively, write UTF-8 content, and use restrictive POSIX permissions where supported.
7. Render the frontend-owned directive with the UUID and substitute it for `<instruction>` using shell-aware argument escaping, without performing a second interpolation pass.
8. Create the terminal with the active workspace root as explicit `cwd`.
9. Associate the instruction path with that terminal for cleanup.

Vector currently supports one governed project per VS Code window. The single `workspaceRoot` selected during extension activation is authoritative for agent actions and terminal creation. Mapping multiple governed projects inside one multi-root workspace is outside this RFC.

The terminal call is explicit:

```typescript
vscode.window.createTerminal({
  name: `Vector: ${agentName} - ${label}`,
  cwd: workspaceRoot,
});
```

If command preparation or terminal creation fails after the file is created, the extension deletes that file immediately. When the associated terminal closes, it deletes the tracked file. On deactivation, it deletes every file still tracked by that extension instance.

On activation, the extension removes only regular files matching the exact recognized filename shape and older than 24 hours. It does not follow links, recurse, or delete unrelated files. Failures to clean stale files are written to a local VS Code diagnostic output channel without telemetry and without preventing activation; failures to create or write the current instruction remain fatal to agent launch.

Instruction reads remain retryable for the terminal lifetime. A successful MCP read never deletes the file.

### 2.4 MCP behavior

The reusable capability is implemented as `GetInstructionOp` in the `runtime-doc` crate, following the same operation input, output, dispatcher, and test structure as the other document operations. The `get_instruction` method in the MCP document tool group is a thin adapter: it maps the MCP UUID parameter to the operation input, invokes `GetInstructionOp` through `PluginDispatcher`, and maps the operation result back to MCP text. Filesystem access, configuration loading, validation, containment, size enforcement, encoding checks, and platform-specific link protection remain inside `runtime-doc`; `mcp-vector` must not duplicate them.

`GetInstructionOp` resolves the active project root from the MCP process working directory, loads `.vector/agents.yaml`, validates `instructions-dir`, and constructs the target path internally.

Before filesystem access, the tool requires the UUID parser's canonical hyphenated lowercase representation to exactly equal the input. It rejects traversal syntax, separators, extra characters, alternate extensions, uppercase or otherwise noncanonical encodings.

For every request, the tool:

1. Resolves and normalizes the configured directory.
2. Constructs the fixed filename internally.
3. Confirms the lexical target remains a direct child of the configured directory.
4. Inspects metadata without following links where the platform permits reliable detection.
5. Rejects symbolic links, reparse-point redirections, directories, and other non-regular targets.
6. Opens the file for reading without accepting any caller-supplied path component.
7. Enforces a maximum size of 1 MiB before and during reading.
8. Requires valid UTF-8.
9. Returns the exact content as the MCP text result.

The implementation must minimize time-of-check-to-time-of-use exposure. On Windows, `runtime-doc` adds a target-specific `windows-sys` dependency and uses the supported handle APIs to open without following reparse-point redirection, reject reparse targets, and validate the opened handle's final location. Other platforms use their strongest supported no-follow and opened-handle checks. Any remaining platform limitation must be documented and covered by the strongest available tests.

Errors distinguish invalid UUID, invalid project configuration, missing or invalid instruction directory, missing or expired instruction, non-regular target, oversized content, invalid UTF-8, and generic read failure. Messages remain actionable but do not reveal unrelated paths or filesystem contents.

### 2.5 Shared semantics

TypeScript and Rust need the same observable configuration semantics, but they must not introduce a cross-language runtime dependency merely to share code. Each side implements its parser at its existing configuration boundary and covers the same valid and invalid cases with independent table-driven tests. A shared cross-language fixture mechanism is intentionally deferred to a separate governed proposal; creating that mechanism is outside this RFC and task.

Documentation and examples must explain the distinct responsibilities:

- `.agents/mcp_config.json` starts the MCP server.
- `.vector/agents.yaml` configures agent execution and the controlled instruction directory.

## 3. Alternatives Considered

- **Keep passing a temporary path:** This has the lowest migration cost, but preserves a broad and difficult-to-audit file-read boundary. Rejected because a path is not an opaque capability.
- **Pass the full instruction content in the command:** This avoids temporary files but creates shell quoting, command-length, process-list exposure, and cross-platform encoding problems. Rejected.
- **Expose `<instruction-id>` and repeat the generic MCP directive in every agent command:** This keeps frontend substitution limited to the UUID, but leaks protocol wording into configuration and allows agent commands to drift. Rejected in favor of one semantic `<instruction>` placeholder backed by a frontend-owned directive.
- **Delete the file after the first MCP read:** This narrows retention but makes retries and multi-step agent behavior unreliable. Rejected; terminal lifetime is the ownership boundary.
- **Support legacy and new placeholders together:** This eases migration but multiplies validation and command-rendering paths while retaining obsolete contracts. Rejected because the requested change is intentionally breaking.
- **Add a shared cross-language configuration library:** This could reduce semantic drift, but adds build and release coupling between TypeScript and Rust. Rejected unless repository inspection demonstrates an existing shared boundary that makes it materially simpler.
- **Use an MCP-managed in-memory store:** This removes instruction files but requires frontend-to-MCP process coordination before agent launch and loses resilience across process boundaries. Deferred as a possible future design, not required for this capability.

## 4. Tradeoffs

| Benefit | Cost or risk |
|---|---|
| The MCP caller receives an opaque UUID instead of a path. | The UUID is a bearer capability while its file exists. |
| Fixed internal path construction sharply reduces arbitrary-file-read risk. | Safe link and reparse-point handling differs by platform and requires careful implementation. |
| A semantic `<instruction>` placeholder centralizes the MCP directive and prevents configuration drift. | Substituting a complete argument requires explicit cross-shell escaping and stronger command-rendering tests. |
| Terminal-scoped cleanup gives retryable reads with bounded lifetime. | Crashes can leave stale files until activation cleanup. |
| A required configuration makes ownership explicit. | Existing projects fail until migrated. |
| Explicit terminal `cwd` stabilizes project-root resolution. | Only the single governed project selected during extension activation is supported; multiple governed roots in one VS Code window remain out of scope. |
| Independent TS/Rust contract tests avoid runtime coupling and defer fixture infrastructure. | Duplicate cases can drift until a separate cross-language fixture proposal is accepted and implemented. |
| Windows handle validation rejects reparse redirection at the security boundary. | `runtime-doc` gains a target-specific `windows-sys` dependency and platform-specific tests. |
| A 1 MiB limit bounds resource use. | Very large generated instructions are rejected and require decomposition. |

## 5. Acceptance Criteria

### Configuration and migration

- [ ] Repository and bootstrap `.vector/agents.yaml` files define `instructions-dir: "${system-temp}/vector/instructions"`.
- [ ] Every bundled agent command uses exactly one `<instruction>` placeholder and no bundled command uses `<file>` or `<instruction-id>`.
- [ ] The generic MCP directive is owned by the frontend and is not duplicated in `.vector/agents.yaml`.
- [ ] Missing, empty, malformed, unsupported, or escaping instruction-directory values fail clearly.
- [ ] Existing configurations without the required property fail with an actionable toast and do not create a terminal.
- [ ] Agent configuration and MCP startup configuration remain documented as separate concerns.

### Frontend

- [ ] Agent actions and buttons resolve inputs, generate a UUID, write the fixed UTF-8 filename exclusively, render the frontend-owned directive, and substitute it for `<instruction>`.
- [ ] Command substitution treats the rendered directive as one shell-safe argument and does not introduce an extra interpolation or interpretation pass.
- [ ] The terminal uses the active workspace root as explicit `cwd`.
- [ ] Created files are cleaned on terminal close and extension deactivation.
- [ ] Files are deleted immediately after post-creation launch failures.
- [ ] Activation removes only recognized regular files older than 24 hours and preserves unrelated files.
- [ ] POSIX files use restrictive permissions where supported.
- [ ] Focused tests cover configuration validation, legacy-placeholder rejection, directive rendering, variable resolution, UUID naming, exclusive creation, UTF-8 writing, both launch surfaces, cross-shell quoting, explicit `cwd`, all cleanup paths, stale-file selection, unrelated-file preservation, and error-toasting.

### MCP

- [ ] `get_instruction` appears in `list_tools` and is routed through the existing server composition.
- [ ] `GetInstructionOp` owns the reusable implementation in `runtime-doc`; the MCP document tool remains a thin `PluginDispatcher` adapter with no duplicated filesystem or configuration logic.
- [ ] A canonical UUID returns the exact instruction content and repeated reads succeed.
- [ ] Invalid, noncanonical, traversal-shaped, separator-containing, and extended inputs fail before file access.
- [ ] Missing configuration, invalid directory configuration, expired instructions, non-regular targets, links where testable, oversized content, invalid UTF-8, and read failures produce bounded actionable errors.
- [ ] The caller cannot provide a path, filename, directory, extension, or variable expression.
- [ ] Target containment and platform link behavior are validated using the strongest reliable platform APIs.
- [ ] Windows uses a target-specific `windows-sys` dependency to reject reparse-point redirection and validate the opened handle's final location.
- [ ] Existing MCP tools continue to pass their tests.

### Verification

- [ ] TypeScript compilation, lint, and focused tests pass.
- [ ] Rust formatting, lint, and focused tests pass.
- [ ] MCP tool-listing and routing tests pass.
- [ ] Project bootstrap asset tests pass.
- [ ] Tests use isolated temporary directories and never modify the developer's real instruction files.
- [ ] TypeScript and Rust independently cover the same configuration contract cases; shared cross-language fixture infrastructure is not introduced by this task.
- [ ] Stale-file cleanup failures produce local diagnostic output and no telemetry.
- [ ] The instruction size limit remains a fixed 1 MiB implementation constant and is not configurable.
- [ ] Repository-prescribed quality gates pass, or unrelated pre-existing failures are explicitly separated from implementation failures.

## 6. Resolved Questions

- **Project root:** Vector currently supports one governed project per VS Code window. The `workspaceRoot` selected during extension activation is authoritative, the terminal receives it as explicit `cwd`, and the MCP operation uses the inherited process working directory. Mapping multiple governed projects is deferred.
- **Cross-language fixtures:** This RFC requires equivalent independent TypeScript and Rust test cases but does not create shared fixture infrastructure. A separate governed document may define that cross-language mechanism later.
- **Windows reparse protection:** `runtime-doc` will add target-specific `windows-sys` support and validate the opened handle and final location instead of relying only on lexical containment or pre-open metadata.
- **Cleanup reporting:** Stale-file cleanup failures produce local diagnostic logs only. The feature does not collect or emit telemetry.
- **Instruction size:** 1 MiB is sufficient for supported workflows and remains a fixed, non-configurable limit. Larger instructions must be decomposed.

## 7. Staff Engineer Assessment

The UUID contract is a material security improvement because it reduces caller authority from an arbitrary path to a narrowly shaped bearer capability. However, UUID validation alone is not the security boundary. The design is only sound if directory resolution, exclusive creation, opened-handle validation, regular-file checks, size enforcement, and lifecycle cleanup are implemented together.

The largest engineering risk is semantic drift between TypeScript and Rust, especially for variable expansion and Windows path behavior. Because shared cross-language fixture infrastructure is explicitly deferred, reviewers must compare the independent test matrices and treat mismatched cases as blocking. The second risk is overstating symbolic-link protection: lexical containment and pre-open metadata checks do not fully eliminate race conditions. Windows handle validation through `windows-sys` is required, and the implementation and completion report must state the guarantees actually achieved on each supported platform.

The semantic `<instruction>` placeholder is the correct configuration boundary because agent commands should declare where the resolved directive belongs without duplicating MCP protocol wording. This centralization introduces a stricter command-rendering responsibility: the frontend must treat the complete directive as one shell-safe argument, and cross-shell contract tests are mandatory.

The breaking migration is justified. Retaining `<file>` or `<instruction-id>` as a silent fallback would preserve obsolete boundaries and make failures harder to diagnose. The cost is immediate configuration churn for existing projects, so the migration error, documentation, and bootstrap assets are part of the feature rather than follow-up work.
