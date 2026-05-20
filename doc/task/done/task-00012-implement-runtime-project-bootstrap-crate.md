---
id: task-00012-implement-runtime-project-bootstrap-crate
type: task
code: "00012"
slug: implement-runtime-project-bootstrap-crate
title: Implement runtime project bootstrap crate
description: Create the runtime/project crate with the create_project plugin operation that provisions the governed project skeleton for new repositories.
status: done
created: 2026-05-05
updated: 2026-05-05
tags:
  - runtime
  - project
  - bootstrap
  - plugin
related:
  - rfc-00012-runtime-project-bootstrap-crate
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
  - spec-00003-project-documentation-folder
supersedes: []
superseded_by: null
---

# Task 00012: Implement runtime project bootstrap crate

## 1. Prime Directive

Create the `runtime/project` crate with the `create_project` plugin operation that provisions the governed project skeleton. Do not introduce MCP SDK dependencies, transport logic, Git initialization policy, or `.obsidian` bootstrap.

## 2. Specs

- **Crate:** `runtime/project`
- **Package name:** `runtime-project`
- **Dependencies:** Rust `std`, approved `runtime/*` contracts — no MCP SDK types
- **Asset loading:** all bootstrap assets must be embedded at compile time using `include_str!` or `include_bytes!` — no runtime file reads from the host filesystem for asset content
- **Asset location:** all asset files live under `runtime/project/assets/`, sibling to `src/`, and are referenced with paths relative to the source file (e.g. `include_str!("../assets/CLAUDE.md")`)

## 3. Checklist

### 3.1. Phase A - Workspace integration and crate bootstrap

- [x] Add the `runtime/project` crate to the workspace
- [x] Set the package name to `runtime-project`
- [x] Define the initial public module layout for the project bootstrap plugin operation
- [x] Wire the crate manifest with only the required runtime contracts — no MCP SDK dependency
- [x] Add tests covering crate bootstrap and public API visibility expectations
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - IO primitives for directory and file creation

- [x] Add `create_dir_all(path: &IoPath)` to `runtime/io` using `tokio::fs::create_dir_all`
- [x] Update `FileWriter::create` to accept `&IoPath` and ensure the parent directory exists before creating the file
- [x] Update `write_file_bytes` to accept `&IoPath` and ensure the parent directory exists before writing
- [x] Update `read_file_bytes` to accept `&IoPath`
- [x] Add `runtime-io` as a dependency in `runtime/project` `Cargo.toml`
- [x] Add tests covering directory creation and parent-guarantee behavior
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - `create_project` operation skeleton

- [x] Define the `create_project` plugin operation type conforming to the runtime plugin operation contract
- [x] Keep the operation transport-agnostic so MCP, CLI, and future frontends can call it without modification
- [x] Define the asset contract that `create_project` owns (the full list of provisioned files and folders)
- [x] Use `runtime_io::create_dir_all` and `runtime_io::write_file_bytes` for all filesystem writes — never call `tokio::fs` directly from `runtime-project`
- [x] Embed all bootstrap asset content using `include_str!` or `include_bytes!` — no runtime reads from the host filesystem for asset content
- [x] Place all asset files under `runtime/project/assets/`, sibling to `src/`
- [x] Add unit tests covering the operation contract surface
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Asset files

Create the physical asset files under `runtime/project/assets/` — no Rust code changes in this phase.

- [x] Create `assets/doc/` tree matching the governed documentation structure
- [x] Create `assets/.vector/document-types.yaml` with `ai-rule`, `prompts`, `template`, and `spec` — without `filename_pattern`
- [x] Create `assets/doc/template/project/template-00002-spec.md` as the spec template file
- [x] Create `assets/doc/template/ai/template-00001-ai-rule.md` as the ai-rule template file
- [x] Create `assets/doc/template/prompts/template-00003-prompts.md` as the prompts template file
- [x] Create `assets/CLAUDE.md`, `assets/AGENTS.md`, `assets/GEMINI.md`
- [x] Create `assets/.editorconfig`, `assets/.gitattributes`, `assets/.gitignore`
- [x] Create `assets/.mcp.json` using `vector` as the command value
- [x] Create `assets/.codex/config.toml` using `vector` as the command value
- [x] Create `assets/.vscode/settings.json` hiding `.vector/`
- [x] Create empty placeholder files for `assets/.claude/`, `assets/.codex/`, `assets/.agents/` folders

