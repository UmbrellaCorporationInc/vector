---
id: rfc-00012-runtime-project-bootstrap-crate
type: rfc
code: "00012"
slug: runtime-project-bootstrap-crate
title: Runtime Project Bootstrap Crate
description: Defines the first runtime project crate that provisions governed project bootstrap assets for new repositories.
status: implemented
created: 2026-05-05
updated: 2026-05-05
authors: []
tags:
  - runtime
  - project
  - bootstrap
  - documentation
  - plugin
related:
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - rfc-00011-mcp-vector-rmcp-dependency-and-thin-tooling-boundary
  - spec-00003-project-documentation-folder
supersedes: []
superseded_by: null
aliases:
  - "RFC 00012: Runtime Project Bootstrap Crate"
---

# RFC 00012: Runtime Project Bootstrap Crate

## 1. Problem

The repository already has one governed project shape, but there is no reusable runtime crate that can provision that shape into a new project.

That leaves four concrete gaps:

- bootstrap logic is implicit in the current repository state instead of owned by one reusable runtime capability
- the governed documentation scaffold still has to be assembled manually when starting a new repository
- file naming rules can be exposed as configuration even though the repository contract requires one fixed governed pattern
- MCP and editor bootstrap files still reflect the old command name `vector-bootstrap` instead of the intended command name `vector`

Without one accepted crate boundary, project bootstrap will become a collection of ad hoc file copy routines distributed across tools, and every future frontend will need to rediscover the same repository contract independently.

The project needs one first runtime crate focused only on project bootstrap as a reusable plugin operation.

## 2. Proposal

Create a new crate at `runtime/project` with package name `runtime-project`.

This crate is the first runtime crate that provides project-oriented plugin operations and reusable libraries. Its initial scope is limited to one plugin operation:

- `create_project`

### Crate responsibility

`runtime-project` owns reusable project bootstrap behavior.

Accepted responsibilities:

- define the project bootstrap asset contract
- expose project bootstrap as a runtime plugin operation
- stay transport-agnostic so MCP, CLI, or future frontends can call the same behavior

Non-responsibilities:

- no MCP tool registration
- no editor-specific transport logic
- no Git initialization policy
- no `.obsidian` bootstrap
- no arbitrary file mirroring outside the accepted asset set

### `create_project`

`create_project` provisions the governed project skeleton required by this repository contract.

After successful execution, the target project contains:

- `doc/` with the governed documentation structure used by this repository
- `doc/ai-rule/active/ai-rule-00000-master-dispatcher.md`
- `doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md`
- `doc/ai-rule/active/ai-rule-00002-english-communication.md`
- `.vector/document-types.yaml`
- `CLAUDE.md`
- `AGENTS.md`
- `GEMINI.md`
- `.claude/`
- `.codex/`
- `.agents/`
- `.vscode/`
- `.editorconfig`
- `.gitattributes`
- `.gitignore`
- `.mcp.json`

The operation must not create `.obsidian/`.

### Documentation type bootstrap

`create_project` must provision `.vector/document-types.yaml` with at least these additional document types:

- `ai-rule`
- `prompts`
- `template`
- `spec`

Governed document file names are not configurable.

The file naming pattern is always:

- `{type}-{code}-{slug}.md`

`filename_pattern` must not be stored in `.vector/document-types.yaml` because it is a repository invariant rather than a user-controlled setting.

Each governed folder created by the bootstrap must have an associated template entry in configuration and one physical template file in the template tree that matches the intended category.

Minimum required template placement:

- spec templates live under `doc/template/project/`
- ai-rule templates live under `doc/template/ai/`
- prompts templates live under `doc/template/prompts/`

The bootstrap must also create one category-based `prompts` document type with at least this initial category:

- `doc_types`

This RFC accepts category ownership and placement, but does not force one final file naming convention for any new template that does not already exist in the repository.

### Bootstrap assets copied from the current project contract

`create_project` must provision the same `.vscode` content currently used by this repository.

That includes hiding `.vector/` through `.vscode/settings.json` file exclusion settings.

`create_project` must also provision repository bootstrap files that exist in the current project contract:

- `.editorconfig`
- `.gitattributes`
- `.gitignore`
- `.mcp.json`

For tool bootstrap files, the accepted command names are:

- `.codex/config.toml` uses `vector` as the command
- `.mcp.json` uses `vector` as the command

This RFC explicitly rejects keeping `vector-bootstrap` in newly created projects.

### Agent instruction files

`create_project` provisions agent-facing instruction files:

- `CLAUDE.md`
- `AGENTS.md`
- `GEMINI.md`

It also provisions the supporting folders:

- `.claude/`
- `.codex/`
- `.agents/`

This RFC accepts the repository contract that `GEMINI.md` must exist in newly created projects even though it does not exist in the current repository snapshot on 2026-05-05.

### AI rules bootstrap

`create_project` must provision the following governed ai-rule documents under `doc/ai-rule/active/`:

- `ai-rule-00000-master-dispatcher.md` — trigger: `always_on`
- `ai-rule-00001-staff-engineer-expertise.md` — trigger: `manual`
- `ai-rule-00006-english-communication.md` — trigger: `always_on`

Each provisioned ai-rule file must conform to the ai-rule template defined in the bootstrap asset tree.

