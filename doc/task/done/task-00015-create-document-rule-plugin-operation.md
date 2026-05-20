---
id: task-00015-create-document-rule-plugin-operation
type: task
code: "00015"
slug: create-document-rule-plugin-operation
title: Create Document Rule Plugin Operation
description: Introduce the create_document_rule plugin operation and realign document-types config so ai-rule-00002-documentation.md is regenerated from template-00006-documentation whenever a doc type is created.
status: done
created: 2026-05-06
updated: 2026-05-06
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00015: Create Document Rule Plugin Operation

## 1. Prime Directive

Every time a new document type is bootstrapped, the AI rule that documents supported
types at `doc/ai-rule/active/ai-rule-00002-documentation.md` must be regenerated
automatically so agents always see an accurate, up-to-date type catalogue.

This task also realigns the document-types config model with the accepted bootstrap
asset and operation flow:

- `doc-type` stays at the YAML root and must be modeled explicitly
- `filename_pattern` must not exist in config
- `description` and `tags` are supported per document type
- `aliases` must not be used by later phases of this task
- `categories` must not be modeled in config because category folders are derived
  dynamically from doc-type subfolders
- every document type must define a `prompt` field, including the `prompts` document type

## 2. Specs

- **Crates touched:** `runtime-doc`, `runtime-project`
- **Config file:** `runtime/project/assets/.vector/document-types.yaml`
- **Template asset:** `runtime/project/assets/doc/template/ai/template-00006-documentation.md`
- **Generated rule:** `doc/ai-rule/active/ai-rule-00002-documentation.md`
- **Dependencies:** existing `BootstrapDocTypeOp`, `runtime_io`, `runtime_core`

## 3. Checklist

### 3.1. Phase A - Realign config model with the bootstrap asset

- [x] Model the root-level `doc-type` block explicitly in the config crate instead of assuming it lives under `document_types`.
- [x] Update the loaders and call sites so `create_doc_type` and `bootstrap_doc_type` read the root-level `doc-type` block from config.
- [x] Remove any remaining config support for `filename_pattern`; the filename shape is fixed by governance and must not be configurable.
- [x] Ensure `DocumentTypeConfig` supports `description` and `tags`.
- [x] Ensure later phases of this task use `tags` only and do not depend on `aliases`.
- [x] Remove config-level `categories` support; category folders must be derived dynamically from the document-type subfolders instead of persisted in YAML.
- [x] Ensure every document type supports a `prompt` field, including the `prompts` document type.
- [x] Tests: add or update config and loader tests to prove:
   - root-level `doc-type` is loaded correctly
  - `filename_pattern` is rejected or unsupported
  - `description`, `tags`, and `prompt` are preserved
  - `categories` are not read from config
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass.

### 3.2. Phase B - Thread the realigned config through doc-type operations

- [x] Update `BootstrapDocTypeInput` to carry:
  - `description: Option<String>` - human-readable purpose of the doc type
  - `tags: Option<Vec<String>>` - searchable labels for the doc type
  - `prompt: Option<String>` when needed by the accepted config flow
- [x] Update `CreateDocTypeInput` with the same config-facing fields and thread them through the `BootstrapDocTypeInput` construction in `create_doc_type.rs`.
- [x] Update `build_doc_type_config` in `bootstrap_doc_type.rs` to emit `description`, `tags`, and `prompt` when present.
- [x] Remove any write path that persists `aliases` or `categories` back into `document-types.yaml`.
- [x] Ensure category-based behavior still works by deriving categories from the existing subfolder layout instead of YAML config.
- [x] Tests: update bootstrap and create doc-type tests so assertions use `tags` and never `aliases`.
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass.

### 3.3. Phase C - create_document_rule operation

Create `runtime/doc/src/operations/create_document_rule.rs`:

- [x] Define `CreateDocumentRuleInput`:
  - `root_dir: IoPath`
  - `output_path: IoPath` - destination file at `doc/ai-rule/active/ai-rule-00002-documentation.md`
  - `template_stem: String` - template stem `"template-00006-documentation"`
- [x] Define `CreateDocumentRuleOutput`:
  - `written_path: IoPath` - the path that was written
