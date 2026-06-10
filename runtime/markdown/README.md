# `runtime-markdown`

`runtime-markdown` owns Markdown-specific runtime APIs for Vector. Its public
boundaries are deterministic Markdown discovery and metadata extraction for
local RAG indexing.

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
- **File-Scoped Extraction**: `extract_markdown_file` and
  `extract_markdown_source` convert one discovery record into either a
  serializable extraction record or a file-scoped extraction error.
- **Structured Metadata**: Extraction preserves package identity, governed
  document identity, content hash, frontmatter, headings, outbound links, body
  span, and diagnostics.

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

## Extraction Contract

`extract_markdown_file(&MarkdownDiscoveryRecord)` reads the file through
`runtime-io` and delegates to `extract_markdown_source(...)`. A read failure is
fatal for that call and returns `MarkdownExtractionFailure::ReadFile`. Parsed
source returns `MarkdownExtractionOutcome`:

- `Extracted(MarkdownExtractionRecord)` for successful extraction;
- `Failed(MarkdownExtractionErrorRecord)` for file-scoped extraction errors
  such as malformed frontmatter.

Successful extraction records contain:

- package identity, or `None` for workspace-local documents;
- governed document stem, type, code, and slug when the stem matches the
  governed naming shape;
- content hash preserved from discovery;
- YAML, TOML, or JSON frontmatter as `MarkdownMetadataValue`, including the
  frontmatter source span and format;
- ATX heading nodes with level, text, normalized anchor, zero-based ordinal,
  hierarchy path, and source span;
- outbound governed wikilinks, inline Markdown links, autolinks, and resolved
  reference-style links;
- body span after frontmatter removal;
- non-fatal diagnostics.

Malformed YAML, TOML, or JSON frontmatter returns
`MarkdownExtractionOutcome::Failed` with error kind
`malformed_frontmatter`. The error preserves package identity, document stem,
content hash, source span, frontmatter format, and parser details when
available, so callers can isolate the bad file without aborting unrelated
indexing work.

The current body extraction is intentionally dependency-light while parser
dependency approval is pending. It extracts ATX headings and supported links
outside fenced code blocks, resolves reference-style links from definitions in
the same file, reports unresolved references as
`unresolved_reference_link`, and reports repeated normalized heading anchors as
`duplicate_anchor`. Each successful extraction also includes the
`parser_dependency_spike_deferred` diagnostic until an approved Markdown body
parser replaces the lightweight implementation.

## Boundary Rules

This crate depends on `runtime-io` for traversal, file reading, and file
hashing. It must not depend on `runtime-rag`, embedding implementations,
retrieval stores, MCP SDK types, or governed document authoring APIs.

## License

MIT
