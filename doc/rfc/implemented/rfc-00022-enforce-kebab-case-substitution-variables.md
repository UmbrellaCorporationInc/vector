---
id: rfc-00022-enforce-kebab-case-substitution-variables
type: rfc
code: "00022"
slug: enforce-kebab-case-substitution-variables
title: Enforce Kebab-Case for Substitution Variables and Vector YAML Fields
description: Standardize hash-brace substitution variables and `.vector/*.yaml` field names on kebab-case and reject underscores.
status: implemented
created: 2026-05-18
updated: 2026-05-18
authors: []
tags:
  - validation
  - vscode
  - runtime-doc
related:
  - rfc-00013-runtime-doc-validation-and-authoring-crate
  - rfc-00018-enhanced-markdown-code-blocks
  - rfc-00020-add-create-document-actions-to-vscode-treeview-doc-type-folders
supersedes: []
superseded_by: null
aliases:
  - "RFC 00022: Enforce Kebab-Case for Substitution Variables and Vector YAML Fields"
---

# RFC 00022: Enforce Kebab-Case for Substitution Variables and Vector YAML Fields

## 1. Problem

The repository currently uses multiple naming styles in two related configuration surfaces:

- `runtime/doc` prompt resolution still emits and replaces snake_case placeholders such as `doc_type` and `file_path`.
- the VS Code extension accepts both kebab-case and snake_case because its substitution regex allows `_`.
- the create-document tree flow injects `document_type`, which makes snake_case part of the extension contract.
- governed Markdown documents and bootstrapped project assets contain both styles.
- `.vector/*.yaml` files do not have one enforced field-naming rule across the repository.
- some YAML contracts already deserialize from kebab-case, but others are still loosely parsed and can silently accept non-kebab field names.

This creates four concrete problems:

- the placeholder contract is inconsistent across runtime docs, extension flows, and documentation
- `validate` does not currently inspect substitution variable syntax at all, so invalid placeholder names survive until runtime
- `.vector` configuration files do not share one field-naming invariant, so repo configuration style drifts by file and loader
- broad migration is harder later because historical docs, prompts, assets, and tests keep reinforcing the old names

## 2. Proposal

Standardize every substitution variable name and every YAML field name under `.vector/` on kebab-case, and make underscore-containing names invalid.

### 2.1. Canonical placeholder contract

After this RFC:

- legacy `doc_type` becomes `#{doc-type}`
- legacy `file_path` becomes `#{file-path}`
- legacy `document_type` becomes `#{document-type}`
- placeholders that already comply, such as `#{code}`, `#{slug}`, `#{layout}`, and `#{types}`, remain unchanged

The variable token grammar becomes:

- first character: ASCII letter
- remaining characters: ASCII letters, digits, or `-`
- `_` is forbidden

### 2.2. Runtime validation contract

The `runtime/doc` `validate` operation must scan governed Markdown content for hash-brace placeholders and fail when any variable name is not kebab-case.

Required behavior:

- validation reports the file path and the offending variable name
- validation applies to governed Markdown content, including prompt documents and forms
- `validate_fix` does not silently rename placeholders because the fix is semantic and must stay aligned with the producer code
- the existing structural validation remains unchanged

### 2.3. `.vector` YAML field contract

All field names in YAML files under `.vector/` must be kebab-case.

Required behavior:

- underscore-containing field names such as `document_types`, `prompt_template`, `create_document_form`, or `quality_gate` are invalid
- validation reports the YAML file path and the offending field name
- the rule applies to every configuration YAML under `.vector/`, including `document-types.yaml`, `agents.yaml`, `language-rules.yaml`, and future YAML files added under that folder
- nested field names are included in the rule
- dynamic map keys that are domain identifiers rather than schema field names are not renamed by this RFC
  examples: document type ids such as `rfc`, profile names such as `create-doc`, and language ids such as `rust`
- failures must occur during repository validation, not only when an individual loader happens to read the file

### 2.4. Runtime producer contract

The Rust authoring operations must emit and resolve only kebab-case placeholders:

- `create_doc` uses `#{doc-type}`, `#{code}`, `#{slug}`, and `#{file-path}`
- `create_doc_type` uses `#{doc-type}` and `#{layout}`
- any bootstrap or template-writing flow that inserts placeholder examples must use kebab-case

For `.vector` YAML parsing:

- Rust loaders must reject non-kebab schema fields consistently
- VS Code YAML loaders must reject non-kebab schema fields consistently
- schema validation behavior must not depend on whether a loader uses typed `serde` deserialization or untyped YAML mapping inspection

