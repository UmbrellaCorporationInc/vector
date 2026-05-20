---
id: rfc-00016-add-directory-layout-for-document-types
type: rfc
code: "00016"
slug: add-directory-layout-for-document-types
title: Add Directory Layout for Document Types
description: Adds a third document-type layout named directory so governed documents can live directly under their type folder without status or category subfolders, and extends the VS Code sidebar contract to render those types correctly.
status: implemented
created: 2026-05-09
updated: 2026-05-09
authors: []
tags:
  - runtime
  - vscode
  - documentation
  - layout
related:
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00014-vs-code-governed-documents-sidebar-extension
supersedes: []
superseded_by: null
aliases:
  - "RFC 00016: Add Directory Layout for Document Types"
---

# RFC 00016: Add Directory Layout for Document Types

## 1. Problem

VECTOR currently supports only two governed document-type layouts:

- `status`
- `category`

That creates a contract gap for document types whose files should live directly under `doc/<type>/` with no grouping folder at all.

Today that gap leaks into multiple layers:

- `runtime/doc` models layout as `status | category` only
- document bootstrapping always derives a second-level folder from either `initial-status` or `category`
- document-type creation accepts only `status` or `category`
- validation assumes every governed document is either status-based or category-based
- the VS Code extension sidebar builds second-level group nodes for status or category and has no flat rendering mode for direct children

The result is that VECTOR cannot represent simple document collections without inventing artificial statuses or categories. That weakens the layout contract, adds unnecessary metadata, and makes the VS Code sidebar semantics incorrect for document types that should be flat.

## 2. Proposal

Add a third layout strategy named `directory`.

For a `directory`-based document type:

- governed files live directly under `doc/<type>/`
- there are no status folders
- there are no category folders
- the document type does not define `statuses`
- the document type does not define `initial-status`
- governed document frontmatter does not require `status`
- governed document frontmatter does not require `category`

Example:

```yaml
document-types:
  research:
    layout: directory
    code-width: 5
    prompt: prompts-00004-create-research
```

### 2.1. Runtime configuration contract

`runtime/doc` must treat `directory` as a first-class layout value alongside `status` and `category`.

Required behavior:

- `DocumentTypeConfig` helper methods must distinguish all three layout kinds
- `create_doc_type` and `bootstrap_doc_type` must accept `layout: directory`
- `statuses` must remain required only for `status`
- `initial-status` must be meaningful only for `status`
- `directory` must not create second-level folders during document-type bootstrap

### 2.2. Document bootstrap and authoring

When bootstrapping a governed document for a `directory`-based type, the target file path must be:

- `doc/<type>/<type>-<code>-<slug>.md`

This path derivation must not depend on category input or status configuration.

`create_doc`, `create_doc_prompt`, and any internal bootstrap operation used by them must therefore support direct placement under the document-type folder.

### 2.3. Validation contract

Validation for `directory`-based document types must enforce:

- the file is located directly under `doc/<type>/`
- the file name still matches `{doc_type}-{code}-{slug}.md`
- required common frontmatter fields still exist
- `status` is not required
- `category` is not required

`validate_fix` must avoid inventing synthetic folders for `directory` layouts. If a directory-based file is misplaced into a nested folder, the fix behavior should move it back to `doc/<type>/` when that correction is unambiguous.

### 2.4. Discovery and lookup

Document discovery for `directory`-based types must scan markdown files directly under `doc/<type>/` and must not require group selection.

Lookup by governed stem or by `{type, code}` should continue to work without a new API shape. The behavioral change is only that one valid search base pattern is now flat instead of grouped.

### 2.5. VS Code sidebar contract

The VS Code extension must support `directory`-based document types in the governed documents sidebar.

For a `directory` layout:

- the document-type root node expands directly to document items
- no intermediate status or category group nodes are shown
- tree items still display code and title
- no status or category badge is rendered

Search by code remains valid and unchanged.

