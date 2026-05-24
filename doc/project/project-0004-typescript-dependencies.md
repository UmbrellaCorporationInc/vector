---
id: project-0004-typescript-dependencies
type: project
code: "0004"
slug: typescript-dependencies
title: TypeScript Dependencies
description: Defines the approved TypeScript/Node.js dependencies for the VECTOR VSCode extension.
created: 2026-05-24
updated: 2026-05-24
tags:
  - project
  - typescript
  - dependencies
related:
  - project-0003-rust-dependencies
---

# Dependencies

## Production dependencies

These ship inside the packaged `.vsix` and run in the extension host process.

### 1. @codemirror/state

Tags: #typescript #editor #codemirror
Scope: `frontend/vscode/vector`
Description: Core state model for the CodeMirror 6 editor framework. Provides the immutable document state, transactions, and extension system used by the inline document editor.

### 2. @codemirror/view

Tags: #typescript #editor #codemirror
Scope: `frontend/vscode/vector`
Description: DOM rendering layer for CodeMirror 6. Draws the editor viewport and handles user input events.

### 3. @codemirror/commands

Tags: #typescript #editor #codemirror
Scope: `frontend/vscode/vector`
Description: Standard keyboard command bindings for CodeMirror 6 (undo, redo, selection movement, etc.).

### 4. @codemirror/language

Tags: #typescript #editor #codemirror
Scope: `frontend/vscode/vector`
Description: Language support infrastructure for CodeMirror 6. Provides syntax highlighting, indentation, and language-aware editing features.

### 5. @codemirror/autocomplete

Tags: #typescript #editor #codemirror
Scope: `frontend/vscode/vector`
Description: Autocomplete framework for CodeMirror 6. Used to provide field-aware completion inside governed document frontmatter.

### 6. js-yaml

Tags: #typescript #serialization #yaml
Scope: `frontend/vscode/vector`
Description: YAML parser and serializer approved for parsing governed document frontmatter and `document-types.yaml` configuration in the extension host process.

### 7. markdown-it

Tags: #typescript #markdown #rendering
Scope: `frontend/vscode/vector`
Description: Markdown renderer approved for the governed document preview webview. Converts document body content to HTML for display inside the custom editor panel.

## Development dependencies

Used only during build, type-checking, linting, and testing. Not shipped in the packaged extension.

### 8. typescript

Tags: #typescript #tooling
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript compiler approved as the primary build tool. Compiles extension source to CommonJS output consumed by the VSCode extension host.

### 9. typescript-eslint

Tags: #typescript #linting
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript-aware ESLint integration approved for static analysis and style enforcement across the extension source.

### 10. eslint

Tags: #typescript #linting
Scope: `frontend/vscode/vector` (dev-only)
Description: JavaScript/TypeScript linter approved as the base linting runtime. Used together with `typescript-eslint` for the full lint pipeline.

### 11. prettier

Tags: #typescript #formatting
Scope: `frontend/vscode/vector` (dev-only)
Description: Opinionated code formatter approved for consistent style enforcement across all TypeScript, JSON, and Markdown files in the extension.

### 12. mocha

Tags: #typescript #testing
Scope: `frontend/vscode/vector` (dev-only)
Description: Test runner approved for extension unit and integration tests. Tests are compiled to JS and executed via a custom loader entry point.

### 13. @vscode/test-electron

Tags: #typescript #testing #vscode
Scope: `frontend/vscode/vector` (dev-only)
Description: Official VSCode extension test runner approved for launching a real extension host process during integration tests.

### 14. glob

Tags: #typescript #filesystem
Scope: `frontend/vscode/vector` (dev-only)
Description: File pattern matching library approved for discovering compiled test files at runtime in the test runner script.

### 15. @highlightjs/cdn-assets

Tags: #typescript #syntax-highlighting
Scope: `frontend/vscode/vector` (dev-only)
Description: Highlight.js pre-built assets approved as the syntax highlighting engine for the document preview webview. Assets are copied into the extension output at build time via `scripts/copy-hljs.mjs`.

### 16. @types/js-yaml

Tags: #typescript #types
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript type declarations for `js-yaml`.

### 17. @types/markdown-it

Tags: #typescript #types
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript type declarations for `markdown-it`.

### 18. @types/mocha

Tags: #typescript #types #testing
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript type declarations for `mocha`.

### 19. @types/node

Tags: #typescript #types
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript type declarations for the Node.js runtime APIs used in the extension host and build scripts.

### 20. @types/vscode

Tags: #typescript #types #vscode
Scope: `frontend/vscode/vector` (dev-only)
Description: TypeScript type declarations for the VSCode extension API. Defines the shape of all VSCode host objects and entry points.

### 21. @eslint/js

Tags: #typescript #linting
Scope: `frontend/vscode/vector` (dev-only)
Description: ESLint's core JavaScript rule set approved as the base rule configuration for the extension lint pipeline.

## Governance notes

- All production dependencies must be justified against the VSCode extension host constraints: no Node.js built-ins that are unavailable in the webview sandbox, no native addons.
- The CodeMirror 6 packages (`@codemirror/*`) form a single cohesive framework and are approved as a group. Adding further `@codemirror/*` packages requires explicit justification.
- `js-yaml` is the single approved YAML boundary for the extension. It mirrors the role `serde_yaml` plays on the Rust side; do not add a second YAML library.
- `markdown-it` is the single approved Markdown rendering boundary. Do not add a second Markdown renderer.
- `@highlightjs/cdn-assets` is a build-time copy dependency only. Its assets are bundled into `out/` and served from the extension host; the package itself is not imported in TypeScript source.
- Type declaration packages (`@types/*`) are always dev-only and do not require separate justification beyond their corresponding runtime package.
- `pnpm` overrides for `serialize-javascript >= 7.0.3` and `diff >= 8.0.3` are security patches applied transitively. These are not direct dependencies and do not require entries here.
