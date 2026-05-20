---
id: rfc-00020-add-create-document-actions-to-vscode-treeview-doc-type-folders
type: rfc
code: "00020"
slug: add-create-document-actions-to-vscode-treeview-doc-type-folders
title: Add Create Document Actions to VS Code Treeview
description: Add per-doc-type and global create actions in the VS Code governed documents tree that open configured create-form documents in the document viewer.
status: implemented
created: 2026-05-12
updated: 2026-05-12
authors: []
tags:
  - vscode
  - viewer
  - treeview
  - forms
related:
  - rfc-00018-enhanced-markdown-code-blocks
  - rfc-00017-vs-code-dashboard-viewer-extension
supersedes: []
superseded_by: null
aliases:
  - "RFC 00020: Add Create Document Actions to VS Code Treeview Doc Type Folders"
---

# RFC 00020: Add Create Document Actions to VS Code Treeview

## 1. Problem

The VS Code governed documents tree lets users browse document types and open existing documents, but it does not provide direct creation flows for either of the two authoring entry points the extension now needs:

- creating a governed document for a selected `doc_type`
- creating a brand-new governed document type

Today the user must leave the tree context and manually assemble whatever creation document or workflow is required. This creates several gaps:

- the treeview exposes document-type folders as navigation containers only, not as action surfaces
- there is no contract that lets a `doc_type` declare which create-form document should open for authoring
- there is no global contract that lets the extension resolve a create-form for new document types
- the extension has no standard way to instantiate a create-document form as temporary viewer content
- document creation UX is disconnected from the embedded form and action primitives already defined for the governed markdown viewer
- the governed documentation validator does not enforce the create-form fields required by these extension flows

This friction is especially visible for governed document types that need structured user input before a document can be created.

## 2. Proposal

Extend the VS Code governed documents tree with two create entry points:

- every `doc_type` folder exposes a `Create Document` action
- the treeview exposes one global `Create Document Type` action

When the user activates `Create Document` on a doc-type folder:

1. the extension resolves the selected `doc_type`
2. the extension loads the create-form document associated with that `doc_type`
3. the extension instantiates a temporary markdown document in a temp location
4. the extension replaces required bootstrap placeholders in that temp content
5. the extension opens the instantiated temp document in the existing governed `document_viewer`

The initial placeholder contract introduced by this RFC is:

- `#{document-type}` -> replaced with the selected document type identifier such as `rfc`, `task`, or `spec`

When the user activates the global `Create Document Type` action:

1. the extension loads the create-form document configured at the global `doc_type` configuration level
2. the extension opens that document in the existing governed `document_viewer`
3. the extension performs no placeholder substitution before opening it

The opened document may contain embedded forms and embedded action buttons. Those interactive blocks are not redefined here. They must reuse the existing viewer behavior already defined by [[rfc-00018-enhanced-markdown-code-blocks]].

### 2.1. Document type configuration contract

Each governed document type under `.vector/document-types.yaml` may define a new field named `create-document-form`.

Proposed shape:

```yaml
document-types:
  rfc:
    description: Request for Comments - architectural decisions and proposals.
    layout: status
    code-width: 5
    prompt: prompts-00002-create-doc
    create-document-form: prompts-00010-rfc-create-form
```

Required behavior:

- `create-document-form` is a governed document identifier
- the referenced document is the source template opened when the user triggers `Create Document` for that `doc_type`
- if a `doc_type` does not define `create-document-form`, the extension must not expose the create action for that folder
- if `create-document-form` is configured but cannot be resolved, the extension must show a bounded error instead of failing the whole treeview

This keeps creation entry points declarative and colocated with document-type governance metadata.

At the global `doc_type` configuration level, `.vector/document-types.yaml` may define a property named `create-document-type-form`.

Proposed shape:

```yaml
doc-type:
  template: template-00004-doc-type-template
  prompt-template: template-00005-doc-type-prompt
  prompt: prompts-00001-create-doc-type
  create-document-type-form: form-00002-create-document-type
```

Required behavior:

- `create-document-type-form` is a governed document identifier
- the referenced document is the source document opened when the user triggers the global `Create Document Type` action
- the extension opens that configured document as-is, without any placeholder substitution
- if `create-document-type-form` is absent, the global `Create Document Type` action is not shown
- if `create-document-type-form` is configured but cannot be resolved, the extension must show a bounded error for that action

