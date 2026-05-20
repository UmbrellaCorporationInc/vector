---
id: rfc-00019-implement-language-quality-gate-plugin-operation-and-mcp-tool
type: rfc
code: "00019"
slug: implement-language-quality-gate-plugin-operation-and-mcp-tool
title: Implement Language Quality Gate Plugin Operation and MCP Tool
description: Adds a reusable language runtime crate and MCP tool that resolve quality-gate prompts for a list of languages by loading language-rules.yaml, stripping prompt frontmatter, and concatenating the resulting prompt bodies.
status: implemented
created: 2026-05-11
updated: 2026-05-11
authors: []
tags:
  - runtime
  - mcp
  - language
  - prompts
related:
  - spec-00004-language-integration-components-for-mcp
  - rfc-00001-thin-mcp-facade-over-runtime-libraries
supersedes: []
superseded_by: null
aliases:
  - "RFC 00019: Implement Language Quality Gate Plugin Operation and MCP Tool"
---

# RFC 00019: Implement Language Quality Gate Plugin Operation and MCP Tool

## 1. Problem

VECTOR currently has language-specific quality-gate references in `.vector/language-rules.yaml`, but there is no runtime or MCP contract that can resolve those references into the actual prompt content an agent should execute.

Today the repository has these gaps:

- `.vector/language-rules.yaml` maps a language to a `quality-gate` prompt identifier, but no reusable operation loads and resolves that mapping
- there is no runtime crate dedicated to governed language-level prompt resolution
- the MCP server does not expose a tool that accepts a language list and returns the combined quality-gate instructions
- prompt consumers would otherwise need to know how to parse YAML, find the corresponding prompt documents, strip frontmatter, and concatenate the bodies themselves
- that logic would be duplicated across clients and would violate the thin-MCP-facade boundary if implemented directly in `mcp/vector`

This creates friction for prompts such as [[prompts-00004-execute-task-phase]], which already expects a `language-quality-gate` capability to exist.

## 2. Proposal

Add a new reusable runtime crate named `runtime/language/` and a new MCP capability group named `Language` that together expose a tool called `language-quality-gate`.

After this RFC is accepted:

- `runtime/language/` will own a plugin operation named `QualityGate`
- the operation will load `.vector/language-rules.yaml`
- the operation will accept a list of language identifiers
- for each requested language, the operation will resolve the configured `quality-gate` prompt document
- the operation will load the referenced prompt markdown document, remove its YAML frontmatter, keep only the prompt body, and concatenate all requested prompt bodies into one returned string
- `mcp/vector/` will expose a `Language` tool group with a tool named `language-quality-gate`
- the MCP tool will accept a list of languages, execute the runtime plugin operation, and return the combined string result

### 2.1. Runtime crate boundary

Create a new crate at `runtime/language/`.

This crate owns:

- loading `.vector/language-rules.yaml`
- validating that each requested language exists in the configuration
- resolving the configured `quality-gate` prompt reference for each language
- locating and reading the governed prompt documents
- stripping markdown frontmatter from each resolved prompt
- concatenating prompt bodies into one deterministic output string

This crate does not own:

- MCP request decoding or response encoding
- prompt execution
- language-native formatting, linting, or testing commands

This keeps the MCP server as a thin adapter and places reusable behavior in a runtime crate, consistent with [[spec-00004-language-integration-components-for-mcp]].

### 2.2. Language rules configuration contract

The operation must load `.vector/language-rules.yaml` from the project root.

Expected shape:

```yaml
rust:
  quality-gate: prompts-00005-rust
typescript:
  quality-gate: prompts-00006-typescript
```

Required behavior:

- the top-level keys are canonical language identifiers such as `rust` and `typescript`
- each language entry must define `quality-gate`
- the `quality-gate` value is a governed prompt document identifier
- the runtime operation must reject missing language entries
- the runtime operation must reject language entries that omit `quality-gate`
- the runtime operation must reject prompt references that cannot be resolved to an existing governed prompt document

The current repository snapshot uses values like `prompt-00005-rust` and `prompt-00006-typescript` in `.vector/language-rules.yaml`. This RFC proposes the canonical contract name `prompts-<code>-<slug>` to align with the governed document type and avoid singular-plural drift.

### 2.3. Prompt resolution contract

`QualityGate` must resolve each configured prompt reference to a governed document under `doc/prompts/`.

The operation may implement prompt resolution by:

- parsing the code from the configured identifier and locating the corresponding `prompts` document, or
- scanning governed prompt filenames for an exact identifier match

The externally visible contract is:

- the configured prompt reference must resolve to exactly one governed prompt document
- ambiguous resolution is an error
- unresolved references are an error

The implementation must not return markdown frontmatter to the caller.

For each resolved prompt document:

1. Read the file content.
2. Detect and remove the leading YAML frontmatter block delimited by `---`.
3. Preserve the remaining markdown body exactly as the prompt payload.

### 2.4. Output assembly contract

The output of `QualityGate` is a single string containing all resolved quality-gate prompt bodies for the requested language list.

Required behavior:

- preserve the order of the input language list
- resolve each language independently
- concatenate the prompt bodies in input order
- separate prompt bodies with a deterministic delimiter

The recommended delimiter is two newline characters between prompt bodies.

