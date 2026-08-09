---
id: adr-00002-remove-rag-functionality-in-vector-mcp
type: adr
code: "00002"
slug: remove-rag-functionality-in-vector-mcp
title: Remove RAG Functionality in Vector MCP
description: Remove the unused RAG subsystem after version 0.5.0 and reserve document indexing for a future, separately evaluated inverted-index design.
status: accepted
created: 2026-08-08
updated: 2026-08-08
tags:
  - rag
  - search
  - mcp
  - architecture
related:
  - task-00077-remove-rag-capability
---

# Remove RAG Functionality in Vector MCP

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: adr-00002-remove-rag-functionality-in-vector-mcp
```

## Context

Vector MCP includes retrieval-augmented generation (RAG) functionality for indexing and retrieving repository content. In observed agent workflows, this subsystem is not used for code search: agents instead use direct repository tools such as `rg`, `grep`, and `awk`, which provide lower-overhead, deterministic search over source files.

Maintaining the RAG subsystem adds implementation, dependency, testing, documentation, and operational cost without demonstrated usage that justifies that cost. Version 0.5.0 is therefore the final release that includes RAG functionality.

Governed technical documents may still benefit from indexed retrieval in the future. An inverted index is a candidate because its deterministic term-based retrieval model is a better fit for structured technical documentation, but that design requires separate evidence and a separate architectural decision.

## Decision

Remove RAG functionality from Vector MCP after version 0.5.0.

The removal includes the public MCP tools, indexing and retrieval implementation, RAG-specific configuration, dependencies, tests, and documentation that exist solely to support RAG. Version 0.5.0 remains the compatibility boundary for consumers that still require this functionality.

Do not introduce an inverted index as part of this removal. Evaluate and specify any replacement for governed-document retrieval independently.

## Rationale

- Direct repository search is already the effective path for agent code discovery.
- Removing unused functionality reduces maintenance surface, dependency exposure, and failure modes.
- A clean removal avoids preserving an unowned compatibility layer with no validated consumer.
- Separating removal from replacement prevents an unvalidated indexing design from inheriting the RAG subsystem's assumptions.

## Alternatives Considered

### Retain RAG unchanged

This preserves compatibility and avoids migration work for unknown consumers, but continues all maintenance and dependency costs despite the absence of demonstrated use.

### Deprecate RAG for an additional release

This provides a longer migration window and better opportunity to collect usage evidence. It also delays simplification and is only valuable if external consumers may exist and can be notified.

### Replace RAG immediately with an inverted index

This could preserve indexed document search while reducing semantic-retrieval complexity. However, it couples two independent changes, creates a new contract before requirements are validated, and increases rollback risk.

## Consequences

### Positive

- Smaller API and implementation surface.
- Fewer dependencies and operational paths to maintain.
- More deterministic code-search behavior based on repository-native tools.
- Freedom to design document retrieval from measured requirements rather than legacy constraints.

### Negative

- Consumers using RAG must remain on version 0.5.0 or migrate.
- Semantic retrieval capabilities disappear until a separately approved replacement exists.
- Removing storage or index formats may make rollback or data reuse harder unless migration guidance is explicit.

## Removal Requirements

- Inventory every public tool, configuration key, dependency, persisted artifact, test, and document tied to RAG before deletion.
- Add release notes that identify version 0.5.0 as the final supported RAG release and list removed interfaces.
- Verify that no non-RAG feature imports or relies on RAG modules transitively.
- Add regression coverage proving the remaining MCP server starts and its retained tools work without RAG configuration or artifacts.
- Define how existing local RAG indexes are handled; do not silently delete user data.
- Record any known consumers and provide a migration path to repository-native search where applicable.
- Measure the governed-document search requirements before proposing an inverted-index contract.

## Staff Engineering Assessment

The removal is directionally sound if telemetry, repository references, and consumer feedback confirm that RAG is unused. The current rationale is weaker than it should be because tool preference by agents does not by itself prove the absence of external consumers or document-retrieval use cases. The main gap is therefore evidence: the implementation change should not ship until the public surface and consumers are inventoried.

Treating 0.5.0 as a firm compatibility boundary is clear, but it shifts migration cost to any undiscovered consumer. A short deprecation period would be safer if Vector MCP has external users; immediate removal is reasonable only when the project can establish that the affected surface is internal or unused.

The inverted-index idea is promising for governed documents, but performance should not be assumed. Corpus size, update frequency, query patterns, ranking expectations, and multilingual tokenization must be measured first, and the replacement should receive its own ADR.
