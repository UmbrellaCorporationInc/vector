---
id: rfc-00013-runtime-doc-validation-and-authoring-crate
type: rfc
code: "00013"
slug: runtime-doc-validation-and-authoring-crate
title: Runtime Doc Validation and Authoring Crate
description: Defines the runtime doc crate that validates governed documentation, locates governed documents by type and code, creates governed documents, and creates governed document types.
status: implemented
created: 2026-05-05
updated: 2026-05-06
authors: []
tags:
  - runtime
  - documentation
  - validation
  - authoring
  - plugin
related:
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - rfc-00012-runtime-project-bootstrap-crate
  - spec-00003-project-documentation-folder
supersedes: []
superseded_by: null
aliases:
  - "RFC 00013: Runtime Doc Validation and Authoring Crate"
---

# RFC 00013: Runtime Doc Validation and Authoring Crate

## 1. Problem

The project already has a governed `doc/` contract, but it still lacks one reusable runtime crate that owns documentation validation and documentation authoring behavior.

That leaves five concrete gaps:

- there is no canonical validator that checks whether `doc/` matches the governed configuration
- frontmatter validity is not enforced consistently across status-based and category-based document types
- creating new governed documents still requires manual code selection, folder selection, and template lookup
- creating a new document type still requires manual folder creation, manual config editing, and manual template setup
- MCP agents have no way to retrieve the governed prompt that describes how to author a specific document type, so each agent must rediscover the correct sequence of tools independently

If those behaviors stay distributed across ad hoc tools, the documentation system will drift in naming, frontmatter, encoding, folder placement, template usage, and agent-facing authoring instructions.

The project needs one transport-agnostic crate that owns documentation validation and document authoring as reusable plugin operations.

## 2. Proposal

Create a new crate at `runtime/doc` with package name `runtime-doc`.

`runtime-doc` owns documentation governance behavior and exposes seven plugin operations:

- `validate`
- `bootstrap_doc`
- `bootstrap_doc_type`
- `find_doc`
- `get_doc_types_tags`
- `create_doc`
- `create_doc_type`

This crate is responsible for rules that apply to the `doc/` tree after a project already exists. It does not own full project bootstrap.

The documentation configuration file lives at:

- `.vector/document-types.yaml`

### Crate responsibility

Accepted responsibilities:

- validate the governed `doc/` tree against the documentation configuration contract
- validate `.vector/document-types.yaml` as the governed documentation configuration source
- normalize and optionally repair governed documentation drift
- bootstrap governed documents from document type configuration
- bootstrap new governed document types, including folders, config entries, and templates
- locate a governed document by type and code and return its absolute path
- return all tags declared across all document types as a comma-separated string
- resolve and return the agent authoring prompt for a governed document type
- resolve and return the agent authoring prompt for a governed document type definition

Non-responsibilities:

- no full project bootstrap
- no MCP-specific transport logic
- no `.obsidian` management
- no Git policy
- no repository assets outside the documentation contract

### `validate`

`validate` checks whether the `doc/` tree follows the accepted documentation contract. If validation fails, the operation returns an error.

`validate` must check:

- `doc/` follows the configured document type layout
- `.vector/document-types.yaml` exists and is internally consistent
- every governed file is UTF-8 without BOM
- every governed file has valid basic frontmatter
- every governed file name follows `{type}-{code}-{slug}.md`
- every wikilink uses only the target file name without the `.md` extension

Minimum required frontmatter fields:

- `id`
- `type`
- `code`
- `slug`
- `title`
- `description`
- `created`
- `tags`

Additional required frontmatter fields by layout:

- `status` for document types that use status layout
- `category` for document types that use category layout

Extra frontmatter fields are allowed, but minimum required fields must exist.

For status-based document types:

- the folder that contains the file must match the `status` value in frontmatter

For category-based document types:

- the folder that contains the file must match the `category` value in frontmatter

`validate` accepts an optional `fix` mode.

When `fix` is enabled, the operation may:

- move files into the correct status or category folder
- normalize markdown structure for governed files
- normalize wikilinks so they use file names without extensions
- apply safe frontmatter and formatting repairs required to satisfy the accepted contract

`fix` must stay bounded to deterministic governance repairs. It must not invent missing semantic content such as titles or descriptions that cannot be derived safely.

### Slug contract

A slug is a filesystem-safe identifier used as the final segment of a governed document file name. Because slugs are embedded directly into file names, the project must enforce a strict slug contract at every point where a slug is accepted as input or read from frontmatter.

A valid slug:

- contains only lowercase ASCII letters (`a`–`z`), ASCII digits (`0`–`9`), and hyphens (`-`)
- does not start or end with a hyphen
- does not contain consecutive hyphens
- is not empty

Characters that are invalid in slugs include but are not limited to: `:`, `/`, `\`, `?`, `*`, `"`, `<`, `>`, `|`, spaces, and any non-ASCII character.

The slug contract must be enforced in two places:

1. **`validate`** — must report a validation error when a governed file has a `slug` frontmatter value that does not satisfy the slug contract.
2. **`bootstrap_doc`** — must reject a caller-provided slug that does not satisfy the slug contract before creating any file.

A shared internal `validate_slug(slug: &str) -> Result<(), SlugError>` function must implement the contract so both enforcement points use identical rules.

### `bootstrap_doc`

`bootstrap_doc` receives a document type and a slug.

It must:

- validate the provided slug against the slug contract before any other step
- resolve the target document type from configuration
- calculate the next available code for that document type
- derive the target path from the document type layout
- create the file under `doc/`
- build the file content from the configured template for that document type when the template exists

If the document type template does not exist, `bootstrap_doc` must create a minimal empty template shape with frontmatter only and use that shape as the generated content baseline.

`bootstrap_doc` must preserve the governance invariant that document file names always use:

- `{type}-{code}-{slug}.md`

### `bootstrap_doc_type`

`bootstrap_doc_type` creates one new governed document type.

This operation must require an explicit layout choice:

- status-based
- category-based

For a status-based type, the operation must also receive the allowed status values.

For a category-based type, the operation must also receive the allowed category values.

After successful execution, the operation must:

- create the document folder structure for the new type
- update documentation configuration
- create a template for the new type

New type templates created by this operation must live under:

- `doc/template/doc/`

This RFC accepts `doc` as the template category for templates created specifically to support governed document types.

### `find_doc`

`find_doc` locates one governed document by document type and numeric code and returns its absolute path on the vault.

An agent calling `find_doc` receives the full absolute path to the file so it can open, read, or edit that document without having to derive the naming convention or walk the directory tree.

`find_doc` must:

- accept a document type identifier and a numeric code
- resolve the layout for that document type from `.vector/document-types.yaml` (reuse the Phase B loader)
- scan the folder tree for that document type for a file whose name matches `{type}-{code}-{slug}.md` for any slug
- return the absolute path of the matching file
- return a typed error when no file matches the given type and code

`find_doc` must not load or parse the file content. It only resolves the path.

### `get_doc_types_tags`

`get_doc_types_tags` returns all tags declared across all document types in `.vector/document-types.yaml` as a single comma-separated string.

This operation exists to give agents a compact, scannable summary of the tag vocabulary in use across the documentation system — for example, to include in a system prompt or to help an agent decide which document type fits a user request.

`get_doc_types_tags` must:

- load `.vector/document-types.yaml` via the configuration loader
- collect all values from the `tags` field of every document type entry
- deduplicate the collected tags
- return them joined as a single comma-separated string in deterministic order (alphabetical)
- return an empty string when no document type declares any tags

`get_doc_types_tags` must not fail when some document types have no `tags` field — those types are silently skipped.

### `create_doc`

`create_doc` creates one governed document and returns the resolved agent authoring prompt for it.

The operation combines two responsibilities: it provisions the document file using the configured template, then returns the prompt that guides the agent through any follow-up steps (filling content, validating the vault, etc.).

`create_doc` must accept:

- a document type identifier (e.g. `"task"`, `"rfc"`)
- an optional category — required for category-based document types, ignored for status-based types
- a human-readable name used as the document title in the template
- a slug — the filesystem-safe identifier for the file name, validated against the slug contract before any other step

`create_doc` must execute in sequence:

1. Validate the provided slug using the slug contract (`validate_slug`).
2. Resolve the document type from `.vector/document-types.yaml` via the configuration loader.
3. Resolve the `prompt` field for that document type from configuration. Return a typed error when no `prompt` is configured.
4. Resolve the `template` field for that document type from configuration. Return a typed error when no `template` is configured.
5. Compute the next available code for that document type using the next-code resolver.
6. Derive the target file path from the document type layout, the computed code, and the slug.
7. Create the document file under `doc/` using the configured template.
8. Load the prompt file content using the file locator by stem.
9. Replace the following placeholders in the prompt with their resolved values:
   - `#{doc-type}` — the document type identifier
   - `#{code}` — the computed numeric code (zero-padded to `code-width`)
   - `#{slug}` — the validated slug
   - `#{file-path}` — the absolute path of the created document file
10. Return the resolved prompt string to the caller.

A minimal example of a resolved prompt for a `task` document type:

```
The task file has been created at #{file-path}.

Open the file and fill in the Prime Directive and phase checklist for #{doc-type}-#{code}-#{slug}.

Then use the validate tool from mcp/vector to confirm the vault is consistent.
```

`create_doc` owns document creation for this flow. It must not be called alongside a separate `bootstrap_doc` call for the same document — it subsumes that step.

The placeholder vocabulary is fixed: `#{doc-type}`, `#{code}`, `#{slug}`, and `#{file-path}`. No other placeholders are replaced. Extra placeholders in the prompt file are left as-is.

### `create_doc_type`

`create_doc_type` creates one new governed document type and returns the resolved agent authoring prompt for it.

The operation mirrors `create_doc`: it provisions the document type infrastructure by calling `bootstrap_doc_type` internally, then returns the prompt that guides the agent through any follow-up steps (adding documents, validating the vault, etc.).

`create_doc_type` must accept:

- a document type name — validated with the same rules as a slug (lowercase ASCII letters, digits, hyphens; no leading or trailing hyphens; no consecutive hyphens)
- a layout choice: `status` or `category`
- allowed status values — required when layout is `status`
- allowed category values — required when layout is `category`
- a `code-width` — the zero-padding width for numeric codes
- an optional template name

`create_doc_type` must execute in sequence:

1. Validate the document type name against the slug contract.
2. Resolve the `prompt` field declared for document type creation from `.vector/document-types.yaml`. Return a typed error when no such prompt is configured.
3. Execute `bootstrap_doc_type` with the provided inputs — creates folders, updates `.vector/document-types.yaml`, and creates the default template.
4. Load the prompt file content using the file locator by stem.
5. Replace the following placeholders in the prompt with their resolved values:
   - `#{doc-type}` — the new document type name
   - `#{layout}` — the chosen layout (`status` or `category`)
6. Leave unrecognized placeholders in the prompt unchanged.
7. Return the resolved prompt string to the caller.

`create_doc_type` subsumes `bootstrap_doc_type` for this flow. Callers must not call `bootstrap_doc_type` separately for the same type when using `create_doc_type`.

### Configuration contract

`runtime-doc` validates and updates documentation type configuration, but it must treat some rules as internal invariants rather than user options.

Accepted invariant:

- `filename_pattern` is not user-configurable
- governed document file names always use `{type}-{code}-{slug}.md`

That means `validate` and `bootstrap_doc_type` must reject or remove configuration shapes that attempt to make file naming user-configurable.

Each document type entry in `.vector/document-types.yaml` may declare an optional `tags` field. The value is a list of strings that describe the document type. Tags have no enforced vocabulary — they are free-form labels used to aid agent discovery and filtering.

Each document type entry in `.vector/document-types.yaml` may declare an optional `prompt` field. The value is a path relative to the project root pointing to a governed prompt document inside `doc/`. When `prompt` is absent for a type, `create_doc` must return a typed error indicating that no authoring prompt is configured for that type.

`runtime-doc` does not relocate the configuration file under `doc/`. The governed configuration location is `.vector/document-types.yaml`.

### Reuse boundary

`runtime-doc` is reusable infrastructure for any transport.

The dependency direction remains:

- transports depend on `runtime-doc`
- `runtime-doc` depends on runtime contracts
- `runtime-doc` does not depend on MCP SDK types

## 3. Alternatives Considered

