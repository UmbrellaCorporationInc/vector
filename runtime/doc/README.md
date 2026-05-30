# runtime-doc

Documentation validation and authoring operations for the vector runtime.

This crate provides transport-agnostic documentation governance operations for MCP, CLI, and future frontends. It centralizes validation rules, code allocation, template lookup, and document type scaffolding.

## Operations

### Validation

- **`validate`**: Checks `doc/` layout against `DocumentTypesConfig`, supporting status-based, category-based, and flat (`directory`) layouts. Verifies UTF-8 without BOM, validates frontmatter fields (omitting `status`/`category` for flat layouts), folder placement, file naming, wikilinks, hash-brace substitution variable names, and `.vector/*.yaml` schema field names. This is the authoritative repository-wide failure path for both governed Markdown placeholder naming and `.vector` YAML schema naming.
- **`validate --fix`**: Auto-repairs file placement (including flattening misplaced documents in `directory` layouts), markdown structure, wikilinks, and frontmatter issues. It does not rewrite substitution variable names or `.vector` YAML schema fields because those changes must stay aligned with producer and loader contracts.

### Document Lifecycle

- **`bootstrap_doc`**: Creates a new governed document with auto-allocated code.
- **`bootstrap_doc_type`**: Creates a new document type with folder structure and template, and fails if documentation-rule regeneration fails.
- **`create_doc`**: Creates a document using a per-type authoring prompt when configured, otherwise falling back to the project default authoring prompt. The resolved prompt contract uses `#{doc-type}`, `#{code}`, `#{slug}`, and `#{file-path}`.
- **`create_doc_type`**: Creates a new document type with prompt template and governance metadata. The resolved prompt contract uses `#{doc-type}` and `#{layout}`.

### Authoring

- **`patch_doc`**: Applies a unified diff to a governed document identified by `doc_type` and `code`. Resolves the document path, enforces that the target is inside the repository `doc/` directory, normalizes agent-produced patch wrappers (e.g. Markdown code fences, prose preamble) to raw unified diff, rejects unsupported patch shapes (create, delete, rename, or target mismatch), applies the diff using `patcher`, verifies the resulting content is UTF-8 without BOM, writes the file, and returns `path` and `content`. Any BOM in the result aborts the write and returns an explicit remediation error.

### Discovery

- **`find_doc`**: Locates a document by type and code. Returns `path` (absolute, canonicalized), `package` (always empty — reserved for future package-aware scoping), and `content` (full document text read in the same lookup). The input `package` field is accepted but ignored; lookup remains repository-wide within `root_dir`. See RFC 00027 for the contract rationale and deferred package semantics.
- **`get_doc_types_tags`**: Collects and deduplicates tags across all document types.

## Architecture

- **`types/config.rs`**: `DocumentTypesConfig` model and YAML loader.
- **`internal/`**: Slug validation, next-code resolution, file locating, and naming utilities.
- **`operations/`**: Plugin operations exposed to the runtime.

## Dependencies

- `runtime-core` — plugin primitives
- `runtime-io` — file and text operations
- `patcher` — embedded unified-diff engine used by `patch_doc`
- `serde`, `serde_yaml` — configuration parsing
- `walkdir` — directory traversal
- `regex` — pattern matching