### 3.5. Phase D2 - AI rule assets

Create the governed ai-rule documents under `runtime/project/assets/doc/ai-rule/active/` — no Rust code changes in this phase.

- [x] Create `assets/doc/ai-rule/active/ai-rule-00000-master-dispatcher.md` following the ai-rule template, trigger `always_on`
- [x] Create `assets/doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md` following the ai-rule template, trigger `manual`
- [x] Create `assets/doc/ai-rule/active/ai-rule-00002-english-communication.md` following the ai-rule template, trigger `always_on`
- [x] Verify each file conforms to the frontmatter contract defined in `assets/doc/template/ai/template-00001-ai-rule.md`

### 3.7. Phase E - Wire assets into the operation

Update `create_project` to embed and provision the assets created in phases D and D2 — no new asset content in this phase.

- [x] Embed each asset file using `include_str!` or `include_bytes!` from `runtime/project/assets/`
- [x] Wire `create_project` to write all embedded assets to the target path using `runtime_io::create_dir_all` and `runtime_io::write_file_bytes`
- [x] Ensure the three ai-rule documents are written to `doc/ai-rule/active/`
- [x] Ensure `.obsidian/` is never created
- [x] Add tests covering that all expected files are provisioned at the correct paths
- [x] execute section "4. Quality Gate"

### 3.8. Phase F - Skip-existing policy
 
- [x] `create_project` must skip any file that already exists at the target path — never overwrite
- [x] The operation must not fail when a file is skipped — continue provisioning the remaining assets
- [x] Skipped files must be reported in the operation result so the caller knows which assets were not written
- [x] Add tests covering: all files written on a clean target, existing files skipped without error, result reports skipped paths
- [x] execute section "4. Quality Gate"

### 3.9. Phase G - Documentation and public integration

- [x] Add README documentation for `runtime-project` ownership, dependency boundary, and reuse contract
- [x] Verify the public API introduces no MCP SDK types, no transport logic, and no Git initialization policy
- [x] Align package docs with [[rfc-00012-runtime-project-bootstrap-crate]]
- [x] execute section "4. Quality Gate"

### 3.10. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] The workspace includes `runtime/project` with package name `runtime-project`
- [x] `create_project` is exposed as a reusable plugin operation
- [x] `create_project` provisions the governed `doc/` structure
- [x] `.vector/document-types.yaml` includes `ai-rule`, `prompts`, `template`, and `spec`
- [x] `filename_pattern` is not present in `.vector/document-types.yaml`
- [x] Governed file names always use `{type}-{code}-{slug}.md`
- [x] Each governed folder has an associated template reference in configuration
- [x] Physical template files exist under `doc/template/project/template-00002-spec.md`, `doc/template/ai/template-00001-ai-rule.md`, and `doc/template/prompts/template-00003-prompts.md`
- [x] The `prompts` document type is category-based and includes `doc_types`
- [x] `create_project` does not create `.obsidian/`
- [x] `create_project` provisions `doc/ai-rule/active/` with the master dispatcher, staff expertise, and English communication rules
- [x] All three ai-rule files conform to the ai-rule template frontmatter contract
- [x] `create_project` provisions `CLAUDE.md`, `AGENTS.md`, and `GEMINI.md`
- [x] `create_project` provisions `.claude/`, `.codex/`, and `.agents/`
- [x] `create_project` provisions `.vscode/` with the current repository content
- [x] `.vscode/settings.json` hides `.vector/`
- [x] `create_project` provisions `.editorconfig`, `.gitattributes`, `.gitignore`, and `.mcp.json`
- [x] `.codex/config.toml` uses `vector` as its command value
- [x] `.mcp.json` uses `vector` as its command value
- [x] `runtime-project` introduces no MCP SDK dependency
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
