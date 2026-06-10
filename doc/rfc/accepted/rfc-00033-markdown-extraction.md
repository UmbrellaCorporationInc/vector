---
id: rfc-00033-markdown-extraction
type: rfc
code: "00033"
slug: markdown-extraction
title: Markdown Extraction
description: Proposes the Markdown metadata extraction boundary for phase 3 of the local RAG implementation.
status: accepted
created: 2026-06-10
updated: 2026-06-10
authors: []
tags:
  - rag
  - markdown
  - extraction
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00033: Markdown Extraction"
---

# RFC 00033: Markdown Extraction

```vector-agent-action
label: Create a task
profile: create-doc
prompt: prompts-00005-create-document
input:
  message: Create a task to implement the `rfc-00033-markdown-extraction`
  document-type: task
  document-name: implement-rfc-00033-markdown-extraction
```

## 1. Problem

Phase 3 of [[spec-00011-rag-plan-implementation]] needs a concrete extraction contract before the indexer can chunk, embed, and persist Markdown documents. The current plan states what must be extracted, but it does not define the runtime boundary, the normalized output shape, or how partial failures should be represented.

Without that contract, later phases risk duplicating Markdown parsing logic, storing inconsistent metadata, or treating malformed frontmatter as a global indexing failure. The extraction layer must be deterministic, package-aware, and precise enough for chunking, filtering, attribution, and debugging.

## 2. Proposal

Introduce a Markdown extraction boundary that receives a discovered Markdown file record and returns either a normalized extraction record or a file-scoped indexing error. The extraction boundary must run before chunking and must not perform embedding or persistence.

The extractor should produce:

- Governed source identity: package, document stem, document type, code, and slug when the stem follows the governed naming convention.
- Frontmatter: parsed YAML, TOML, or JSON as structured metadata, plus the original format.
- Heading hierarchy: stable heading nodes with level, text, normalized anchor, ordinal, source span, and parent path.
- Outbound links: Markdown links, governed wikilinks, autolinks, and reference-style links where the parser can resolve the target from the same file.
- Body span: the Markdown content range after frontmatter, so chunking can operate on the correct source text.
- Diagnostics: non-fatal warnings for duplicate anchors, unresolved reference definitions, or unsupported link forms.

The first implementation should use a parser-backed Markdown pipeline instead of ad hoc line scanning. Frontmatter detection may remain a small deterministic pre-pass because frontmatter delimiters are outside the Markdown body and must be removed before heading and link extraction.

### Parser And Dependency Direction

This RFC proposes a two-part parsing strategy:

- Use `gray_matter` for frontmatter extraction because it directly supports YAML, TOML, and JSON metadata.
- Run a short implementation spike before committing the Markdown body parser. Compare `markdown::to_mdast` against `pulldown-cmark::OffsetIter` using the fixtures required by this RFC.

The preferred candidate is `markdown::to_mdast` because the extraction output is AST-shaped: headings, links, reference definitions, source positions, and hierarchy need to become structured records. `pulldown-cmark` remains a viable fallback because it is mature and supports source offset iteration, but its event-based model may require more custom state management to reconstruct heading hierarchy, section spans, and reference-style link resolution.

The spike must decide the Markdown body parser using these criteria:

- Source positions are stable enough to produce `source_span` and `body_span`.
- Heading hierarchy can be reconstructed without fragile string scanning.
- Links inside fenced code blocks are ignored by construction.
- Inline links, autolinks, and resolvable reference-style links are extractable.
- Parser behavior is deterministic across repeated runs on the same source.
- The dependency is actively maintained and acceptable under the project dependency-governance process.

Governed wikilinks such as `[[rfc-00032-markdown-discovery]]` are not standard CommonMark links. The first implementation should treat wikilink extraction as a Vector-owned extension layered on top of parser output, preferably by inspecting text nodes outside code blocks rather than scanning the full raw document.

### Extraction Record Example

```json
{
  "package": null,
  "document_stem": "spec-00011-rag-plan-implementation",
  "document_type": "spec",
  "document_code": "00011",
  "document_slug": "rag-plan-implementation",
  "document_hash": "sha256:8d4f...",
  "frontmatter": {
    "format": "yaml",
    "metadata": {
      "id": "spec-00011-rag-plan-implementation",
      "type": "spec",
      "code": "00011",
      "slug": "rag-plan-implementation",
      "title": "RAG Plan Implementation",
      "tags": ["rag", "implementation", "local"]
    }
  },
  "headings": [
    {
      "level": 1,
      "text": "SPEC 00011: RAG Plan Implementation",
      "anchor": "spec-00011-rag-plan-implementation",
      "ordinal": 0,
      "path": ["SPEC 00011: RAG Plan Implementation"],
      "source_span": { "start": 364, "end": 406 }
    },
    {
      "level": 3,
      "text": "Phase 3: Extract Markdown Metadata",
      "anchor": "phase-3-extract-markdown-metadata",
      "ordinal": 7,
      "path": [
        "SPEC 00011: RAG Plan Implementation",
        "2. Definition",
        "Phase 3: Extract Markdown Metadata"
      ],
      "source_span": { "start": 2216, "end": 2261 }
    }
  ],
  "links": [
    {
      "kind": "wikilink",
      "raw": "[[research-00003-local-rag]]",
      "target": "research-00003-local-rag",
      "label": null,
      "heading": null,
      "source_span": { "start": 514, "end": 542 }
    },
    {
      "kind": "wikilink",
      "raw": "[[rfc-00032-markdown-discovery]]",
      "target": "rfc-00032-markdown-discovery",
      "label": null,
      "heading": null,
      "source_span": { "start": 1537, "end": 1570 }
    }
  ],
  "body_span": { "start": 364, "end": 8265 },
  "diagnostics": []
}
```