### 2.2. Validation contract for governed configuration

The `validate` plugin operation in `runtime/doc` must enforce the create-form fields required by this RFC.

Required behavior:

- validation fails when any governed document type under `document_types` is missing `create-document-form`
- validation fails when the global `doc_type` configuration is missing `create-document-type-form`
- validation errors must point to `.vector/document-types.yaml` as the source of the contract violation
- validation must keep all existing governed configuration checks intact

This requirement intentionally makes the create-form contract mandatory at validation time even though the extension still treats unresolved runtime actions as bounded local failures.

### 2.3. Treeview action contract

The VS Code extension must add a `Create Document` button to each doc-type folder item in the governed documents tree.

Required behavior:

- the button is attached to the tree item that represents the `doc_type` folder
- the button is visible only when that `doc_type` has a valid `create-document-form` configuration
- the button invokes a dedicated extension command with the selected `doc_type` as input
- the button does not appear on status folders, category folders, directory folders, or leaf documents

The intent is to keep document creation anchored to the exact document type the user has chosen, without adding global toolbar ambiguity.

The VS Code extension must also add one global `Create Document Type` action at the governed-documents tree level.

Required behavior:

- the action is shown in the treeview title or equivalent global action surface
- the action is visible only when `doc_type.create-document-type-form` is configured
- the action invokes a dedicated extension command for document-type creation
- the action is not attached to any doc-type folder item because it operates on the global `doc_type` contract

### 2.4. Create-form document instantiation contract

When the user triggers `Create Document`, the extension resolves the configured `create-document-form` document and creates a temporary instantiated copy.

Required behavior:

- the source create-form document remains unchanged on disk
- the instantiated document is written to a temp location controlled by the extension
- the temp document content is based on the source document content after placeholder replacement
- the extension must replace `#{document-type}` before opening the temp document in the viewer
- the temp document may later collect additional values through embedded forms rendered in the viewer

The initial contract is intentionally narrow. This RFC standardizes one required replacement token now so the creation flow can become operational without designing a broader variable system prematurely.

For the global document-type creation flow, there is no instantiation contract in this RFC beyond loading the configured source document and opening it unchanged in the viewer.

### 2.5. Viewer integration contract

The temporary instantiated create-form document must open in the existing governed `document_viewer`.

Required behavior:

- the extension reuses the existing viewer pipeline instead of introducing a second create-form-specific viewer
- embedded `vector-form`, `vector-agent-button`, `vector-agent-action`, and related interactive blocks continue to behave according to [[rfc-00018-enhanced-markdown-code-blocks]]
- the viewer renders the temp document as ordinary governed markdown content after substitution
- actions triggered from the temp document operate on the instantiated content, not on the original source form document

This RFC therefore depends on existing viewer-side interactive block support rather than redefining form execution behavior.

### 2.6. Error handling contract

The extension must fail locally and clearly when the create flow cannot be completed.

Required behavior:

- if the selected `doc_type` has no `create-document-form`, no create button is shown
- if `doc_type.create-document-type-form` is absent, the global `Create Document Type` action is not shown
- if the configured create-form document cannot be found, the user receives a bounded error message for that action
- if the temp document cannot be written, the user receives a bounded error message for that action
- failures in one `doc_type` create flow must not break tree rendering for other document types
- failures in the global document-type create flow must not break the governed documents treeview

### 2.7. Scope of change

This RFC affects at least these areas:

- `.vector/document-types.yaml` contract for `create-document-form`
- `.vector/document-types.yaml` contract for `doc_type.create-document-type-form`
- `runtime/doc` validation behavior for required create-form fields
- governed documents tree item modeling and command wiring in the VS Code extension
- create-form document resolution
- temp document instantiation and placeholder substitution
- opening temp create-form content in the existing governed `document_viewer`
- opening the configured document-type create form unchanged in the existing governed `document_viewer`
- automated tests for tree item action visibility, document resolution, temp-file creation, substitution, and bounded failure behavior

## 3. Alternatives Considered

