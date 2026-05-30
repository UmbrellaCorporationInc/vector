---
id: task-00051-implement-rfc-00027-update-find-doc
type: task
code: "00051"
slug: implement-rfc-00027-update-find-doc
title: "Implement RFC 00027: Expand find-doc to return package and content"
description: Evolve the runtime and MCP `find_doc` contract so callers receive path, reserved package field, and document content in one lookup.
status: in-progress
created: 2026-05-30
updated: 2026-05-30
tags:
  - runtime
  - documentation
  - mcp
  - api
related:
  - rfc-00027-update-find-doc
  - rfc-00013-runtime-doc-validation-and-authoring-crate
supersedes: []
superseded_by: null
---

# Task 00051: Implement RFC 00027: Expand find-doc to return package and content

## 1. Prime Directive

> [!Prime Directive]
> `find_doc` currently forces higher-level consumers to perform a second file read after lookup and exposes no stable slot for future package-aware scoping. This task removes that friction by enriching the runtime and MCP contract in one backward-conscious change.

## 2. Specs

- **Module:** `runtime/doc`, `mcp/vector`
- **Dependencies:**
  - `runtime-io` for reading resolved document content
  - `runtime-core` and `runtime-channel` for operation and dispatcher flow
  - no new external dependency unless existing crates cannot satisfy file reading

## 3. Checklist

### 3.1. Phase A - Runtime `find_doc` Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00051
  phase: Phase A
  language: rust, markdown
```

- [x] Add `package` to `FindDocInput` and make the implementation ignore its value
- [x] Extend `FindDocOutput` to include `path`, empty `package`, and `content`
- [x] Read the resolved document content during the same lookup operation
- [x] Preserve existing repository-wide lookup semantics and current not-found behavior
- [x] Add or update runtime tests for success, unknown type, not found, directory-based lookup, ignored input package, empty output package, and returned content
- [x] Quality gates for runtime changes pass

### 3.2. Phase B - MCP Adapter and Tool Surface

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00051
  phase: Phase B
  language: rust, markdown
```

- [ ] Extend MCP `FindDocParams` to accept `package`, `doc_type`, `code`, and `root_dir`
- [ ] Update the MCP `find_doc` tool handler to pass the reserved `package` field through and return the enriched response shape
- [ ] Update schema-oriented tests so the tool contract requires the RFC fields and exposes the new response expectations
- [ ] Update adapter tests to cover populated content and stable empty package output
- [ ] Review caller impact and document any compatibility risk if existing consumers assume a path-only string response
- [ ] Quality gates for MCP changes pass

### 3.3. Phase C - Documentation and Validation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00051
  phase: Phase C
  language: markdown, rust
```

- [ ] Update package documentation that describes `find_doc` behavior and contract
- [ ] Confirm the implemented behavior satisfies every RFC 00027 acceptance criterion
- [ ] Capture gaps, flaws, and tradeoffs discovered during implementation, especially payload size and client compatibility
- [ ] Run task-relevant automated tests and validation commands

### 3.4. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00051
  phase: Phase Z
  language: rust, markdown
```

- [ ] `xtask quality-lint` passes
- [ ] `xtask quality-test` passes
- [ ] Update README files on packages modified
