---
id: rfc-00035-incremental-validation-index-phase-2
type: rfc
code: "00035"
slug: incremental-validation-index-phase-2
title: Incremental Validation Index Phase 2
description: Add a rebuildable validation index so frequent validate_fix runs only reprocess changed governed Markdown files.
status: draft
created: 2026-06-10
updated: 2026-06-10
authors:
  - Heli Jerez
tags:
  - validation
  - performance
  - database
  - markdown
related:
  - task-00062-fix-patch-doc-or-find-doc
  - rfc-00039-phase-7-incremental-indexing
supersedes: []
superseded_by: null
aliases:
  - "RFC 00035: Incremental Validation Index Phase 2"
---

# RFC 00035: Incremental Validation Index Phase 2

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: rfc-00035-incremental-validation-index-phase-2
```

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00035-incremental-validation-index-phase-2`
  document-type: task
  document-name: implement-rfc-00035-incremental-validation-index-phase-2
```

## 1. Problem

`validate_fix` is expected to run every time a governed Markdown file changes. The current validation model walks and reads governed Markdown files directly, which is simple and correct, but it does unnecessary repeated work when only one file changed.

This becomes more expensive as the workspace grows to thousands of documents and synchronized packages under `.vector-database/packages/`. The new bare document stem validation proposed in [[task-00062-fix-patch-doc-or-find-doc]] also benefits from a fast document stem index, because each file must know which local and package-qualified stems are valid.

The missing piece is an incremental, rebuildable validation index that lets `validate_fix` reuse previous scan results while preserving filesystem truth and deterministic validation behavior.

## 2. Proposal

Add a Phase 2 validation index stored under `.vector-database/validation/`.

The index is a local cache owned by Vector. It is never the source of truth. If the index is missing, stale, corrupt, or created by an incompatible validation version, Vector rebuilds it from the filesystem.

### 2.1. Storage Path

Use this directory:

```text
.vector-database/validation/
```

The initial implementation may use either:

- `index.json`, if the dataset remains small and single-process access is enough.
- `index.sqlite`, if the implementation needs queryable tables, safer incremental updates, or future backlink queries.

The path must not use `.vector_database` because this repository already uses `.vector-database` for package and RAG storage.

### 2.2. Indexed Records

The index stores one record per governed Markdown file:

- `path`
- `package`
- `document_type`
- `code`
- `slug`
- `stem`
- `mtime_ns`
- `size_bytes`
- `content_hash`
- `validation_version`
- `document_types_config_hash`
- `last_validated_at`
- `bare_stem_errors`
- `wikilink_targets`

The index also stores a stem lookup set derived from current workspace documents and synchronized package documents:

- local stem, for example `task-00062-fix-patch-doc-or-find-doc`
- package-qualified stem, for example `package-name/task-00062-fix-patch-doc-or-find-doc`
- resolved source path

### 2.3. Incremental Validation Flow

`validate_fix` performs this sequence:

1. Load `.vector/document-types.yaml` and compute a config hash.
2. Load the validation index if it exists and matches the current schema and config hash.
3. Walk governed Markdown files in `doc/` and `.vector-database/packages/*/doc/`.
4. For each file, compare `mtime_ns` and `size_bytes` against the indexed record.
5. If both match, reuse the indexed hash and validation result.
6. If either differs or the record is missing, read the file, verify UTF-8, compute `content_hash`, extract the stem, and run validation for that file.
7. Rebuild the global stem lookup from current indexed records.
8. Run bare document stem validation against changed files using the global stem lookup.
9. Apply safe `validate_fix` rewrites only to changed files or files with known fixable indexed errors.
10. Persist the updated index atomically.

`validate` may use the same index for speed, but it must expose a full-rescan mode for CI and debugging.

### 2.4. Correctness Rules

- The filesystem remains authoritative.
- The index is discardable and can always be rebuilt.
- `mtime_ns + size_bytes` is only a fast path. When either changes, Vector must read the file and recompute `content_hash`.
- A changed `.vector/document-types.yaml` invalidates validation records that depend on document type configuration.
- A changed validation algorithm version invalidates affected records.
- Index writes must be atomic to avoid corrupting the cache when `validate_fix` is interrupted.
- Invalid UTF-8 files are recorded as validation errors but are not rewritten.
- Files rewritten by `validate_fix` must be rehashed and revalidated before the index is persisted.

### 2.5. Bare Stem Wikilink Validation

The Phase 2 index directly supports bare stem validation:

- Build a `HashSet` of valid local and package-qualified stems from the index.
- Scan Markdown body content only, excluding frontmatter.
- Extract candidates matching `[<package>/]<doc-type>-<code>-<slug>`.
- Confirm candidates against the stem `HashSet`.
- Report bare stems from `validate`.
- Rewrite confirmed bare stems to wikilinks from `validate_fix`.

This keeps validation approximately linear in changed file content rather than multiplying files by known stems.

## 3. Alternatives Considered

- **Always full-scan all governed Markdown files:** Discarded for Phase 2 because `validate_fix` is expected to run on every file change. Full scans are simple, but they waste IO and parsing work when only one file changed.
- **Use modified time only:** Discarded because filesystem timestamps can be coarse, preserved by copy tools, or affected by clock behavior. `mtime_ns + size_bytes` is acceptable as a fast path, but content hashes remain the correctness boundary after a detected change.
- **Make the database authoritative:** Discarded because validation must reflect the repository on disk. A cache that can block or distort validation after corruption would make the system harder to reason about.
- **Use one database for validation and RAG data immediately:** Discarded for this phase because validation needs a small, deterministic cache. RAG storage has different query and embedding lifecycle requirements.

## 4. Tradeoffs

| Pro                                                                                                     | Con                                                                                                  |
|---------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------|
| Repeated `validate_fix` runs become proportional to changed files instead of all files.                 | Adds cache invalidation logic and schema versioning.                                                 |
| Bare document stem validation can use a direct `HashSet` lookup instead of expensive repeated searches. | The implementation must keep the global stem set consistent after moves, deletes, and package syncs. |
| The cache improves editor workflows where validation runs after each save.                              | Atomic writes and corruption recovery need explicit tests.                                           |
| Keeping the index discardable preserves correctness.                                                    | Rebuilds still pay the full scan cost when the cache is missing or invalidated.                      |

## 5. Acceptance Criteria

- [ ] Vector stores validation cache data under `.vector-database/validation/`.
- [ ] The validation index stores path, package, stem, modified time, size, content hash, validation version, and document type config hash.
- [ ] `validate_fix` reuses cached records for unchanged files.
- [ ] `validate_fix` reads, hashes, validates, fixes, rehashes, and reindexes changed files.
- [ ] `validate_fix` persists index updates atomically.
- [ ] If the index is missing, corrupt, or version-incompatible, Vector rebuilds it from the filesystem.
- [ ] Bare document stem validation uses a stem lookup set derived from indexed local and package documents.
- [ ] `validate` supports a full-rescan mode that ignores the index.
- [ ] Tests cover unchanged-file reuse, changed-file revalidation, file deletion, file move, package-qualified stems, config hash invalidation, schema version invalidation, interrupted write recovery, and corrupt index rebuild.

## 6. Open Questions

- Should the first implementation use JSON for simplicity or SQLite for future queryability?
- Should synchronized package files be indexed during package sync, during validation, or both?
- Should the index store backlink data now, or defer backlinks until a separate navigation or graph RFC?
- Should `validate` default to indexed mode locally and full-rescan mode in CI?
- [[rfc-00039-phase-7-incremental-indexing]] defers `IndexResult` persistence to this RFC. Should the validation index infrastructure under `.vector-database/validation/` also cover RAG indexing run results (counts of skipped, re-indexed, deleted, and failed documents), or should RAG run-result tracking use a separate store under `.vector-database/rag/`?
