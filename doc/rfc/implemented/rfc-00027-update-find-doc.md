---
id: rfc-00027-update-find-doc
type: rfc
code: "00027"
slug: update-find-doc
title: Expand find-doc to return package and content
description: Proposes evolving find-doc to return package and content in addition to the resolved document path.
status: implemented
created: 2026-05-30
updated: 2026-05-30
authors: []
tags:
  - governance
  - architecture
  - mcp
  - api
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00027: Expand find-doc to return package and content"
---

# RFC 00027: Expand find-doc to return package and content

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00027-update-find-doc`
  document-type: task
  document-name: implement-rfc-00027-update-find-doc
```

## 1. Problem

`find_doc` currently returns only the file path of the matched document. That is enough for simple navigation, but it is too narrow for higher-level consumers that need to understand the package boundary and read document content without a second lookup step.

The input contract is also too rigid for the planned evolution of package-aware lookup. Today the lookup is effectively global within `root_dir`, and there is no explicit place in the API to constrain the search to a package.

This creates three concrete issues:

- Callers must perform extra file I/O after `find_doc` just to read the document body.
- The API shape does not reserve a stable field for future package scoping.
- Future changes risk becoming breaking changes if package support is added later without preparing the contract now.

## 2. Proposal

Evolve `find_doc` so that its request and response carry the minimum structure needed for package-aware lookup and direct content consumption.

### Request

`find_doc` should accept:

- `package`: reserved field for future evolution. For now callers may send any value, but the implementation must ignore it.
- `doc_type`: the governed document type to locate.
- `code`: the numeric document code to locate.
- `root_dir`: the repository root used for resolution.

### Response

`find_doc` should return:

- `path`: the resolved document path.
- `package`: always an empty value for now. The field remains in the contract only to reserve the shape for a future RFC.
- `content`: the document content as text.

### Behavioral expectations

- The implementation must ignore the input value of `package`.
- The response value of `package` must always be empty.
- Lookup behavior should remain equivalent to the current global lookup within `root_dir`.
- When no matching document exists, the operation should continue to signal absence through the existing error or not-found path used by the implementation.
- `content` must reflect the current contents of the resolved file at lookup time.
- `package` must be present in the response even when empty so downstream clients can rely on a stable shape.
- The meaning and validation rules for `package` are intentionally deferred to a future RFC.

### Scope boundary

This RFC does not define package resolution semantics. It only reserves the request and response fields needed to add that behavior safely later. The authoritative meaning and usage rules for `package` will be introduced in a separate RFC.

## 3. Alternatives Considered

- **Keep returning only `path`:** Rejected because every consumer that needs content would still need a second step, and the API would remain unprepared for package-aware lookup.
- **Add only `content` and defer `package` fields:** Rejected because it would solve only part of the problem and would likely force another contract change when package semantics are introduced.
- **Create a separate `read_doc` operation instead of evolving `find_doc`:** Rejected for now because the immediate requirement is to enrich the existing lookup flow, not to split the API surface into separate discovery and read stages.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Reduces follow-up file reads for consumers that need the document body. | Response payloads become larger because the full content is returned. |
| Establishes a forward-compatible place for package scoping. | The API becomes more opinionated before package semantics are fully implemented. |
| Keeps lookup and read concerns together for common client flows. | Returning content may be unnecessary overhead for callers that only need the path. |

### Gaps

- This RFC does not define the future type or semantics of `package`; it only states that the field is ignored on input and empty on output for now.
- This RFC does not define encoding, size limits, or binary-file handling because governed documents are expected to be text.
- This RFC does not specify whether `content` should include frontmatter normalization or preserve raw file bytes beyond textual reading.

### Flaws and risks

- If large documents are returned frequently, response size may become a performance concern.
- Because `package` is intentionally ignored for now, some consumers may incorrectly assume it already has behavior unless the contract is documented clearly.
- Existing consumers may assume the old response shape unless the change is versioned or coordinated carefully.

## 5. Acceptance Criteria

- [x] `find_doc` accepts `package`, `doc_type`, `code`, and `root_dir`.
- [x] `find_doc` returns `path`, `package`, and `content`.
- [x] Any input value provided for `package` is ignored by the implementation.
- [x] The returned `package` field is always empty.
- [x] Lookup behavior remains compatible with current repository-wide resolution.
- [x] `content` is populated from the resolved document file in the same operation.
- [x] The returned `package` field is present even when empty.
- [x] Existing callers are migrated or compatibility handling is documented before release.

## 6. Open Questions

- Should the enriched response replace the current response in place, or should it be introduced behind a versioned interface?
- Should `content` always be returned, or should the API eventually support an opt-in flag for metadata-only lookups?
- Which future RFC will define the meaning, validation rules, and matching semantics of `package`?