- **Add one global `Create Document` toolbar action for the whole tree:** Discarded because creation depends on `doc_type` context, and a global action would force a second selection step or ambiguous defaults.
- **Reuse the per-doc-type instantiation flow for global document-type creation:** Discarded because the requested document-type create form should open unchanged and does not require placeholder replacement in this RFC.
- **Open the configured create-form source document directly without creating a temp copy:** Discarded because placeholder substitution and user-interactive form state must not mutate the governed source document.
- **Store the create-form path outside `.vector/document-types.yaml`:** Discarded because the association is part of document-type behavior and should remain declarative alongside other doc-type metadata.
- **Keep the create-form fields optional at validation time and only enforce them inside the extension UI:** Discarded because the extension feature depends on governed configuration completeness, and missing fields should be caught as repository contract failures before runtime.
- **Build a brand-new dedicated create-form webview instead of reusing `document_viewer`:** Discarded because the viewer already has the markdown rendering and embedded interactive block model needed for this flow.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Makes document creation discoverable exactly where users already choose a document type. | Adds another action surface to each doc-type tree item that must remain visually tidy. |
| Adds a dedicated global entry point for creating new document types without overloading doc-type folder actions. | Introduces two related creation flows that must stay consistent but intentionally behave differently. |
| Keeps create-form selection declarative in `.vector/document-types.yaml`. | Introduces a new config field that must be validated and documented. |
| Lets repository validation fail early when required create flows are not fully configured. | Removes the ability to omit create-form fields for document types that are not meant to be creatable through the extension. |
| Protects governed source documents by instantiating temp copies before user interaction. | Requires temp-file lifecycle management and cleanup discipline in the extension. |
| Reuses the existing `document_viewer` and embedded form/action blocks instead of duplicating UI logic. | Couples the create flow to the viewer's interactive block behavior and temp-document handling. |
| Starts with a minimal substitution contract centered on `#{document-type}`. | Future creation flows may need more variables, which could require a follow-up RFC to extend the substitution model. |

## 5. Acceptance Criteria

- [ ] `.vector/document-types.yaml` supports a required `create-document-form` field for governed document types.
- [ ] `.vector/document-types.yaml` supports a required `doc_type.create-document-type-form` field for global document-type creation.
- [ ] The `runtime/doc` `validate` plugin operation fails when any `document_types.<type>` entry omits `create-document-form`.
- [ ] The `runtime/doc` `validate` plugin operation fails when `doc_type.create-document-type-form` is missing.
- [ ] The VS Code governed documents tree shows a `Create Document` action on a doc-type folder only when that type defines `create-document-form`.
- [ ] The VS Code governed documents tree shows a global `Create Document Type` action only when `doc_type.create-document-type-form` is configured.
- [ ] Activating the action invokes a dedicated extension command with the selected `doc_type`.
- [ ] Activating the global action invokes a dedicated extension command for document-type creation.
- [ ] The extension resolves `create-document-form` to exactly one governed source document.
- [ ] The extension resolves `doc_type.create-document-type-form` to exactly one governed source document.
- [ ] The extension writes an instantiated temp markdown document instead of modifying the governed source document.
- [ ] Before opening the temp document, the extension replaces `#{document-type}` with the selected document type identifier.
- [ ] The temp instantiated document opens in the existing governed `document_viewer`.
- [ ] The configured document-type create form opens in the existing governed `document_viewer` without placeholder replacement.
- [ ] Embedded forms and action blocks inside the temp document continue to work through the existing viewer contract defined by [[rfc-00018-enhanced-markdown-code-blocks]].
- [ ] Embedded forms and action blocks inside the document-type create form continue to work through the existing viewer contract defined by [[rfc-00018-enhanced-markdown-code-blocks]].
- [ ] Missing or broken `create-document-form` configuration fails only the local action and does not break the rest of the treeview.
- [ ] Missing or broken `create-document-type-form` configuration fails only the global action and does not break the rest of the treeview.
- [ ] Automated tests cover action visibility per doc type, configured-form resolution, placeholder substitution, temp-document opening, and bounded error handling.

## 6. Open Questions

- Should `create-document-form` resolve only `prompts` documents, or can it target any governed document type intended to act as a create form?
- Should the temp instantiated document be deleted immediately after the viewer closes, or should the extension keep it for the duration of the VS Code session?
- Should future placeholder expansion remain hardcoded in the extension, or should create-form documents declare their required input variables explicitly?
