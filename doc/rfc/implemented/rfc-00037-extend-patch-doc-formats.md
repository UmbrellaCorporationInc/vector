---
id: rfc-00037-extend-patch-doc-formats
type: rfc
code: "00037"
slug: extend-patch-doc-formats
title: Extend Document Patch and Replacement Operations
description: Proposes extending patch_doc to support explicit patch formats and adding replace_doc for full governed document replacement.
status: implemented
created: 2026-06-11
updated: 2026-06-11
authors: []
tags:
  - mcp
  - patching
  - documents
related:
  - spec-00001-repository-directory-structure
  - task-00066-implement-rfc-00037-extend-patch-doc-formats
  - task-00067-update-mcp-tool-patch-doc
supersedes: []
superseded_by: null
aliases:
  - "RFC 00037: Extend Document Patch and Replacement Operations"
---

# RFC 00037: Extend Document Patch and Replacement Operations

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00037-extend-patch-doc-formats`
  document-type: task
  document-name: implement-rfc-00037-extend-patch-doc-formats
```

## 1. Problem

`patch_doc` currently accepts unified diff input. Unified diffs are precise and familiar to source-control tooling, but they are error-prone for agent-authored edits because the caller must produce valid hunk headers, line counts, and prefixed blank context lines.

This creates avoidable failures for simple document edits. A caller can know exactly which text should change and still fail because the serialized diff is malformed. The failure mode is especially common when a hunk includes blank lines or when an agent manually estimates hunk ranges.

The project already uses an `apply_patch`-style patch format in agent workflows. That format is easier for agents to author because it identifies the target operation and file explicitly and does not require numeric hunk ranges for common updates.

There is also a separate bootstrap friction. After `create_doc_prompt` creates a governed document skeleton, agents sometimes need to replace the entire document with authored Markdown. For that workflow, forcing the caller to synthesize a full-document patch adds risk without adding meaningful safety. The operation already has an authoritative document target through `doc_type`, `code`, and optional `package`; the caller should be able to provide the full replacement content directly.

## 2. Proposal

### 2.1. Extend `patch_doc` Formats

Extend `patch_doc` so callers can choose the patch syntax with a `format` field:

- `format: "unified"` means the patch payload is interpreted as a standard unified diff.
- Omitted `format` means the patch payload is interpreted as an `apply_patch`-style patch.

The default format is intentionally the `apply_patch`-style schema because it is the safer default for agent-generated edits. Unified diff remains available for callers that already have source-control-native patches.

The tool input should support this shape:

```json
{
  "root_dir": "/path/to/project",
  "doc_type": "rfc",
  "code": 37,
  "format": "unified",
  "patch": "--- a/doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n+++ b/doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n@@ -1 +1 @@\n-old\n+new\n"
}
```

When `format` is omitted, the same tool should parse `patch` as:

```patch
*** Begin Patch
*** Update File: doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md
@@
-old
+new
*** End Patch
```

The existing document targeting fields remain authoritative. `patch_doc` should reject patches that attempt to update a different governed document than the one resolved by `doc_type`, `code`, and optional `package`.

The tool should normalize validation errors by format. For unified diffs, errors should mention invalid hunk counts, missing prefixes, or target-file mismatches. For `apply_patch`-style patches, errors should mention missing patch boundaries, unsupported operations, ambiguous context, or target-file mismatches.

### 2.2. Add `replace_doc`

Add a complementary `replace_doc` operation in `runtime-doc` and expose it through an MCP tool named `replace_doc`.

`replace_doc` should receive the same authoritative document targeting fields as `patch_doc`, but instead of a patch payload it receives the complete replacement document content as plain text:

```json
{
  "root_dir": "/path/to/project",
  "doc_type": "task",
  "code": 66,
  "content": "---\nid: task-00066-implement-rfc-00037-extend-patch-doc-formats\ntype: task\ncode: \"00066\"\nslug: implement-rfc-00037-extend-patch-doc-formats\n---\n\n# Task 00066: Implement RFC 00037\n"
}
```

The replacement content is not a patch, diff, or special edit format. It is the complete Markdown/plain-text content that should be written to the resolved governed document.

The primary use case is document bootstrap: `create_doc_prompt` creates the target file and `replace_doc` writes the fully authored document without requiring the caller to generate a patch against the placeholder template.

`replace_doc` must keep the same safety boundary as the patch operation:

