---
id: research-00002-how-the-agent-understands-code
type: research
code: "00002"
slug: how-the-agent-understands-code
title: How the Agent Understands Code
description: Research note on how an AI coding agent builds, maintains, and validates an operational understanding of a codebase.
category: code-search
created: 2026-06-09
updated: 2026-06-09
tags:
  - agents
  - code-understanding
  - research
related:
  - research-00001-source-code-agnostic-ast
---

# How the Agent Understands Code

## Context

This research complements [[research-00001-source-code-agnostic-ast]] by describing the runtime process an agent uses to understand code. A source-code-agnostic AST can provide a normalized structural representation, but the agent still needs a working model that connects syntax, behavior, intent, tests, tooling, and repository conventions.

The agent does not understand code as a single complete parse of the repository. It builds a task-oriented model that is refined through evidence. That model is useful when it is grounded in files, commands, tests, and explicit project rules, and it becomes risky when it relies on unstated assumptions or stale context.

## Understanding Loop

An effective coding agent usually follows this loop:

1. **Intent intake:** Translate the user request into a concrete technical objective, including constraints from repository rules, active instructions, and the current workspace state.
2. **Context selection:** Identify the smallest set of files, documents, tests, and commands needed to answer the objective.
3. **Structural reading:** Parse code organization through modules, namespaces, imports, public APIs, data schemas, tests, and call paths.
4. **Semantic compression:** Convert detailed source code into a compact working model: responsibilities, invariants, dependencies, edge cases, and likely change points.
5. **Hypothesis formation:** Predict what change or explanation is needed, while marking assumptions that still require validation.
6. **Validation:** Use tests, static checks, focused searches, or runtime inspection to confirm or reject the hypothesis.
7. **Model update:** Revise the working model when evidence contradicts the initial understanding.

This loop is iterative. The agent should not assume that the first plausible interpretation is correct.

## Layers of Code Understanding

### Surface Structure

The first layer is the code shape that can be extracted directly:

- File and directory layout.
- Namespaces, packages, modules, and imports.
- Function, class, protocol, schema, and configuration declarations.
- Test locations and fixture structure.
- Build, lint, and runtime entry points.

This layer is where a source-code-agnostic AST is strongest. It can normalize syntax differences across languages and expose comparable structure for search, navigation, and indexing.

### Behavioral Meaning

The second layer is behavior:

- What data enters and leaves a component.
- What side effects occur.
- Which invariants must hold.
- Which errors are expected, retried, ignored, or escalated.
- Which behavior is covered by tests and which behavior is implicit.

This layer cannot be recovered from syntax alone. It requires reading tests, contracts, call sites, logs, examples, and sometimes production-facing documentation.

### Repository Conventions

The third layer is local convention:

- Naming patterns.
- Error handling style.
- Dependency boundaries.
- Preferred helper APIs.
- Test style and fixture reuse.
- Documentation and governance workflows.

Agents need this layer to make changes that fit the codebase. Without it, generated code can be locally correct but socially or operationally misaligned with the project.

### Task Relevance

The fourth layer is relevance. A repository can contain more information than the task can justify reading. The agent needs to choose what matters for the current objective and defer unrelated exploration.

This is a tradeoff: narrow context improves speed and reduces noise, but increases the risk of missing shared behavior. Broad context improves confidence, but can dilute attention and waste time.

## Relationship to a Source-Code-Agnostic AST

A source-code-agnostic AST helps the agent by making structural information explicit and searchable across languages. It can support:

- Fast symbol discovery.
- Cross-language navigation.
- Call graph and dependency extraction.
- Change impact analysis.
- Indexing for retrieval-augmented code understanding.
- Safer automated refactoring primitives.

However, an AST is not the full understanding layer. It should be treated as evidence, not as the final model. The agent still needs to connect the AST to tests, runtime behavior, project rules, and user intent.

The strongest architecture is layered:

1. Use the AST as the stable structural substrate.
2. Enrich it with semantic metadata from tests, type systems, schemas, runtime traces, and documentation.
3. Let the agent build a task-specific working model from that enriched graph.
4. Require validation before presenting conclusions or committing changes.

## Failure Modes

Common failure modes include:

- **Symbol-level overconfidence:** The agent finds a matching function name and assumes it has found the relevant behavior.
- **Context truncation:** The agent loses important earlier constraints or silently drops a file from its working model.
- **Convention drift:** The agent writes code that works but does not match the repository's style or operational boundaries.
- **Test misreading:** The agent treats tests as exhaustive instead of as partial evidence.
- **Stale assumption reuse:** The agent carries forward an earlier hypothesis after new evidence contradicts it.
- **Representation bias:** The agent trusts the normalized AST even when language-specific semantics matter.

These failures are reduced when the agent keeps assumptions explicit and validates them against executable checks or authoritative project documents.

## Practical Implications

For this repository, agent-oriented code understanding should optimize for:

- Governed document rules as first-class context.
- Fast retrieval of relevant source, tests, and documentation.
- Explicit links between research documents, design documents, and implementation tasks.
- Validation workflows that run after generated changes.
- Compact summaries that preserve decisions, assumptions, gaps, and tradeoffs.

For future work on the source-code-agnostic AST, this suggests that the AST should expose not only syntax, but also hooks for semantic enrichment. Useful metadata includes ownership, test coverage links, call-site summaries, schema relationships, runtime entry points, and confidence levels for inferred relationships.

## Open Questions

- How much semantic metadata should be stored in the AST layer versus derived on demand?
- What confidence model should represent inferred relationships between symbols, tests, and documentation?
- How should the system detect when a task requires broad repository context instead of focused local context?
- Which validation signals are strong enough to let an agent act autonomously, and which require human review?

## Conclusion

An agent understands code by constructing a validated, task-specific model from multiple evidence layers. A source-code-agnostic AST can make the structural layer reliable and portable, but the agent still needs semantic enrichment, repository conventions, and validation loops to act correctly.

The main design goal is not to make the agent read everything. It is to make the agent read the right evidence, preserve uncertainty, and update its model when the codebase proves an assumption wrong.