If the same language appears multiple times in the input list, the operation should reject the request rather than duplicate prompt sections silently. This keeps the output intentional and prevents accidental repeated policy blocks.

### 2.5. Plugin operation contract

`runtime/language/` must define a plugin operation named `QualityGate`.

Proposed runtime input/output shape:

```rust
pub struct QualityGateInput {
    pub root_dir: IoPath,
    pub languages: Vec<String>,
}

pub struct QualityGateOutput {
    pub prompt: String,
}
```

Required behavior:

- `languages` must be non-empty
- each language token must be matched case-sensitively against `.vector/language-rules.yaml`
- the operation returns one output containing the concatenated prompt string
- operation errors must explain whether the failure came from configuration loading, unknown language selection, missing prompt metadata, prompt lookup failure, or frontmatter parsing

### 2.6. MCP tool contract

`mcp/vector/` must add a new tool group named `Language`.

That tool group must expose a tool named `language-quality-gate`.

Proposed MCP-facing parameters:

```rust
pub struct LanguageQualityGateParams {
    pub root_dir: String,
    pub languages: Vec<String>,
}
```

Required MCP behavior:

- deserialize the request parameters
- map them into `QualityGateInput`
- execute the runtime `QualityGate` operation through `PluginDispatcher`
- return the final concatenated prompt string to the caller

This tool is a read-only prompt-resolution capability. It must not mutate repository files.

### 2.7. Scope of change

This RFC affects at least these areas:

- new crate scaffolding for `runtime/language/`
- language rules config loading and validation
- governed prompt lookup and markdown frontmatter stripping
- MCP server tool registration
- MCP tool schema and adapter implementation
- automated tests for runtime and MCP layers
- documentation that references the new tool contract

## 3. Alternatives Considered

- **Implement `language-quality-gate` directly inside `mcp/vector`:** Discarded because it would duplicate reusable runtime behavior in the transport layer and violate the thin-facade architecture.
- **Return full markdown files including frontmatter:** Discarded because callers need executable prompt content, not governed metadata. Returning frontmatter would force every consumer to strip it again.
- **Store the full quality-gate text inline in `.vector/language-rules.yaml`:** Discarded because it duplicates governed prompt content, weakens document governance, and makes prompts harder to review and version as first-class documents.
- **Create one MCP tool per language, such as `rust_quality_gate` and `typescript_quality_gate`:** Discarded because the request shape is naturally list-based and multi-language tasks should not require one tool call per language.
- **Make the shared `runtime/lang/` crate own this behavior:** Discarded because this proposal is about governed language-policy resolution, not cross-language parsing or tree-sitter infrastructure.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Gives prompts and agents a canonical way to retrieve language-specific quality-gate instructions. | Adds a new runtime crate and MCP tool surface to maintain. |
| Keeps governed prompt metadata out of the returned execution payload. | Requires a robust frontmatter stripping implementation with clear error handling. |
| Preserves the thin-MCP-facade boundary by placing reusable logic in `runtime/language/`. | Introduces another configuration-to-document resolution path that must stay aligned with naming conventions. |
| Supports multi-language requests in one deterministic response. | Input validation becomes stricter, especially around duplicate or unknown languages. |
| Reuses governed prompt documents instead of duplicating quality-gate text in YAML. | Existing `.vector/language-rules.yaml` values may need normalization to the canonical `prompts-*` identifier form. |

## 5. Acceptance Criteria

- [ ] A new crate exists at `runtime/language/` and owns the reusable quality-gate prompt resolution behavior.
- [ ] The new crate defines a plugin operation named `QualityGate`.
- [ ] `QualityGate` accepts `root_dir` and a non-empty `languages` list.
- [ ] `QualityGate` loads `.vector/language-rules.yaml` from the provided project root.
- [ ] For every requested language, `QualityGate` resolves the configured `quality-gate` prompt reference to exactly one governed prompt document.
- [ ] `QualityGate` rejects unknown languages, missing `quality-gate` mappings, ambiguous prompt references, and unresolved prompt references with explicit errors.
- [ ] `QualityGate` removes YAML frontmatter from every resolved prompt document before assembling the result.
- [ ] `QualityGate` concatenates the resulting prompt bodies into one string in the same order as the input language list.
- [ ] `mcp/vector/` exposes a new tool group named `Language`.
- [ ] The `Language` tool group exposes a tool named `language-quality-gate`.
- [ ] `language-quality-gate` accepts `root_dir` and `languages` as MCP parameters and returns the concatenated prompt string.
- [ ] The tool path remains read-only and does not modify repository files.
- [ ] Automated tests cover runtime config loading, prompt lookup, frontmatter stripping, deterministic prompt concatenation, duplicate-language rejection, and MCP tool registration and execution.
- [ ] Documentation and prompt references that depend on `language-quality-gate` align with the final tool name and request shape.

## 6. Open Questions

- Should the runtime operation support aliases such as `ts` for `typescript`, or should the configuration contract remain strict and accept only canonical language identifiers?
- Should duplicate languages in the input list be rejected, or should they be deduplicated while preserving first-seen order?
- Should prompt reference resolution require the full governed identifier format `prompts-<code>-<slug>`, or should legacy forms such as `prompt-<code>-<slug>` be accepted temporarily for compatibility?