The master dispatcher and the English communication rule are always-on. The staff engineer expertise rule is loaded on any technical request as directed by the master dispatcher.

### Conflict policy

`create_project` must skip any asset file that already exists at the target path. It must never overwrite an existing file.

Overwriting would violate the configurability and extensibility principle: a user may have intentionally modified a bootstrapped file, and the operation must preserve that intent.

The operation must not fail when a file is skipped. It must continue provisioning the remaining assets and report all skipped paths in the operation result.

### Reuse boundary

`mcp/vector` and future frontends may expose tools that call `runtime-project`, but they must not own the reusable project bootstrap rules themselves.

The dependency direction remains:

- transports depend on `runtime-project`
- `runtime-project` depends on runtime contracts
- runtime project behavior does not depend on MCP SDK types

## 3. Alternatives Considered

- **Keep project bootstrap only inside `mcp/vector`:** Discarded because it would repeat the same mistake rejected in earlier RFCs by pushing reusable behavior into a transport facade.
- **Keep project bootstrap and documentation governance in the same crate:** Discarded because repository bootstrap and ongoing `doc/` governance have different responsibilities and should evolve independently.
- **Bootstrap `.obsidian` along with the rest of the repository scaffolding:** Discarded because the requested project contract explicitly excludes `.obsidian` and should not couple editor-local state to governed bootstrap.
- **Keep `vector-bootstrap` as the command name for compatibility:** Discarded because the accepted target contract for newly created projects is `vector`, and carrying the old name forward would preserve known drift.
- **Keep `filename_pattern` configurable in `document-types.yaml`:** Discarded because `{type}-{code}-{slug}.md` is a governance invariant and should not be changeable by project users.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| One reusable crate gives every transport one canonical project bootstrap contract. | `runtime-project` becomes the owner of a large file-template surface that must be kept aligned with repository governance changes. |
| Rejecting `.obsidian` keeps the bootstrap focused on governed project assets instead of local editor state. | Users who want an Obsidian-specific setup will need a separate capability or manual setup step. |
| Changing command names to `vector` removes known bootstrap drift at creation time. | Existing repositories that still use `vector-bootstrap` will need an explicit migration path outside this RFC. |
| Making file naming an internal invariant removes a class of configuration drift. | Future support for alternative naming schemes would require a new code change or RFC instead of a config edit. |
| Requiring template placement by category strengthens document governance consistency. | The RFC still leaves exact naming for new template files open, so one follow-up decision may still be needed during implementation. |

## 5. Acceptance Criteria

- [ ] The workspace adds a new crate at `runtime/project`.
- [ ] The crate package name is `runtime-project`.
- [ ] `runtime-project` exposes a reusable `create_project` plugin operation.
- [ ] `create_project` provisions the governed `doc/` structure used by this repository.
- [ ] `create_project` provisions `.vector/document-types.yaml`.
- [ ] The provisioned documentation type configuration includes `ai-rule`, `prompts`, `template`, and `spec`.
- [ ] Governed document file names always use `{type}-{code}-{slug}.md`.
- [ ] `.vector/document-types.yaml` does not expose `filename_pattern` as user configuration.
- [ ] Each governed folder created by bootstrap has an associated template reference in configuration.
- [ ] The bootstrap creates physical template files that match the accepted category placement.
- [ ] Spec templates are placed under `doc/template/project/`.
- [ ] AI rule templates are placed under `doc/template/ai/`.
- [ ] Prompts templates are placed under `doc/template/prompts/`.
- [ ] The provisioned `prompts` document type is category-based.
- [ ] The provisioned `prompts` document type includes `doc_types` as an initial category.
- [ ] `create_project` does not create `.obsidian/`.
- [ ] `create_project` provisions `CLAUDE.md`, `AGENTS.md`, and `GEMINI.md`.
- [ ] `create_project` provisions `.claude/`, `.codex/`, and `.agents/`.
- [ ] `create_project` provisions `doc/ai-rule/active/ai-rule-00000-master-dispatcher.md` with trigger `always_on`.
- [ ] `create_project` provisions `doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md` with trigger `manual`.
- [ ] `create_project` provisions `doc/ai-rule/active/ai-rule-00002-english-communication.md` with trigger `always_on`.
- [ ] All provisioned ai-rule files conform to the ai-rule template.
- [ ] `create_project` provisions `.vscode/` with the current repository content.
- [ ] The generated `.vscode/settings.json` hides `.vector/`.
- [ ] `create_project` provisions `.editorconfig`, `.gitattributes`, `.gitignore`, and `.mcp.json`.
- [ ] The generated `.codex/config.toml` uses `vector` as its command value.
- [ ] The generated `.mcp.json` uses `vector` as its command value.
- [ ] `runtime-project` introduces no MCP SDK dependency.
- [ ] `create_project` skips any asset file that already exists at the target path — never overwrites.
- [ ] `create_project` does not fail when files are skipped.
- [ ] The operation result reports all skipped paths.

## 6. Open Questions

- Should `create_project` copy bootstrap assets from embedded templates, from filesystem fixtures, or from another repository-owned source abstraction?
- ~~Should `create_project` fail on existing conflicting files, or support an explicit overwrite policy in v1?~~ Resolved: skip existing files silently, report skipped paths in the result.