- **Keep documentation validation inside `runtime-project`:** Discarded because project bootstrap and ongoing documentation governance are separate responsibilities with different lifecycles.
- **Implement validation only as a CLI or MCP behavior:** Discarded because it would duplicate the same rules across frontends instead of centralizing them in one runtime crate.
- **Make `validate` read-only with no `fix` option:** Discarded because some governance drift is mechanical and should be repairable deterministically.
- **Allow `fix` to rewrite arbitrary document content:** Discarded because governance repair should be structural and safe, not an uncontrolled content mutator.
- **Require templates to exist before `bootstrap_doc` can run:** Discarded because a missing template should not block document creation when a safe frontmatter-only fallback exists.
- **Create new document type templates under `doc/template/project/`:** Discarded because document-type templates are their own governance concern and should live under a dedicated `doc` template category.
- **Make `create_doc` generate the document content instead of returning a prompt:** Discarded because the agent already knows how to call MCP tools — returning an agent-facing prompt keeps document creation under agent control and allows the prompt to orchestrate multiple tool calls (bootstrap, edit, validate) in sequence.
- **Embed authoring prompts directly in code instead of in `doc/`:** Discarded because prompts are governed documents and should live in the repository alongside the types they describe, so they can be reviewed, versioned, and updated without code changes.
- **Make the `prompt` field required for all document types:** Discarded because not all types need an agent authoring flow at creation time; making the field optional lets types opt in progressively.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| One crate centralizes documentation governance rules instead of spreading them across tools. | `runtime-doc` becomes the owner of a broad validation surface that will need careful evolution. |
| `validate --fix` can remove mechanical drift quickly. | Any automatic fixer carries risk if repair boundaries are not kept strict. |
| `bootstrap_doc` makes code allocation and template use consistent. | The operation needs clear fallback behavior when templates or layout metadata are incomplete. |
| `bootstrap_doc_type` lowers the cost of extending the documentation system. | Allowing new types increases the importance of strict validation so the system does not become incoherent. |
| `create_doc` and `create_doc_type` return agent-facing prompts from governed documents, keeping authoring instructions versioned alongside the types they describe. | Prompt contracts can drift unless placeholder semantics are standardized later. |
| Returning a prompt string instead of generating document content keeps `runtime-doc` free of LLM or agent dependencies. | The agent must understand and follow the returned prompt correctly; the runtime cannot enforce agent behavior. |
| Keeping file naming as an invariant removes one major source of configuration drift. | Supporting alternate naming schemes later would require a new RFC and code change. |

## 5. Acceptance Criteria

