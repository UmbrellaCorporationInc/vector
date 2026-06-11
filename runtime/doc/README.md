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

- **`patch_doc`**: Applies a patch to a governed document identified by `doc_type`, `code`, and optional `package`. Supported formats are `unified` and `apply_patch`; omitted `format` defaults to `apply_patch`. The operation resolves the document path, enforces that the patch target matches the resolved governed document, applies the selected format, verifies the resulting content is UTF-8 without BOM, writes the file, and returns `path` and `content`. Unified-diff payloads preserve malformed hunk line-count diagnostics that compare declared `@@ -a,b +c,d @@` counts to the parsed hunk body. `apply_patch` payloads support `*** Update File:` for the resolved document and reject add, delete, move, ambiguous context, missing-boundary, and target-mismatch cases with format-specific diagnostics. Any BOM in the result aborts the write and returns an explicit remediation error.

Unified diff example:

```rust
use runtime_doc::operations::{PatchDocFormat, PatchDocInput};
use runtime_io::path::IoPath;

let input = PatchDocInput::with_format(
    IoPath::new("/path/to/project"),
    String::new(),
    "rfc".to_string(),
    37,
    PatchDocFormat::Unified,
    "--- a/doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n+++ b/doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n@@ -1 +1 @@\n-old\n+new\n".to_string(),
);
```

Omitted-format `apply_patch` example:

```rust
use runtime_doc::operations::{PatchDocFormat, PatchDocInput};
use runtime_io::path::IoPath;

let format = PatchDocFormat::parse_optional(None).expect("omitted format defaults to apply_patch");
let input = PatchDocInput::with_format(
    IoPath::new("/path/to/project"),
    String::new(),
    "rfc".to_string(),
    37,
    format,
    "*** Begin Patch\n*** Update File: doc/rfc/draft/rfc-00037-extend-patch-doc-formats.md\n@@\n-old\n+new\n*** End Patch\n".to_string(),
);
```

### Discovery

- **`find_doc`**: Locates a document by type and code. Returns `path` (absolute, canonicalized), `package` (the package name, or empty for workspace-local lookup), and `content` (full document text read in the same lookup). The optional input `package` field allows resolving against the synchronized package location under `.vector-database/packages/{package}/` when set, rather than performing a workspace-local lookup. See RFC 00030 for package-qualified lookup semantics.
- **`get_doc_types_tags`**: Collects and deduplicates tags across all document types.

## Architecture

- **`types/config.rs`**: `DocumentTypesConfig` model and YAML loader.
- **`internal/`**: Slug validation, next-code resolution, file locating, and naming utilities.
- **`operations/`**: Plugin operations exposed to the runtime.

## Dependencies

- `runtime-core` — plugin primitives
- `runtime-io` — file and text operations
- `patcher` — embedded unified-diff engine used by `patch_doc` when `format` is `unified`
- `serde`, `serde_yaml` — configuration parsing
- `walkdir` — directory traversal
- `regex` — pattern matching
