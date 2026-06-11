---
id: task-00062-fix-patch-doc-or-find-doc
type: task
code: "00062"
slug: fix-patch-doc-or-find-doc
title: Fix patch_doc Rejections and Markdown Hygiene
description: Make governed document patching reliable and enforce canonical Markdown hygiene.
status: in-progress
created: 2026-06-10
updated: 2026-06-11
tags:
  - mcp
  - patching
  - governance
related: []
supersedes: []
superseded_by: null
---

# Task 00062: Fix patch_doc Rejections and Markdown Hygiene

## 1. Prime Directive

> [!Prime Directive]
> Agents cannot reliably update governed documents through `patch_doc` because valid-looking patches can be rejected, likely due to line-ending, trailing-newline, or context-offset mismatches between the content returned by `find_doc` and the content consumed by the patch engine.

The structural friction to eliminate is the mismatch between the document content agents inspect and the document content `patch_doc` validates. The fix should make the patch path deterministic, observable, and covered by regression tests.

The same work should also make governed Markdown hygiene explicit: Markdown files must be UTF-8, must not contain a UTF-8 BOM, must use LF line endings, and must use wikilinks for governed document references.

## 2. Specs

- **Module:** vector MCP document lookup and patching tools
- **Dependencies:** none

### 2.1. Decision Context

The proposed idea is to change `find_doc` so it returns only the package and path, requiring the agent to read the file directly before calling `patch_doc`.

That change may reduce one class of content mismatch when the target document is local and readable from the agent environment. It also weakens the `find_doc` contract, removes useful context from a lookup tool, and may break package-qualified or remote-like document flows where the caller cannot safely assume direct filesystem access.

The recommended path is to keep `find_doc` content-compatible and fix the authoritative mutation path in `patch_doc`.

### 2.2. Target Behavior

- `find_doc` continues returning path, package, and content for lookup workflows.
- `patch_doc` applies patches against the same canonical document bytes or normalized text representation used to produce lookup content.
- `patch_doc` accepts patches that differ only by safe newline details, including final newline presence, CRLF versus LF normalization, and common unified-diff formatting differences.
- `patch_doc` rejects genuinely invalid patches with diagnostics that identify the failing hunk, expected context, observed context, and newline mode.
- `validate` fails governed Markdown files that contain CRLF line endings.
- `validate_fix` converts governed Markdown files from CRLF to LF and reports the fix, including template documents.
- `validate` fails governed Markdown files that contain a UTF-8 BOM.
- `validate_fix` removes a UTF-8 BOM from governed Markdown files and reports the fix, including template documents.
- `validate` and `validate_fix` fail governed Markdown files whose bytes are not valid UTF-8; invalid UTF-8 must not be silently rewritten because there is no lossless automatic repair.
- `validate` fails governed Markdown body content that contains a bare governed document stem in the form `[<package>/]<doc-type>-<code>-<slug>` outside frontmatter.
- `validate_fix` rewrites bare governed document stems outside frontmatter to the same stem wrapped as a wikilink, for example `[[task-00062-fix-patch-doc-or-find-doc]]` or `[[package-name/task-00062-fix-patch-doc-or-find-doc]]`.
- Frontmatter values are excluded from bare-stem wikilink enforcement because governed identifiers there are metadata, not prose references.
- Tests cover the failure mode before implementation and the accepted behavior after implementation.

Use [[prompts-00004-execute-task-phase]] for phase execution.

## 3. Checklist

### 3.1. Phase A - Reproduce and Localize

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase A
  language: Rust, Markdown
```

- [x] Add or identify a failing regression case where `patch_doc` rejects a patch generated from `find_doc` content.
- [x] Confirm whether the rejection is caused by trailing newline handling, CRLF normalization, hunk context mismatch, offset drift, or another parser issue.
- [x] Document the exact failing path in the test name or assertion message.

### 3.2. Phase B - Stabilize patch_doc

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase B
  language: Rust, Markdown
```

- [ ] Make `patch_doc` patch against a canonical representation that preserves enough information to write the final file correctly.
- [ ] Normalize only newline differences that are safe and reversible for governed Markdown documents.
- [ ] Preserve existing frontmatter and validation semantics.
- [ ] Improve rejection diagnostics for failed hunks.
- [ ] Keep `find_doc` backward-compatible unless implementation evidence proves the contract itself is the root cause.

### 3.3. Phase C - Enforce Markdown Encoding and Line Endings

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase C
  language: Rust, Markdown
```

- [ ] Add a validation check that rejects CRLF line endings in all governed Markdown files, including templates.
- [ ] Add a `validate_fix` repair that rewrites all governed Markdown files to LF line endings, including templates.
- [ ] Preserve a final newline when present and avoid introducing unrelated content changes during LF normalization.
- [ ] Confirm `validate` rejects UTF-8 BOM files and invalid UTF-8 files.
- [ ] Confirm `validate_fix` removes UTF-8 BOM files, including templates, and still fails invalid UTF-8 files without rewriting them.
- [ ] Add flow-level tests proving `validate` fails and `validate_fix` repairs BOM and CRLF cases where repair is safe.
- [ ] Add tests proving invalid UTF-8 is reported by both `validate` and `validate_fix`.

### 3.4. Phase D - Normalize Governed Document References

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase D
  language: Rust, Markdown
```

- [ ] Build a document stem index from configured document types and known governed document files.
- [ ] Detect bare stems in Markdown body content only, excluding frontmatter.
- [ ] Support both local stems like `task-00062-fix-patch-doc-or-find-doc` and package-qualified stems like `package-name/task-00062-fix-patch-doc-or-find-doc`.
- [ ] Make `validate` report bare governed document stems with the expected wikilink replacement.
- [ ] Make `validate_fix` rewrite bare governed document stems to wikilinks without touching already-correct wikilinks.
- [ ] Avoid rewriting filenames, URLs, fenced code blocks, or inline code unless there is an explicit product decision to enforce references inside code spans too.
- [ ] Add tests for frontmatter exclusion, body rewrite, already-valid wikilinks, package-qualified stems, and false-positive avoidance.

### 3.5. Phase E - Evaluate Optional API Refinement

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase E
  language: Rust, Markdown
```

- [ ] Decide whether `find_doc` should add an optional path-only or metadata-only mode instead of removing content from the default response.
- [ ] If a new mode is added, keep the default response compatible for existing agents.
- [ ] Add tests for both default lookup and any new optional lookup mode.

### 3.6. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00062
  phase: Phase Z
  language: Rust, Markdown
```

- [ ] Run the relevant unit and integration tests.
- [ ] Run governed document validation.
- [ ] Update README files only if user-facing tool behavior changes.