- The document path is resolved from `doc_type`, `code`, and optional `package`; the caller does not provide a writable path.
- The resolved target must remain under the governed document tree.
- The replacement content must be valid UTF-8 text without a BOM.
- The replacement content must preserve governed front matter identity for the resolved document, including `id`, `type`, `code`, and `slug`.
- The operation returns the resolved path and final content after writing.
- The MCP adapter stays thin and delegates validation and writing rules to `runtime-doc`.

## 3. Alternatives Considered

- **Keep unified diff only:** Rejected because it keeps a low-level serialization format as the only interface for simple document edits.
- **Auto-detect patch format:** Rejected as the primary behavior because ambiguous inputs can hide caller mistakes. The only implicit behavior should be the documented default when `format` is omitted.
- **Use `patch_doc` for bootstrap full-document writes:** Rejected because it forces callers to generate a large patch even when the intended operation is a full replacement of the already resolved governed document.
- **Add a generic `replace_text` tool:** Rejected because the operation should remain governed-document-aware rather than becoming a path-based arbitrary file writer.
- **Keep omitted `format` as unified for backward compatibility:** Rejected for the proposed end state because the preferred agent-facing default should be the less error-prone schema. This creates a migration concern for existing callers that send unified diffs without `format`.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Agent-generated patches become easier to author correctly. | Existing callers that omit `format` while sending unified diffs may need to be updated. |
| Unified diff remains available for source-control-native workflows. | The MCP must maintain two parsing paths and two error surfaces. |
| The default schema aligns with the agent editing workflow already used in this repository. | `apply_patch`-style syntax is less standard outside the agent ecosystem. |
| Format-specific errors can make patch failures faster to diagnose. | The tool input contract becomes slightly larger. |
| Bootstrap flows can replace a newly created document without manufacturing a full-document patch. | A full replacement operation has a larger write blast radius than a targeted patch. |
| `replace_doc` keeps full-document writes inside the governed document resolver instead of exposing path-based writes. | The implementation must validate front matter identity before writing to avoid corrupting governed documents. |

## 5. Acceptance Criteria

- [x] `patch_doc` accepts `format: "unified"` and applies a valid unified diff to the resolved governed document.
- [x] `patch_doc` accepts an omitted `format` and applies a valid `apply_patch`-style patch to the resolved governed document.
- [x] `patch_doc` rejects unknown `format` values with an actionable error that lists supported values.
- [x] `patch_doc` rejects patches whose target file does not match the governed document resolved by `doc_type`, `code`, and optional `package`.
- [x] Unified diff parsing preserves existing behavior when `format: "unified"` is provided.
- [x] `apply_patch`-style parsing supports `Update File` for the resolved document.
- [x] `apply_patch`-style parsing rejects `Add File`, `Delete File`, and `Move to` unless those operations are explicitly added to the governed document contract in a future RFC.
- [x] Error messages identify the active format and the reason parsing or application failed.
- [x] Documentation includes one working example for `format: "unified"` and one working example for omitted `format`.
- [x] The implementation includes tests for both formats, omitted default behavior, unknown formats, and target-file mismatch rejection.
- [x] `replace_doc` exists as a `runtime-doc` operation that resolves the target governed document by `doc_type`, `code`, and optional `package`.
- [x] The MCP server exposes a thin `replace_doc` tool with `root_dir`, `doc_type`, `code`, optional `package`, and `content`.
- [x] `replace_doc` writes the complete replacement content when the governed front matter identity matches the resolved document.
- [x] `replace_doc` rejects content with missing or mismatched governed identity fields: `id`, `type`, `code`, or `slug`.
- [x] `replace_doc` rejects BOM content and any target outside the governed document tree.
- [x] `replace_doc` returns the resolved path and final content after a successful write.
- [x] Documentation includes one working `replace_doc` bootstrap example.
- [x] The implementation includes runtime and MCP tests for successful replacement, missing document, mismatched identity, BOM rejection, and exposed tool parameter handling.

## 6. Open Questions

- Should `patch_doc` keep the existing `git_diff` field as a deprecated alias for `patch` when `format: "unified"` is provided?
- Should there be a short transition period where omitted `format` attempts `apply_patch` first and then unified diff, or should the new default be strict from the first release?
- Should the `apply_patch`-style schema eventually support governed document moves, or should `patch_doc` remain update-only?
- Should `replace_doc` run full document validation before writing, or should it only enforce identity and leave repository-wide validation to a follow-up `validate_fix` call?