- [ ] The workspace adds a new crate at `runtime/doc`.
- [ ] The crate package name is `runtime-doc`.
- [ ] `runtime-doc` exposes `validate`, `bootstrap_doc`, `bootstrap_doc_type`, `find_doc`, `create_doc`, and `create_doc_type` as reusable plugin operations.
- [ ] `find_doc` accepts a document type identifier and a numeric code.
- [ ] `find_doc` resolves the layout for the given type from `.vector/document-types.yaml`.
- [ ] `find_doc` scans the folder tree for a file matching `{type}-{code}-{slug}.md` for any slug.
- [ ] `find_doc` returns the absolute path of the matching file.
- [ ] `find_doc` returns a typed error when no file matches the given type and code.
- [ ] `find_doc` does not load or parse file content.
- [ ] `validate` fails when the governed `doc/` tree does not match the configured documentation layout.
- [ ] `validate` uses `.vector/document-types.yaml` as the governed documentation configuration source.
- [ ] `validate` fails when `.vector/document-types.yaml` is missing or inconsistent.
- [ ] `validate` checks that governed files are UTF-8 without BOM.
- [ ] `validate` checks minimum frontmatter fields.
- [ ] `validate` allows extra frontmatter fields beyond the minimum contract.
- [ ] `validate` requires `status` for status-based document types.
- [ ] `validate` requires `category` for category-based document types.
- [ ] `validate` ensures the containing folder matches frontmatter `status` for status-based document types.
- [ ] `validate` ensures the containing folder matches frontmatter `category` for category-based document types.
- [ ] `validate` ensures wikilinks use only target file names without `.md`.
- [ ] `validate` ensures governed file names always use `{type}-{code}-{slug}.md`.
- [ ] `validate` reports an error when a governed file has a `slug` frontmatter value that violates the slug contract.
- [ ] `validate` supports an optional `fix` mode.
- [ ] `fix` can move files into the correct layout folder when placement is wrong.
- [ ] `fix` can normalize markdown structure and wikilinks for governed files.
- [ ] `fix` does not invent missing semantic content that cannot be derived safely.
- [ ] A shared internal `validate_slug` function defines the slug contract used by both `validate` and `bootstrap_doc`.
- [ ] A valid slug contains only lowercase ASCII letters, ASCII digits, and hyphens.
- [ ] A valid slug does not start or end with a hyphen.
- [ ] A valid slug does not contain consecutive hyphens.
- [ ] A valid slug is not empty.
- [ ] `bootstrap_doc` rejects a caller-provided slug that violates the slug contract before creating any file.
- [ ] `bootstrap_doc` accepts a document type and slug.
- [ ] `bootstrap_doc` computes the next available code for the selected document type.
- [ ] `bootstrap_doc` creates the governed document in the correct folder for its layout.
- [ ] `bootstrap_doc` uses the configured template when it exists.
- [ ] `bootstrap_doc` falls back to a frontmatter-only template shape when the configured template does not exist.
- [ ] `bootstrap_doc_type` requires an explicit choice between status-based and category-based layout.
- [ ] `bootstrap_doc_type` requires allowed status values for status-based types.
- [ ] `bootstrap_doc_type` requires allowed category values for category-based types.
- [ ] `bootstrap_doc_type` creates the required document folders.
- [ ] `bootstrap_doc_type` updates the documentation configuration.
- [ ] `bootstrap_doc_type` updates `.vector/document-types.yaml`.
- [ ] `bootstrap_doc_type` creates a template under `doc/template/doc/`.
- [ ] Each document type entry in `.vector/document-types.yaml` supports an optional `tags` field containing a list of free-form string labels.
- [ ] `DocumentTypesConfig` deserializes the `tags` field as an optional list of strings.
- [ ] `get_doc_types_tags` loads `.vector/document-types.yaml` via the configuration loader.
- [ ] `get_doc_types_tags` collects tags from all document type entries.
- [ ] `get_doc_types_tags` deduplicates collected tags.
- [ ] `get_doc_types_tags` returns tags joined as a single comma-separated string in alphabetical order.
- [ ] `get_doc_types_tags` returns an empty string when no document type declares any tags.
- [ ] `get_doc_types_tags` does not fail when some document types have no `tags` field.
- [ ] Each document type entry in `.vector/document-types.yaml` supports an optional `prompt` field pointing to a governed prompt file path.
- [ ] `create_doc` accepts a document type identifier, an optional category, a name, and a slug.
- [ ] `create_doc` validates the slug against the slug contract before any other step.
- [ ] `create_doc` resolves the `prompt` field for the given document type from `.vector/document-types.yaml`.
- [ ] `create_doc` returns a typed error when the target document type has no `prompt` field configured.
- [ ] `create_doc` resolves the `template` field for the given document type from `.vector/document-types.yaml`.
- [ ] `create_doc` computes the next available code for the document type.
- [ ] `create_doc` derives the target file path from the layout, the computed code, and the slug.
- [ ] `create_doc` returns a typed error when no template is configured for the document type.
- [ ] `create_doc` creates the document file using the configured template.
- [ ] `create_doc` loads the prompt file content using the file locator by stem.
- [ ] `create_doc` replaces `#{doc-type}` in the prompt with the document type identifier.
- [ ] `create_doc` replaces `#{code}` in the prompt with the computed numeric code zero-padded to `code-width`.
- [ ] `create_doc` replaces `#{slug}` in the prompt with the validated slug.
- [ ] `create_doc` replaces `#{file-path}` in the prompt with the absolute path of the created file.
- [ ] `create_doc` leaves unrecognized placeholders in the prompt unchanged.
- [ ] `create_doc` returns the resolved prompt string to the caller.
- [ ] `create_doc_type` accepts a document type name, layout, allowed statuses or categories, code width, and optional template name.
- [ ] `create_doc_type` validates the document type name against the slug contract before any other step.
- [ ] `create_doc_type` resolves the prompt declared for document type creation from `.vector/document-types.yaml`.
- [ ] `create_doc_type` returns a typed error when no document type creation prompt is configured.
- [ ] `create_doc_type` executes `bootstrap_doc_type` internally to create folders, update config, and create the default document template.
- [ ] `create_doc_type` creates a prompt template file under `doc/template/doc/` with frontmatter only — title, description, and body left empty as placeholders.
- [ ] `create_doc_type` updates `.vector/document-types.yaml` to set the `prompt` field for the new type pointing to the created prompt template.
- [ ] `create_doc_type` loads the prompt file content using the file locator by stem.
- [ ] `create_doc_type` replaces `#{doc-type}` in the prompt with the new document type name.
- [ ] `create_doc_type` replaces `#{layout}` in the prompt with the chosen layout.
- [ ] `create_doc_type` leaves unrecognized placeholders in the prompt unchanged.
- [ ] `create_doc_type` returns the resolved prompt string to the caller.
- [ ] `runtime-doc` does not expose `filename_pattern` as user configuration.
- [ ] `runtime-doc` introduces no MCP SDK dependency.

## 6. Open Questions

- Should `validate --fix` reorder frontmatter fields into one canonical order, or only ensure that required fields exist?
- Should `bootstrap_doc` require the caller to provide initial `status` or `category` when the target type supports multiple valid folders?
- Should `bootstrap_doc_type` also create one default empty document instance for the new type, or only the type infrastructure?
- What placeholder vocabulary and replacement rules should `create_doc` and `create_doc_type` support?
- Should UTF-8 without BOM validation apply to every file under `doc/`, or only to governed markdown and governed config files?
- Should `create_doc` validate that the resolved prompt file is itself a governed document (i.e. has valid frontmatter), or treat it as an opaque text file?
- Should `bootstrap_doc_type` automatically create a default prompt document and set the `prompt` field in configuration, or leave prompt authoring as a manual follow-up step?