The `List` command must remain available for consistency, but for a `directory`-based type it offers only:

- `All`

Choosing `All` must keep the root in its flat document view.

This RFC explicitly rejects introducing a fake group such as `root`, `default`, or `all` merely to preserve a two-level tree shape. That would encode UI convenience as false domain structure.

### 2.6. Scope of change

This RFC affects at least these areas:

- `runtime/doc` layout parsing and helpers
- document bootstrapping path derivation
- document-type bootstrap rules
- validation and auto-fix behavior
- MCP-facing layout acceptance for document-type creation prompts
- VS Code document discovery
- VS Code tree provider behavior
- VS Code filter command behavior for flat document types

## 3. Alternatives Considered

- **Model flat document types as `category` with one implicit folder:** Discarded because it encodes a nonexistent category and forces every file into meaningless metadata and filesystem structure.
- **Model flat document types as `status` with one implicit status:** Discarded because status implies workflow state, which is false for many document collections and would contaminate frontmatter and sidebar labels.
- **Keep the current sidebar shape by inventing a synthetic group node for `directory`:** Discarded because it preserves implementation symmetry at the cost of introducing UI semantics that are not part of the governed contract.
- **Add a broader generic layout system now:** Discarded because the current need is concrete and narrow. Expanding immediately to arbitrary layout plugins would raise implementation cost without a demonstrated second use case.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Lets VECTOR represent flat document collections without fake workflow or taxonomy metadata. | Introduces a third layout branch across runtime, MCP, and VS Code code paths. |
| Simplifies authoring for document types that do not need grouping. | Tree provider logic becomes asymmetric because some types expand to groups while others expand directly to documents. |
| Keeps the filesystem contract honest by matching real information architecture. | Validation and auto-fix need extra rules for direct-child placement. |
| Avoids synthetic badges and filters in the VS Code sidebar. | Some shared UI flows such as `List` become less meaningful for directory-based types. |
| Preserves existing search and filename conventions without changing governed identifiers. | Existing tests and helper types that assume only `status | category` must be updated broadly. |

## 5. Acceptance Criteria

- [ ] `.vector/document-types.yaml` accepts `layout: directory` as a valid document-type layout.
- [ ] `create_doc_type` and `bootstrap_doc_type` accept `directory` without requiring `statuses`.
- [ ] Bootstrapping a `directory`-based document creates the file at `doc/<type>/<type>-<code>-<slug>.md`.
- [ ] `create_doc` and `create_doc_prompt` work for `directory`-based document types without category input.
- [ ] Validation for `directory`-based documents does not require `status` or `category`.
- [ ] Validation rejects or repairs nested placement for `directory`-based documents according to a deterministic rule that returns them to `doc/<type>/` when safe.
- [ ] Existing `status` and `category` behavior remains unchanged.
- [ ] MCP-facing schemas and prompt-generation flows that describe supported layouts include `directory`.
- [ ] VS Code document discovery can list flat document files directly under `doc/<type>/`.
- [ ] In the VS Code sidebar, a `directory`-based root expands directly to document items with no intermediate group nodes.
- [ ] In the VS Code sidebar, `directory`-based document items render without status or category badges.
- [ ] The VS Code `List` command for a `directory`-based type does not invent synthetic filter values beyond `All`.
- [ ] Search by code continues to resolve `directory`-based documents correctly.
- [ ] Automated tests cover runtime config loading, bootstrap path derivation, validation, document discovery, and VS Code tree rendering for the new layout.

## 6. Open Questions

- Should validation merely ignore unexpected `status` or `category` fields on `directory`-based documents, or should it reject them to keep the layout contract strict?
- Should `validate_fix` flatten misplaced directory-based files automatically in every nested-folder case, or only when the target file path is collision-free?
- Does the VS Code root node for a `directory`-based type need a distinct context key so commands can tailor behavior without branching on raw layout strings throughout the extension?