### Malformed Frontmatter Error Example

```json
{
  "package": "shared-docs",
  "document_stem": "rfc-00020-example",
  "document_hash": "sha256:38be...",
  "error": {
    "kind": "malformed_frontmatter",
    "message": "YAML frontmatter could not be parsed.",
    "source_span": { "start": 0, "end": 180 },
    "details": {
      "format": "yaml",
      "line": 6,
      "column": 12
    }
  }
}
```

### Heading Path Example For Chunking

```json
{
  "document_stem": "spec-00011-rag-plan-implementation",
  "section": {
    "heading_path": [
      "SPEC 00011: RAG Plan Implementation",
      "2. Definition",
      "Phase 3: Extract Markdown Metadata"
    ],
    "heading_anchor": "phase-3-extract-markdown-metadata",
    "content_span": { "start": 2262, "end": 2664 }
  }
}
```

The extractor should keep duplicate headings distinct by assigning a stable ordinal to each heading. Anchor normalization may produce the same anchor for duplicate headings, but the heading path and ordinal together must remain stable for chunking.

## 3. Alternatives Considered

- **Chunk first, extract metadata later:** Discarded because chunking needs heading paths and body spans to produce stable, attributable chunks.
- **Store raw frontmatter only:** Discarded because retrieval filters need structured fields such as tags, type, status, category, and authors.
- **Abort the whole indexing run on malformed frontmatter:** Discarded because the phase plan requires malformed document failures to stay isolated to the affected file.
- **Use regular expressions for all Markdown extraction:** Discarded because nested headings, fenced code blocks, reference links, and escaped Markdown syntax are parser concerns.
- **Commit directly to `pulldown-cmark` from the research recommendation:** Deferred because the RFC output is closer to an AST contract than a streaming event contract, and source-span behavior must be verified against fixtures.
- **Commit directly to `markdown::to_mdast` without a spike:** Deferred because crate maturity, source-position stability, and extension behavior must be verified before introducing the dependency.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Creates one reusable contract for chunking, indexing, filtering, and diagnostics. | Requires a parser-backed implementation and fixtures before visible retrieval behavior improves. |
| Keeps malformed frontmatter isolated to a single source file. | Callers must handle partial indexing results instead of assuming all files either succeed or fail together. |
| Preserves package and governed document identity before persistence. | The extraction record is larger than a minimal chunking input. |
| Makes heading paths deterministic and testable. | Duplicate headings require explicit ordinal handling in downstream chunk identifiers. |
| Uses `gray_matter` for frontmatter instead of custom parsing. | Adds a third-party dependency that must pass dependency governance. |
| Spikes the Markdown body parser before committing. | Adds a small upfront task before phase 3 implementation can begin. |

## 5. Acceptance Criteria

- [ ] The extractor accepts stable Markdown file records emitted by phase 2 and preserves package, document stem, document hash, and read identity.
- [ ] YAML, TOML, and JSON frontmatter are parsed into structured metadata with format information.
- [ ] Malformed frontmatter returns a clear file-scoped indexing error and does not abort unrelated files.
- [ ] The implementation records the spike result choosing either `markdown::to_mdast`, `pulldown-cmark`, or a documented fallback before adding the Markdown body parser dependency.
- [ ] Heading extraction preserves nested hierarchy, duplicate heading occurrences, source spans, normalized anchors, and stable ordinals.
- [ ] Outbound links are extracted for inline Markdown links, governed wikilinks, autolinks, and resolvable reference-style links.
- [ ] Fenced code blocks do not produce headings or links from their literal contents.
- [ ] Fixtures cover valid frontmatter, malformed frontmatter, duplicate headings, nested headings, links inside prose, links inside code blocks, and package-scoped documents.
- [ ] The extraction output is serializable for tests and debugging without requiring LanceDB or embedding dependencies.

## 6. Open Questions

- Should unresolved reference-style links be warnings, ignored links, or hard extraction errors?
- Should anchor normalization match GitHub Markdown anchors exactly, or should Vector define its own stable anchor algorithm?
- Should frontmatter metadata preserve original scalar types exactly, or normalize selected governed fields such as `tags`, `aliases`, and `related`?
- Should the parser spike live as a separate research note, a task artifact, or an implementation section in this RFC before acceptance?
