---
id: template-00007-package-readme
type: template
code: "00007"
slug: package-readme
title: Package README
description: Standard README template for defining a package's objective, boundaries, and public interface.
category: project
created: 2026-05-01
updated: 2026-05-01
tags: []
related: []
---

# Template: Package README

<!-- Replace all <placeholder> values before using. -->

# `<package-name>`

## 1. Objective

<!-- One paragraph. What problem does this package solve? Who is the intended consumer?
     Be specific: a vague objective is a sign the package boundary is not yet clear. -->

<describe the single, focused responsibility of this package>

## 2. Boundaries

### In scope

<!-- What this package owns and is responsible for. -->

- <capability or concern this package handles>
- <...>

### Out of scope

<!-- What this package deliberately does NOT do. Helps consumers know where NOT to look. -->

- <concern that belongs to another package>
- <...>

### Dependencies

<!-- Direct dependencies this package takes on (crates, services, external systems).
     List only direct deps; transitive deps are implicit. -->

| Dependency | Role |
|------------|------|
| `<dep>`    | <why this package needs it> |

## 3. Public Interface

<!-- List the main types, traits, and functions that form the public API.
     Do not exhaustively list every symbol — focus on entry points. -->

### Types

- `<Type>` — <one-line description>

### Traits

- `<Trait>` — <one-line description>

### Key functions / constructors

- `<fn>` — <one-line description>

## 4. Usage Example

```rust
// Minimal example showing the happy path.
```

## 5. Non-Goals & Future Work

<!-- Decisions deferred intentionally. Prevents scope creep and explains design choices. -->

- <thing we chose not to support yet, and why>
