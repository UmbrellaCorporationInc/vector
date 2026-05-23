# `runtime-language`

## 1. Objective

`runtime-language` owns transport-agnostic language policy operations for the vector system. Its responsibilities are resolving governed quality-gate prompts and best-practices prompts for one or more requested languages by loading `.vector/language-rules.yaml`, locating governed prompt documents, stripping frontmatter, and returning the concatenated prompt bodies.

## 2. Boundaries

### In scope

- Loading `.vector/language-rules.yaml` from a project root
- Normalizing requested language identifiers to lowercase before config lookup
- Resolving governed `prompts-*` quality-gate references
- Resolving governed `prompts-*` best-practices references
- Reading prompt markdown files and stripping YAML frontmatter
- Concatenating prompt bodies in deterministic input order

### Out of scope

- MCP request decoding or response encoding
- Prompt execution
- Language-native lint, test, format, or audit command execution
- Transport-specific error mapping

## 3. Public Interface

### Types

- `QualityGateInput` — input contract containing `root_dir` and ordered `languages`
- `QualityGateOutput` — output contract containing the concatenated `prompt`
- `QualityGateOp` — plugin operation that resolves governed quality-gate prompts
- `BestPracticesInput` — input contract containing `root_dir` and ordered `languages`
- `BestPracticesOutput` — output contract containing the concatenated `prompt`
- `BestPracticesOp` — plugin operation that resolves governed best-practices prompts

## 4. Invariants

- `languages` must be non-empty
- Duplicate languages are rejected after lowercase normalization
- Requested languages with no matching `.vector/language-rules.yaml` entry are skipped
- Languages with a missing `quality-gate` or `best-practices` field, or with either field set to `"none"` or empty, are skipped
- Each configured field that is used must resolve to exactly one governed prompt document
- Returned prompt content must exclude YAML frontmatter

## 5. Dependency Boundary

- This crate must remain transport-agnostic
- MCP-specific logic stays in `mcp/vector`
- Reusable prompt-resolution behavior stays in `runtime-language`
