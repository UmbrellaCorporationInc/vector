---
id: rfc-00028-authoring-document-operations
type: rfc
code: "00028"
slug: authoring-document-operations
title: Document-Scoped Authoring Patch Operation
description: Define a crate doc authoring operation and MCP tool that update governed documents from git diffs using patcher while refusing writes outside the doc tree and rejecting BOM-encoded content.
status: accepted
created: 2026-05-30
updated: 2026-05-30
authors:
  - Codex
tags:
  - governance
  - authoring
  - documents
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00028: Document-Scoped Authoring Patch Operation"
---

# RFC 00028: Document-Scoped Authoring Patch Operation

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00028-authoring-document-operations`
  document-type: task
  document-name: implement-rfc-00028-authoring-document-operations
```

## 1. Problem

The current document authoring flow can create or update governed documents, but it does not define a narrow operation dedicated to patching an existing governed document from a git diff. That gap leaves several risks unresolved:

- The agent may modify files outside the governed `doc/` tree.
- Diff application behavior is underspecified, which weakens reproducibility and auditability.
- Encoding rules are implicit, so a write could introduce a UTF-8 BOM and break downstream tooling or document validation.

The crate doc layer needs a single operation that accepts a document type, a document code, and a git diff; resolves the governed document; applies the patch safely; and returns the final document content. That operation also needs an MCP-facing tool boundary so agents can invoke it through the server without gaining broader filesystem mutation capabilities.

## 2. Proposal

Add a document authoring operation in crate doc with the following contract:

- Input:
  - `doc_type`
  - `code`
  - `git_diff`
- Lookup:
  - Resolve the governed target document by `doc_type` and `code`.
- Scope enforcement:
  - Allow updates only when the resolved path is inside the repository `doc/` directory.
  - Refuse any patch that would create, rename, or modify files outside `doc/`.
- Patch execution:
  - Use the `patcher` crate as the embedded unified-diff engine.
  - Apply the provided git diff to the resolved document update flow.
  - Treat the diff as an update operation, not a general repository patch executor.
  - Normalize agent-produced patch payloads to raw unified diff before invoking `patcher`.
  - Reject unsupported patch shapes such as non-text patches, rename-only patches, delete patches, or any patch whose target does not match the resolved governed document.
- Encoding enforcement:
  - Before writing, verify the resulting content is UTF-8 without BOM.
  - If BOM is present in the target content or in the generated replacement content, do not write anything.
  - Return an explicit error instructing the agent to remove the BOM and retry.
- Output:
  - Return the final document content after the patch is successfully applied.

Expose this operation through an MCP tool named `patch_doc`:

- Tool name:
  - `patch_doc`
- Tool responsibility:
  - Accept MCP input for `doc_type`, `code`, and `git_diff`.
  - Delegate the patch behavior to the crate doc operation.
  - Return the final patched document content or a structured validation error.
- Boundary rule:
  - `patch_doc` is a thin transport adapter and must not duplicate patching rules, path authorization rules, or encoding logic that belong in the runtime operation.

Dependency governance:

- Register `patcher` in [project-0003-rust-dependencies.md](C:/Users/ferna/OneDrive/Obsidian/vector/doc/project/project-0003-rust-dependencies.md) before the implementation is merged.
- Limit the approval scope to the crate that owns this document authoring operation unless a broader use case is approved separately.

This operation is intentionally narrow. It does not authorize arbitrary repository edits, non-document writes, or automatic BOM cleanup.

## 3. Alternatives Considered

- **General repository patch operation:** Discarded because it expands the blast radius from governed document authoring to unrestricted repository mutation.
- **Direct string replacement without git diff semantics:** Discarded because it loses context, makes review harder, and increases the risk of malformed updates.
- **Auto-strip BOM during write:** Discarded because silent normalization hides content hygiene problems and can mask upstream tool defects.
- **Use `git2` or libgit2-backed apply semantics:** Discarded for this operation because it adds native dependency and repository-level complexity that is disproportionate to a single-document governed patch flow.
- **Implement patching directly inside the MCP layer:** Discarded because it would duplicate runtime rules, weaken testability, and leak transport concerns into document mutation logic.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Tight scope reduces accidental writes outside `doc/`. | The operation cannot be reused for non-document authoring tasks. |
| `patcher` keeps the diff engine embeddable in Rust without introducing a native Git dependency. | `patcher` compatibility is strongest for git-style unified text diffs, so agent output still needs normalization and rejection rules. |
| Git diff input preserves reviewable intent and aligns with existing patch workflows. | Diff parsing and application add implementation complexity and failure modes. |
| BOM rejection keeps encoding failures explicit and auditable. | The agent must perform a cleanup step before retrying, which adds friction. |
| Returning final content gives the caller a deterministic post-write artifact. | Large documents may increase response payload size. |

## 5. Acceptance Criteria

- [ ] The operation accepts `doc_type`, `code`, and `git_diff`.
- [ ] The MCP server exposes a tool named `patch_doc` for this capability.
- [ ] The operation resolves the governed document from `doc_type` and `code`.
- [ ] The operation refuses to modify any file outside the repository `doc/` directory.
- [ ] The operation uses `patcher` as the embedded diff engine for unified text patches.
- [ ] The operation normalizes or rejects agent-produced patch wrappers before patch application.
- [ ] The operation applies git-diff-based updates only to the resolved governed document flow.
- [ ] The `patch_doc` tool delegates to the runtime operation rather than reimplementing patching logic in the MCP layer.
- [ ] The operation verifies that outgoing content is UTF-8 without BOM before writing.
- [ ] If BOM is detected, the operation performs no write and returns an explicit remediation error.
- [ ] On success, the operation returns the final document content.
- [ ] The `patcher` dependency is registered in [project-0003-rust-dependencies.md](C:/Users/ferna/OneDrive/Obsidian/vector/doc/project/project-0003-rust-dependencies.md) before implementation merge.
- [ ] Tests cover valid patch application, out-of-scope path rejection, missing document handling, malformed diff rejection, and BOM rejection.

## 6. Open Questions

- Should the operation reject multi-file diffs outright, or may it accept them when every touched file still resolves under `doc/`?
- Should the returned document content be the entire file or a structured response that includes metadata and content separately?
- How strict should the normalization layer be when extracting raw unified diff content from agent-formatted Markdown or mixed prose responses?
