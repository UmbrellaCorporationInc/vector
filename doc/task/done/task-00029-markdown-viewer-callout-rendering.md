---
id: task-00029-markdown-viewer-callout-rendering
type: task
code: "00029"
slug: markdown-viewer-callout-rendering
title: Markdown Viewer Callout Rendering
description: The markdown viewer renders Obsidian callout syntax as a plain blockquote instead of a styled callout component, making callout blocks visually indistinguishable from regular quotes.
status: done
created: 2026-05-11
updated: 2026-05-11
tags:
  - markdown
  - viewer
  - bug
related:
  - task-00028-enhanced-markdown-code-blocks
supersedes: []
superseded_by: null
---

# Task 00029: Markdown Viewer Callout Rendering

## 1. Prime Directive

> [!Prime Directive]
> The viewer treats `> [!type]` blockquotes as regular quotes. Documents that use callouts — including governed task files like task-00028 — lose their visual structure entirely. This task adds a callout parser and renderer so the `> [!type]` syntax displays as a styled, typed component.

## 2. Bug Description

Obsidian callout syntax opens with `> [!type]` on the first line of a blockquote, optionally followed by a title on the same line, and the body on subsequent `> ` lines:

```markdown
> [!warning] Optional Title
> Body content of the callout.
```

The current viewer passes blockquotes through unchanged, so the `[!type]` token appears as raw text inside an unstyled `<blockquote>`. There is no type-specific styling, no icon, and no title bar.

**Reproducer:** open any task document in the viewer — the `> [!Prime Directive]` block in §1 renders as a plain quote.

## 3. Specs

- **Module:** markdown viewer (Vector VSCode extension)
- **Dependencies:** none — detection is a pure string parse on the first blockquote line

## 4. Checklist

### 4.1. Phase A — Callout Parser

- [x] Detect the `> [!type]` pattern as the opening line of a blockquote
- [x] Extract `type` (case-insensitive) and optional inline title (`> [!type] Title text`)
- [x] Extract the body lines (remaining `> ` prefixed lines)
- [x] Pass unmatched blockquotes through to the standard renderer unchanged
- [x] Execute quality gate

### 4.2. Phase B — Callout Renderer

- [x] Render matched callouts as a styled component with a title bar showing the type and optional title
- [x] Apply type-specific icon and accent color for the known Obsidian types: `note`, `info`, `tip`, `success`, `warning`, `danger`, `bug`, `example`, `quote`
- [x] Unknown types (e.g., `Prime Directive`) fall back to the `note` style
- [x] Render the callout body as parsed markdown (supports inline formatting, nested lists, code spans)
- [x] Execute quality gate

### 4.3. Phase Z — Wrap-up

- [x] Verify callouts in all existing governed documents render correctly
- [x] Update extension README / changelog
