# `runtime-markdown`

`runtime-markdown` owns Markdown-specific runtime APIs for Vector. Its first
public boundary is deterministic Markdown discovery for local RAG indexing.

## Features

- **Explicit Discovery Inputs**: `MarkdownDiscoveryRequest` accepts workspace
  document roots, synchronized package document roots, traversal options, and
  hashing behavior from callers.
- **Package-Aware Records**: `MarkdownDiscoveryRecord` keeps workspace-local
  documents as `None` package values and preserves package identity for
  synchronized package documents.
- **Governed Stem Validation**: Discovery records are emitted only for Markdown
  files whose stems follow the governed `<doc-type>-<code>-<slug>` shape.
- **Content Hashing**: File hashes are computed through `runtime-io` with
  BLAKE3 over file bytes only.
- **Deterministic Output**: Records and non-fatal issues are sorted so repeated
  runs over the same filesystem state return stable ordering.

## Discovery Contract

`discover_markdown_files(&MarkdownDiscoveryRequest)` walks only the document
roots supplied by the caller. It includes `.md` and `.markdown` files, applies
generic ignored path prefixes through `runtime-io`, validates governed document
stems, and returns records containing:

- package identity, or `None` for workspace-local documents;
- governed document stem;
- file modification time when available;
- content hash;
- internal read path for later indexing.

Missing workspace document roots are fatal
`MarkdownDiscoveryFailure::WorkspaceDiscovery` errors. Missing package document
roots are non-fatal `MarkdownDiscoveryIssue::PackageStructure` values so callers
can report package structure problems without failing the whole workspace
corpus.

## Boundary Rules

This crate depends on `runtime-io` for traversal and file hashing. It must not
depend on `runtime-rag`, embedding implementations, retrieval stores, MCP SDK
types, or governed document authoring APIs.

## License

MIT
