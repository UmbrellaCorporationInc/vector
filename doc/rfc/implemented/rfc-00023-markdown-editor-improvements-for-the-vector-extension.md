---
id: rfc-00023-markdown-editor-improvements-for-the-vector-extension
type: rfc
code: "00023"
slug: markdown-editor-improvements-for-the-vector-extension
title: Markdown Editor Improvements for the Vector Extension
description: Proposes inline header actions, prompt-enriched execution, clearer action visibility, and a global validate-fix action in the container view.
status: implemented
created: 2026-05-18
updated: 2026-05-18
authors: []
tags:
  - editor
  - markdown
  - vector-agent
related:
  - prompts-00006-update-document
supersedes: []
superseded_by: null
aliases:
  - "RFC 00023: Markdown Editor Improvements for the Vector Extension"
---

# RFC 00023: Markdown Editor Improvements for the Vector Extension

## 1. Problem

The markdown editor currently exposes agent actions in a way that limits in-place document workflows.
Authors cannot trigger a contextual action from a header while supplying additional prompt content at
execution time, and existing `vector-agent-action` elements are not visually distinct enough for
reliable discovery. The container view also exposes search, reload, collapse, and add-doc-type
controls, but it does not provide an equivalent `validate-fix` entry point for running governed
document repair flows from the same surface. This creates friction for document maintenance tasks
that should be fast, local, and explicit.

## 2. Proposal

This RFC proposes five coordinated changes to the markdown editor in the Vector extension:

1. Add a new action type named `vector-agent-inline-action`.
2. Render an inline action on every markdown header that binds to `prompts-00006-update-document`.
3. Display header inline actions with a pencil-style icon, preferably using a UTF glyph when viable.
4. Add visible styling for existing `vector-agent-action` elements.
5. Add a global `validate-fix` action to the container view.

### 2.1 New `vector-agent-inline-action`

`vector-agent-inline-action` will behave similarly to `vector-agent-button` and `vector-agent-action`,
but it introduces a pre-execution overlay. The overlay must:

- Open before action execution.
- Provide a chat-style input for author-supplied extra content.
- Merge that content into the final prompt payload under the `prompt-message` variable.
- Include an action information control that also triggers the action when clicked.

This preserves the current action execution model while enabling lightweight prompt enrichment without
forcing the author to leave the editor context.

### 2.2 Header-bound inline actions

Every markdown header rendered in the editor must expose an inline action wired to
`prompts-00006-update-document` from the `form-actions` category in the `prompts` doc type.

The action input contract must include at minimum:

- `document-stem`
- `profile`, using the value `create-doc`

The intent is to make document update flows available exactly where authors identify a section that
needs revision.

### 2.3 Header iconography

The inline action should use a pencil-like affordance to communicate edit intent. A UTF glyph is
preferred if it is visually stable across the supported rendering surfaces. If glyph consistency is
not acceptable, a dedicated icon asset or existing icon component should be used instead.

### 2.4 Visible styling for `vector-agent-action`

Existing `vector-agent-action` elements must gain explicit styling so they are recognizable as
interactive controls. The styling should improve affordance without overpowering markdown content.
At minimum, the component should have a clear hover state, focus state, and non-ambiguous default
appearance.

### 2.5 Container view `validate-fix` action

The container view currently shows search, reload, collapse, and add-doc-type actions. It must gain
an additional `validate-fix` action that executes a governed repair flow for the repository.

Required behavior:

- The action is exposed as a global container-level control, not as a per-document-type action.
- Triggering the action calls the agent using the `create-doc` profile.
- The prompt used by that action is resolved from the global `doc-type` block in
  `.vector/document-types.yaml`.
- The configured field name is `doc-type.prompt-validate-fix`.
- The resolved prompt applies to all document types because it is declared under the shared `doc-type`
  contract, not under individual `document-types.<type>` entries.

This keeps the validate-fix flow aligned with the repository-level document governance model instead
of fragmenting it by document type.

## 3. Alternatives Considered

- **Reuse `vector-agent-action` without an overlay:** Discarded because it cannot collect structured
  prompt augmentation at the moment of execution without overloading the existing interaction model.
- **Use a full modal workflow outside the editor surface:** Discarded because it adds too much friction
  for a task that should remain local to a header or inline editing context.
- **Configure `validate-fix` per document type:** Discarded because the requested action is global to
  the container view and should resolve from the shared `doc-type` contract that applies across all
  governed document types.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Inline prompt enrichment enables more precise agent execution. | Overlay behavior adds UI complexity and state management cost. |
| Header-local actions reduce navigation and make document maintenance faster. | Rendering actions on every header may create visual noise in dense documents. |
| Stronger action styling improves discoverability and accessibility. | Styling changes can alter existing layout expectations and require regression testing. |
| UTF icon usage keeps implementation lightweight when supported. | Cross-platform glyph inconsistency may force a fallback implementation. |
| A global `validate-fix` action gives users one consistent repair entry point at container level. | The flow now depends on a new repository-wide configuration field that must exist and resolve correctly. |

## 5. Gaps and Risks

- The RFC does not yet define the exact payload merge strategy beyond assigning the extra content to
  `prompt-message`; implementation must specify escaping, trimming, and empty-input behavior.
- The overlay interaction model needs keyboard and focus-management rules to avoid accessibility
  regressions.
- Header action injection must be tested against nested markdown structures and large documents to
  avoid rendering or performance regressions.
- The `form-actions` category reference is treated as intentional, but implementation should verify the
  governed prompt metadata to avoid binding to an incorrect identifier.
- The global `doc-type.prompt-validate-fix` field does not appear in the current
  `.vector/document-types.yaml` sample, so implementation must define validation behavior when the
  field is absent or points to an unresolved governed prompt.

## 6. Acceptance Criteria

- [ ] The editor supports a new `vector-agent-inline-action` component or action mode.
- [ ] Triggering that action opens an overlay before execution.
- [ ] The overlay includes a chat-style input whose submitted content is passed as `prompt-message`.
- [ ] The overlay includes an additional control that exposes action information and triggers the action.
- [ ] Every rendered markdown header exposes an inline action tied to `prompts-00006-update-document`.
- [ ] The header action passes `document-stem` and `profile=create-doc`.
- [ ] The header action uses a pencil-like visual affordance.
- [ ] Existing `vector-agent-action` elements have visible default, hover, and focus styling.
- [ ] The container view exposes a global `validate-fix` action alongside its existing controls.
- [ ] Activating that action invokes the agent with the `create-doc` profile.
- [ ] The `validate-fix` action resolves its prompt from `doc-type.prompt-validate-fix` in `.vector/document-types.yaml`.
- [ ] The resolved `doc-type.prompt-validate-fix` contract is treated as global and applies across all document types.

## 7. Open Questions

- Should empty `prompt-message` values be omitted from the payload or sent as an empty string?
- Should header inline actions appear persistently or only on hover/focus?
- Should the overlay action information control expose prompt metadata, execution details, or both?
- Should the container hide the `validate-fix` action when `doc-type.prompt-validate-fix` is absent, or
  should it show the action and fail with a bounded error at runtime?