- [x] Implement `create_document_rule`:
  1. Load `document-types.yaml` via `load_document_types_config`.
   2. Load the root-level `doc-type` config when needed by the generation flow.
  3. Locate the template file by stem using `locate_file_by_stem`.
  4. Read the template content.
  5. Build the `#{types}` replacement string by iterating over all doc types in deterministic sorted order by name:

     ```text
     **document type:** <name>
     **tags:** <comma-separated list from tags field, or "-" if absent>
     **description:** <description field value, or "-" if absent>
     ```

     Each type is separated by a blank line.
  6. Replace the `#{types}` placeholder with the built string.
  7. Update the frontmatter fields `created` and `updated` to today's date, preserving `created` if the file already exists.
  8. Write the result to `output_path`, creating parent directories if needed.
  9. Send `CreateDocumentRuleOutput`.
- [x] Register with `declare_plugin_operations!`:
  `CreateDocumentRuleOp => create_document_rule(CreateDocumentRuleInput, CreateDocumentRuleOutput)`
- [x] Add `mod create_document_rule;` to `runtime/doc/src/operations/mod.rs`.
- [x] Unit tests in `create_document_rule_test.rs`:
  - happy path: template with `#{types}` is correctly expanded
  - missing `#{types}` placeholder: file is written unchanged
  - `tags` or `description` absent: renders `-`
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass.

### 3.4. Phase D - Wire into bootstrap_doc_type

- [x] After `update_document_types_yaml` succeeds in `bootstrap_doc_type`, invoke `CreateDocumentRuleOp` with:
  - `output_path` = `root_dir/doc/ai-rule/active/ai-rule-00002-documentation.md`
  - `template_stem` = `"template-00006-documentation"`
- [x] Use an internal mock sender with no forwarded output to the caller.
- [x] Integration test: bootstrapping a new doc type rewrites `ai-rule-00002-documentation.md` and the new type appears in the `#{types}` block.
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass.

### 3.5. Phase Z - Wrap-up

- [x] Regenerate `doc/ai-rule/active/ai-rule-00002-documentation.md` by running the operation manually or through an equivalent command so the live file reflects all current doc types and their `description` and `tags`.
- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files for `runtime-doc` and `runtime-project` if modified.

### 3.6. Phase Y - Replace hardcoded rule dates

- [x] Replace the hardcoded date in `create_document_rule.rs` with the real execution date used at runtime.
- [x] Update or extend tests so date handling proves:
  - `updated` reflects the execution date
  - `created` is preserved when the destination file already exists
- [x] Validation vector: `xtask quality-lint` + `xtask quality-test` pass.

### 3.7. Phase X - Reconcile task completion with real behavior

- [x] Review the completed task contract against the implemented crates and remove any stale acceptance claims that no longer describe the real behavior.
- [x] Documentation-rule regeneration from `bootstrap_doc_type` is mandatory, not best-effort.
- [x] Stop swallowing `CreateDocumentRuleOp` failures in `bootstrap_doc_type` so doc-type bootstrap fails if rule regeneration fails.
- [x] Ensure the task contract, validation vector, and implementation all reflect mandatory documentation-rule regeneration.
- [x] Every document type must define `prompt` in `document-types.yaml`; preserve that requirement in the task contract and enforce it consistently in implementation and tests.
- [x] Validation vector: `cargo fmt --all --check` + `cargo clippy -p runtime-doc --tests -- -D warnings` + `cargo test -p runtime-doc` pass.

## 4. Quality Gate

- [x] `cargo clippy -p runtime-doc --tests -- -D warnings` passes
- [x] `cargo test -p runtime-doc` passes

## 5. Validation Vector

- [x] All phase checkboxes completed.
- [x] All quality gates pass.
- [x] `doc/ai-rule/active/ai-rule-00002-documentation.md` contains an entry for every doc type defined in `document-types.yaml`.
- [x] Creating a brand-new doc type via `BootstrapDocTypeOp` causes `ai-rule-00002-documentation.md` to be updated with the new type's `name`, `tags`, and `description`.
- [x] `BootstrapDocTypeOp` fails if documentation-rule regeneration fails.
- [x] Every document type defined in `document-types.yaml` carries a `prompt` field.
