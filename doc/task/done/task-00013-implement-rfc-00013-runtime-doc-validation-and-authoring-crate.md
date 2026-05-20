---
id: task-00013-implement-rfc-00013-runtime-doc-validation-and-authoring-crate
type: task
code: "00013"
slug: implement-rfc-00013-runtime-doc-validation-and-authoring-crate
title: "Implement RFC 00013: Runtime Doc Validation and Authoring Crate"
description: Create the runtime-doc crate that centralizes documentation validation and authoring as reusable plugin operations.
status: done
created: 2026-05-05
updated: 2026-05-06
tags:
  - runtime
  - documentation
  - validation
  - authoring
related:
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00012-runtime-project-bootstrap-crate
supersedes: []
superseded_by: null
---

# Task 00013: Implement RFC 00013: Runtime Doc Validation and Authoring Crate

## 1. Prime Directive

No single crate owns documentation governance for the `doc/` tree. Validation rules, code allocation, template lookup, and document type scaffolding are scattered across ad hoc tools. This task eliminates that friction by delivering `runtime-doc` as the canonical, transport-agnostic owner of all documentation governance behavior.

## 2. Specs

- **Module:** `runtime/doc` (package name `runtime-doc`)
- **Dependencies:**
  - `runtime-core` (plugin primitives)
  - `serde`, `serde_yaml` (configuration parsing)
  - no MCP SDK dependency

## 3. Checklist

### 3.1. Phase A — Crate Bootstrap

- [x] Add `runtime/doc` crate to the workspace (`Cargo.toml`)
- [x] Set package name to `runtime-doc`
- [x] Verify the crate compiles as an empty library
- [x] Execute section "4. Quality Gate"

### 3.2. Phase B — Configuration Model and Loader

- [x] Define `DocumentTypesConfig` model matching the `.vector/document-types.yaml` schema
- [x] Implement loader that reads and deserializes `.vector/document-types.yaml` from a given root path
- [x] Return a typed error when the file is missing or malformed
- [x] Tests covering Phase B
- [x] Execute section "4. Quality Gate"

> **Note:** Phase B will be extended in Phase K (optional `tags` field) and Phase L (optional `prompt` field) as those operations are implemented.

### 3.3. Phase C — `validate` Operation

- [x] Implement `validate`: check `doc/` layout against the loaded `DocumentTypesConfig`
- [x] Verify `.vector/document-types.yaml` exists and is internally consistent (reuse Phase B loader)
- [x] Check every governed file is UTF-8 without BOM
- [x] Check minimum frontmatter fields: `id`, `type`, `code`, `slug`, `title`, `description`, `created`, `tags`
- [x] Check `status` is present for status-based document types
- [x] Check `category` is present for category-based document types
- [x] Check containing folder matches `status` frontmatter for status-based types
- [x] Check containing folder matches `category` frontmatter for category-based types
- [x] Check governed file names follow `{type}-{code}-{slug}.md`
- [x] Check wikilinks use only target file names without `.md` extension
- [x] Allow extra frontmatter fields beyond the minimum contract
- [x] Tests covering Phase C
- [x] Execute section "4. Quality Gate"

### 3.4. Phase D — `validate --fix` Mode

- [x] Implement optional `fix` mode on `validate`
- [x] `fix` moves files into the correct status or category folder when placement is wrong
- [x] `fix` normalizes markdown structure for governed files
- [x] `fix` normalizes wikilinks to use file names without `.md`
- [x] `fix` applies safe frontmatter repairs (does not invent missing semantic content)
- [x] `fix` rejects `filename_pattern` as a user-configurable field
- [x] Tests covering Phase D
- [x] Execute section "4. Quality Gate"

### 3.5. Phase E — Next Code Resolver

