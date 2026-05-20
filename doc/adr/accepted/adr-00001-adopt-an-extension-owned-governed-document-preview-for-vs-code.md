---
id: adr-00001-adopt-an-extension-owned-governed-document-preview-for-vs-code
type: adr
code: "00001"
slug: adopt-an-extension-owned-governed-document-preview-for-vs-code
title: Adopt an Extension-Owned Governed Document Preview for VS Code
description: Records the decision to replace native Markdown Preview integration with a Vector-owned governed preview for governed documents and wikilink navigation.
status: accepted
created: 2026-05-08
updated: 2026-05-08
authors: []
tags:
- vscode
- frontend
- markdown
- preview
related:
- rfc-00014-vs-code-governed-documents-sidebar-extension
- task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
supersedes: []
superseded_by: null
aliases:
- "ADR 00001: Adopt an Extension-Owned Governed Document Preview for VS Code"
---

# ADR: Adopt an Extension-Owned Governed Document Preview for VS Code

> [!ABSTRACT] Prime Directive
> **Context & Entropy Source:** VS Code native Markdown Preview extension points are too constrained for VECTOR governed-document wikilink navigation and controlled reading flows.
> **Objective:** Adopt an extension-owned preview, rendered with `markdown-it`, for governed documents opened through the Vector VS Code extension.

Related governed documents:

- [[rfc-00014-vs-code-governed-documents-sidebar-extension]]
- [[task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension]]

---

## 1. System Topology

- **Target Modules:** `frontend/vscode/vector`
- **Action:** Replace the governed-document reading flow that depends on native Markdown Preview with an extension-owned preview surface driven by `markdown-it`.
- **Scope boundary:** This preview applies only to governed documents opened through Vector commands or tree interactions. It must not replace the default Markdown experience for unrelated workspace files.

## 2. Logical Justification

- **Cost of Opportunity:** Continuing to extend native Markdown Preview would keep Vector dependent on limited VS Code preview hooks, increase workaround complexity for wikilinks, and make future governed-reading behavior brittle.
- **Entropy Reduction:** An extension-owned preview gives Vector full control over render pipeline, wikilink interaction, and document-to-document navigation while keeping governed document lookup logic in one frontend-safe integration boundary.
- **Alternatives Rejected:**
- *Alternative A: Continue with native Markdown Preview.* Discarded because the current extension points are insufficient for reliable governed wikilink navigation and impose ongoing implementation debt.
- *Alternative B: Replace Markdown handling globally for all workspace files.* Discarded because it expands scope unnecessarily, degrades isolation, and would impose Vector-specific behavior on non-governed content.

## 3. Decision

Vector will introduce an extension-owned governed document preview for the VS Code extension.

The preview must follow these constraints:

- It uses `markdown-it` as the rendering pipeline.
- It is read-focused and limited to governed documents.
- It reuses the same governed document lookup and stem-resolution rules used by the sidebar navigation flow.
- Clicking a governed wikilink resolves the target by governed stem and reopens the resolved document inside the same governed preview flow.
- It does not claim ownership of general Markdown editing or preview for unrelated files.

## 4. Tradeoffs

- **Gain:** Deterministic control over rendering, navigation, and future governed-document UX.
- **Gain:** Clear isolation of Vector-specific behavior to Vector-owned reading surfaces.
- **Cost:** Higher maintenance burden than native Markdown Preview integration.
- **Cost:** RFC 00014 must be updated because its current preview strategy and rejected-alternative rationale no longer match the chosen direction.

## 5. Consequences

- [[rfc-00014-vs-code-governed-documents-sidebar-extension]] must be revised to replace native Markdown Preview as the primary governed reading surface.
- A new RFC and implementation task must define delivery of the extension-owned governed preview without retroactively expanding [[task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension]].
- The extension should expose one preview-opening path for governed documents so tree selection and wikilink navigation do not fork into separate resolution behaviors.

## 6. Validation Vector

- [ ] The VS Code extension can render a governed document in an extension-owned preview without relying on native Markdown Preview hooks.
- [ ] The governed preview is only invoked for governed documents opened through Vector flows.
- [ ] Governed wikilink clicks resolve targets by governed stem and reopen the target inside the governed preview flow.
- [ ] The preview implementation reuses the same governed lookup boundary as sidebar document resolution.
- [ ] [[rfc-00014-vs-code-governed-documents-sidebar-extension]] and the new follow-up RFC/task are updated to reflect this decision.
