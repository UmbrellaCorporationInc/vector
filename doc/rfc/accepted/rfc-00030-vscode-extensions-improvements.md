---
id: rfc-00030-vscode-extensions-improvements
type: rfc
code: "00030"
slug: vscode-extensions-improvements
title: VS Code Extension Cross-Package Wikilinks and Package Sync
description: This RFC proposes cross-package wikilink resolution, a package sync action in the VS Code extension, and package-aware document lookup support in MCP vector tools.
status: accepted
created: 2026-06-06
updated: 2026-06-06
authors:
  - Codex
tags:
  - vscode
  - mcp
  - packages
  - wikilinks
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00030: VS Code Extension Cross-Package Wikilinks and Package Sync"
---

# RFC 00030: VS Code Extension Cross-Package Wikilinks and Package Sync

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00030-vscode-extensions-improvements`
  document-type: task
  document-name: implement-rfc-00030-vscode-extensions-improvements
```

## 1. Problem

The current VS Code extension resolves wikilinks only within the active workspace context and does not support explicit package-qualified references. This creates three concrete gaps:

- Users cannot reliably link to governed documents stored in synchronized packages by using `[[package/doc-id]]`.
- Unqualified links such as `[[doc-id]]` have no documented fallback rule for local-only resolution versus package-aware lookup.
- The extension tree view lacks an explicit action to refresh synchronized package metadata, forcing users to leave the editor to run package sync commands manually.

This limitation also leaks into MCP vector tooling. If the agent receives an identifier like `package/doc-id`, it cannot consistently infer that `package` identifies a synchronized package namespace and that `doc-id` is the target governed document identifier.

## 2. Proposal

After this RFC is accepted, the following behavior must exist:

1. The VS Code extension supports two wikilink forms:
   - `[[doc-id]]`: resolve within the current workspace.
   - `[[package/doc-id]]`: resolve against the synchronized package named `package`.
2. Clicking either wikilink opens the governed document by searching in the appropriate package location under the packages folder.
3. The extension tree view includes a new `Sync` action alongside the existing validate, search, refresh, and create doc type actions.
4. Invoking `Sync` opens a terminal and executes `vector-database package sync`.
5. MCP vector tools that accept document identifiers, including `find_doc` and any related lookup tool, must accept package-qualified identifiers in the form `package/doc-id`.
6. MCP vector lookup logic must parse the identifier into:
   - `package`: synchronized package namespace
   - `doc-id`: governed document identifier
7. MCP vector must preserve current behavior for non-qualified identifiers so existing workspace-only flows do not regress.

Implementation guidance:

- Package-qualified parsing should be centralized rather than duplicated across extension handlers and MCP tool entrypoints.
- Resolution should fail clearly when the package is unknown, unsynchronized, or the target document does not exist.
- The extension should reuse existing open-document behavior after resolution instead of introducing a second navigation path.

## 3. Alternatives Considered

- **Alternative A: Support only `[[doc-id]]` and search every synchronized package implicitly.** Discarded because it introduces ambiguous matches, weakens determinism, and makes link intent impossible to reason about.
- **Alternative B: Add cross-package lookup only in MCP vector, but not in the VS Code extension.** Discarded because the editor and agent would diverge on the same link format, creating user confusion and inconsistent behavior.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Explicit `package/doc-id` syntax makes cross-package references deterministic. | Users must learn a second wikilink form for cross-package references. |
| A dedicated `Sync` action reduces context switching from editor to shell. | The tree view gains another command and can become noisier if actions keep accumulating. |
| Package-aware MCP parsing aligns editor and agent behavior. | Tool contracts and validation paths become more complex because identifiers now have two supported formats. |
| Backward compatibility for `[[doc-id]]` avoids breaking existing links. | Ambiguity remains possible if future requirements want global search for unqualified identifiers. |

## 5. Acceptance Criteria

- [ ] Clicking `[[doc-id]]` resolves and opens a document from the active workspace using current behavior.
- [ ] Clicking `[[package/doc-id]]` resolves and opens a document from the matching synchronized package.
- [ ] The VS Code extension searches the packages folder when a package-qualified wikilink is activated.
- [ ] The tree view exposes a `Sync` action near the existing validate, search, refresh, and create doc type actions.
- [ ] The `Sync` action opens a terminal and runs `vector-database package sync`.
- [ ] MCP vector accepts package-qualified identifiers for document lookup flows without breaking existing non-qualified identifiers.
- [ ] Unknown packages and missing documents return actionable errors instead of silent failures.
- [ ] Tests cover local wikilinks, package-qualified wikilinks, sync command invocation, MCP identifier parsing, and failure cases.

## 6. Open Questions

- Should `[[doc-id]]` remain strictly workspace-local forever, or should future behavior optionally search synchronized packages on ambiguity?
- Which MCP tools, beyond `find_doc`, must become package-aware in the first implementation to avoid partial support?
- Where should synchronized package metadata be cached so both the extension and MCP vector can share a stable view of package names and paths?