- [x] Implement `next_code_for(doc-type, root_dir)` as an internal utility function
- [x] Resolve the folder tree for the given document type via the Phase B loader
- [x] Scan all `.md` files under `doc/{type}/` recursively (all subfolders — status and category alike)
- [x] Parse the numeric code from each file name using the `{type}-{code}-{slug}.md` pattern enforced by `validate`
- [x] Select the highest existing code and return `highest + 1`
- [x] Return `1` when no files exist yet for that type
- [x] Return a typed error when a file name does not match the expected pattern (indicates a validation gap)
- [x] This function is internal — not exposed as a plugin operation
- [x] Tests covering Phase E
- [x] Execute section "4. Quality Gate"

### 3.6. Phase F — Slug Validator

- [x] Implement internal `validate_slug(slug: &str) -> Result<(), SlugError>`
- [x] Accept only lowercase ASCII letters (`a`–`z`), ASCII digits (`0`–`9`), and hyphens (`-`)
- [x] Reject slugs that start or end with a hyphen
- [x] Reject slugs that contain consecutive hyphens
- [x] Reject empty slugs
- [x] Extend `validate` (Phase C) to check the `slug` frontmatter field of every governed file using `validate_slug`
- [x] Tests covering Phase F
- [x] Execute section "4. Quality Gate"

### 3.7. Phase G — `bootstrap_doc` Operation

- [x] Implement `bootstrap_doc` accepting document type and slug
- [x] Validate the provided slug using the Phase F `validate_slug` function before any other step
- [x] Resolve the target document type via the Phase B loader
- [x] Compute the next available code using the Phase E resolver
- [x] Derive the target path from the document type layout
- [x] Create the file under `doc/` using the configured template when it exists
- [x] Fall back to a frontmatter-only template shape when the configured template does not exist
- [x] Enforce `{type}-{code}-{slug}.md` naming invariant
- [x] Tests covering Phase G
- [x] Execute section "4. Quality Gate"

### 3.8. Phase H — `bootstrap_doc_type` Operation

- [x] Implement `bootstrap_doc_type` requiring an explicit layout choice (status-based or category-based)
- [x] Require allowed status values for status-based types
- [x] Require allowed category values for category-based types
- [x] Create the document folder structure for the new type
- [x] Update `.vector/document-types.yaml` via the Phase B loader (read → mutate → write)
- [x] Create a template for the new type under `doc/template/doc/`
- [x] Tests covering Phase H
- [x] Execute section "4. Quality Gate"

### 3.9. Phase I — File Locator by Stem

- [x] Implement internal `locate_file_by_stem(stem: &str, root_dir: &Path) -> Result<PathBuf, LocateError>`
- [x] Parse the document type from the stem using the `{type}-{code}-{slug}` naming pattern
- [x] Resolve the folder tree for that document type via the Phase B loader
- [x] Scan all subfolders of `doc/{type}/` recursively for a file whose name without extension matches the given stem
- [x] Return the absolute path of the matching file
- [x] Return a typed error when no file matches
- [x] This function is internal — not exposed as a plugin operation
- [x] Tests covering Phase I
- [x] Execute section "4. Quality Gate"

### 3.10. Phase J — `find_doc` Operation

- [x] Implement `find_doc` accepting a document type identifier and a numeric code
- [x] Construct the stem prefix `{type}-{code}-` and use the Phase I locator to find the file for any slug
- [x] Return the absolute path of the matching file
- [x] Return a typed error when no file matches the given type and code
- [x] `find_doc` must not load or parse file content
- [x] Tests covering Phase J
- [x] Execute section "4. Quality Gate"

### 3.11. Phase K — `tags` field and `get_doc_types_tags` Operation

- [x] Add optional `tags` field to the `DocumentTypesConfig` model (extends Phase B)
- [x] Loader deserializes `tags` as an optional list of strings when present in `.vector/document-types.yaml`
- [x] Implement `get_doc_types_tags`: collect tags from all document type entries, deduplicate, sort alphabetically, return as comma-separated string
- [x] Return empty string when no document type declares any tags
- [x] Skip document types with no `tags` field without error
- [x] Tests covering Phase K
- [x] Execute section "4. Quality Gate"