### 2.5. VS Code extension contract

The VS Code extension must treat kebab-case as the only valid placeholder naming style:

- substitution regexes must stop matching `_`
- unresolved-variable detection must stop matching `_`
- the create-document tree flow must inject `document-type` instead of the legacy `document_type` token
- tests that currently prove mixed-style support must be rewritten to prove underscore rejection

For `.vector` YAML files:

- extension readers such as `.vector/agents.yaml` must validate field names instead of accepting arbitrary mapping keys for schema fields
- error messages must identify the exact file and invalid field

### 2.6. Documentation and asset migration

All governed Markdown documents that contain active placeholder examples or executable prompt content must be migrated to kebab-case, including:

- project docs under `doc/`
- mirrored bootstrap assets under `runtime/project/assets/doc/`
- extension-facing forms and prompts
- RFC and task documents that would otherwise fail the new validation rule because they still contain snake_case placeholder examples
- `.vector` YAML examples embedded in governed documents

## 3. Alternatives Considered

- **Keep mixed support and only recommend kebab-case:** Discarded because it preserves ambiguity and leaves `validate` unable to enforce a single contract.
- **Patch only the VS Code extension and leave Rust prompts unchanged:** Discarded because the repository would still publish two placeholder dialects.
- **Enforce kebab-case only for placeholders and ignore `.vector` YAML fields:** Discarded because configuration style drift would remain in the main repository control plane.
- **Validate only prompt and form documents, not all governed Markdown:** Discarded because invalid examples would continue to spread through RFCs and tasks, then get copied back into active assets.
- **Auto-fix underscore placeholders in `validate_fix`:** Discarded because renaming placeholders without updating the producer or caller can break flows silently.
- **Rely on each YAML loader to enforce its own naming style independently:** Discarded because some loaders are strict today and others are permissive, which produces inconsistent repository rules.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Creates one placeholder naming contract across Rust, VS Code, docs, and assets. | Forces a broad migration across historical Markdown documents, not only executable prompts. |
| Moves failure detection into `validate` instead of runtime-only discovery. | Validation becomes stricter and may surface many pre-existing violations at once. |
| Removes underscore support from the extension and prevents new drift. | Breaks backward compatibility for any local document still using snake_case placeholders. |
| Keeps the rule simple: kebab-case everywhere. | Historical RFC text that documents old contracts must be rewritten to describe the new contract instead of preserving the old examples verbatim. |
| Extends the same naming invariant into `.vector` YAML, which reduces config ambiguity. | Requires new schema validation work for YAML readers that currently parse loose mappings. |

## 5. Acceptance Criteria

- [ ] `runtime/doc` validation fails when any governed Markdown file contains a hash-brace placeholder with `_` in the variable name.
- [ ] Validation error messages identify the governed file and the offending placeholder name.
- [ ] Repository validation fails when any field name in a `.vector/*.yaml` file is not kebab-case.
- [ ] YAML validation error messages identify the `.vector` file and the offending field name.
- [ ] `runtime/doc` `create_doc` resolves `#{doc-type}`, `#{code}`, `#{slug}`, and `#{file-path}`.
- [ ] `runtime/doc` `create_doc_type` resolves `#{doc-type}` and `#{layout}`.
- [ ] The VS Code substitution regex accepts kebab-case placeholders and rejects underscore-containing names.
- [ ] The VS Code create-document flow injects `document-type` instead of the legacy `document_type` token.
- [ ] `.vector/document-types.yaml`, `.vector/agents.yaml`, `.vector/language-rules.yaml`, and other governed YAML files under `.vector/` reject non-kebab schema fields consistently.
- [ ] Governed prompts, forms, and mirrored project assets are migrated from snake_case placeholder names to kebab-case.
- [ ] Governed docs that embed `.vector` YAML examples are migrated to kebab-case field names.
- [ ] Historical governed Markdown documents that still contain snake_case placeholder examples are updated so repository validation passes.
- [ ] Rust and TypeScript tests cover acceptance of kebab-case and rejection of underscore-containing placeholders.
- [ ] Rust and TypeScript tests cover rejection of non-kebab `.vector` YAML schema fields.

## 6. Open Questions

- Should validation inspect every hash-brace placeholder occurrence in the raw Markdown body, or should there be an escape mechanism for literal documentation examples?
- If an escape mechanism is needed later, should it be a fenced-code exemption, an inline escape syntax, or explicit validator allowlisting?
- Should `.vector` YAML field validation live entirely in a repository-wide validator, or should each loader also return the same naming errors defensively when called directly?