### 3.12. Phase L — Prompt field in `DocumentTypesConfig`

- [x] Add optional `prompt` field to the `DocumentTypesConfig` model (extends Phase B)
- [x] Loader deserializes `prompt` as an optional path string when present in `.vector/document-types.yaml`
- [x] Tests covering Phase L
- [x] Execute section "4. Quality Gate"

### 3.13. Phase M — `create_doc` Operation

- [x] Implement `create_doc` accepting: document type, optional category, name, and slug
- [x] Validate the slug using the Phase F `validate_slug` function before any other step
- [x] Resolve the document type from config via the Phase B loader
- [x] Resolve the `prompt` field for the type — return a typed error if absent
- [x] Resolve the `template` field for the type from config
- [x] Compute the next available code using the Phase E resolver
- [x] Derive the target file path from the layout, the computed code, and the slug
- [x] Return a typed error when no template is configured for the type
- [x] Create the document file using the configured template
- [x] Load the prompt file content using the Phase I locator by stem
- [x] Replace `#{doc-type}` in the prompt with the document type identifier
- [x] Replace `#{code}` in the prompt with the computed code zero-padded to `code-width`
- [x] Replace `#{slug}` in the prompt with the validated slug
- [x] Replace `#{file-path}` in the prompt with the absolute path of the created file
- [x] Leave unrecognized placeholders in the prompt unchanged
- [x] Return the resolved prompt string to the caller
- [x] Tests covering Phase M
- [x] Execute section "4. Quality Gate"

### 3.14. Phase N — Bootstrap Assets for Document Type Creation

- [x] Add `doc/template/project/template-00004-doc-type-template.md` to `runtime/project/assets` — frontmatter-only template used as the base document template for any new type created by `bootstrap_doc_type`
- [x] Add `doc/template/project/template-00005-doc-type-prompt.md` to `runtime/project/assets` — frontmatter-only template used as the base prompt template for any new type created by `create_doc_type`
- [x] Add `doc/prompts/doc-type/prompts-00001-create-doc-type.md` to `runtime/project/assets` — the governed prompt document that `create_doc_type` loads; body contains the authoring instructions with `#{doc-type}` and `#{layout}` placeholders
- [x] Update `runtime/project/assets/.vector/document-types.yaml` to add top-level fields `doc-type.template`, `doc-type.prompt-template`, and `doc-type.prompt` pointing to the three files above
- [x] Verify the `prompts` document type in the assets config includes `doc-type` as an allowed category
- [x] Tests covering Phase N: `create_project` provisions all three new asset files in a fresh project
- [x] Execute section "4. Quality Gate"

### 3.15. Phase O — `create_doc_type` Operation

- [x] Implement `create_doc_type` accepting: document type name, layout, allowed statuses or categories, code width, and optional template name
- [x] Validate the document type name against the slug contract before any other step
- [x] Resolve the prompt declared for document type creation from the top-level `doc-type.prompt` field in `.vector/document-types.yaml` — return a typed error when absent
- [x] Execute `bootstrap_doc_type` internally (Phase H) to create folders, update config, and create the default document template (using the Phase N `doc-type.template` asset as base)
- [x] Create a prompt template file for the new type under `doc/template/doc/` using the Phase N `doc-type.prompt-template` asset as base — frontmatter only
- [x] Update `.vector/document-types.yaml` to set the `prompt` field for the new type pointing to the created prompt template
- [x] Load the `doc-type.prompt` file content using the Phase I locator by stem
- [x] Replace `#{doc-type}` in the prompt with the new document type name
- [x] Replace `#{layout}` in the prompt with the chosen layout
- [x] Leave unrecognized placeholders in the prompt unchanged
- [x] Return the resolved prompt string to the caller
- [x] Tests covering Phase O
- [x] Execute section "4. Quality Gate"

### 3.16. Phase Z — Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update `runtime/doc/README.md`
- [x] Confirm no MCP SDK dependency introduced

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] All acceptance criteria from RFC 00013 section 5 are satisfied
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
