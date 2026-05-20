---
id: task-00022-implement-rfc-00015-extension-owned-governed-document-preview-for-vs-code
type: task
code: "00022"
slug: implement-rfc-00015-extension-owned-governed-document-preview-for-vs-code
title: Implement RFC 00015 Extension-Owned Governed Document Preview for VS Code
description: Implements the phased delivery of the Vector-owned WebviewPanel preview for governed documents in VS Code defined by RFC 00015.
status: done
created: 2026-05-08
updated: 2026-05-09
tags:
  - vscode
  - frontend
  - preview
  - markdown
related:
  - rfc-00015-extension-owned-governed-document-preview-for-vs-code
  - adr-00001-adopt-an-extension-owned-governed-document-preview-for-vs-code
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00021-harden-vs-code-sidebar-activation-for-vector-extension
  - rfc-00013-runtime-doc-validation-and-authoring-crate
supersedes: []
superseded_by: null
---

# Task 00022: Implement RFC 00015 Extension-Owned Governed Document Preview for VS Code

## 1. Prime Directive

> Replace the governed reading flow that still depends on native Markdown Preview with a Vector-owned `WebviewPanel` preview that renders governed Markdown predictably, keeps wikilink navigation inside the same reader, and preserves a clean separation between VS Code lifecycle wiring, governed document resolution, and `markdown-it` rendering.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** VS Code extension runtime, `markdown-it`, governed document discovery and resolution modules already used by the sidebar flow

## 3. Checklist

### 3.1. Phase A - Preview Panel Foundation

- [x] Introduce one reusable governed preview controller built around `vscode.WebviewPanel`
- [x] Ensure governed document open flows can route to the preview controller instead of native Markdown Preview
- [x] Keep the preview limited to governed documents opened through Vector-owned flows
- [x] Separate preview lifecycle responsibilities into subscription, provider, and renderer boundaries
- [x] Preserve safe behavior when the target governed document cannot be resolved or read
- [x] Tests covering Phase A
- [x] Validation vector for Phase A
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Base `markdown-it` Renderer

- [x] Add a dedicated `markdown-it` construction path for governed preview rendering
- [x] Keep parsing concerns separate from presentation concerns
- [x] Reuse native `markdown-it` support for fenced code blocks, inline code, and tables before adding custom parsing
- [x] Add renderer customization hooks for governed HTML classes and wrappers where visual treatment requires them
- [x] Add a preview HTML shell builder suitable for webview rendering
- [x] Tests covering Phase B
- [x] Validation vector for Phase B
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Governed Wikilink Parsing and Same-Panel Navigation

- [x] Implement governed wikilink support as `markdown-it` inline parsing or plugin logic for `[[target]]`
- [x] Preserve the governed target stem in structured HTML attributes emitted by the renderer
- [x] Route governed wikilink clicks from the webview back to an extension-owned VS Code command
- [x] Resolve wikilink targets through the same governed lookup boundary used by sidebar navigation
- [x] Re-render successful wikilink targets inside the same preview panel by default
- [x] Avoid blind workspace-wide filename search as the primary wikilink resolution strategy
- [x] Tests covering Phase C
- [x] Validation vector for Phase C
- [x] execute section "4. Quality Gate"

### 3.4. Phase C.5 - Frontmatter Properties Panel

- [x] Extract YAML frontmatter from the raw document content before passing the body to `renderGovernedMarkdown`
- [x] Implement a dedicated frontmatter renderer module that converts parsed frontmatter fields into a styled HTML properties panel
- [x] Render the properties panel above the markdown body inside the same preview HTML shell
- [x] Support rendering scalar values (strings, numbers, booleans), array values (as tag chips), and date-like values
- [x] Keep frontmatter parsing separate from markdown-it rendering — no markdown-it plugin required
- [x] Preserve safe behavior when frontmatter is absent or malformed
- [x] Tests covering Phase C.5
- [x] Validation vector for Phase C.5
- [x] execute section "4. Quality Gate"

### 3.5. Phase C.6 - Frontmatter Document Links

- [x] Detect scalar and array-item values in the frontmatter panel that match the governed stem pattern and render them as clickable document links
- [x] Skip the `id` and `slug` fields — never linkify their values
- [x] Emit the link using a dedicated message type that mirrors the wikilink click dispatch pattern
- [x] Handle the link click in the preview controller by resolving the stem through the governed lookup boundary
- [x] Show an error toast when the stem cannot be resolved instead of silently failing
- [x] Add CSS for the frontmatter document link element consistent with the wikilink pill style
- [x] Tests covering Phase C.6
- [x] Validation vector for Phase C.6
- [x] execute section "4. Quality Gate"

### 3.6. Phase D - Callouts, Code Presentation, and Tables

- [x] Implement callout rendering for the governed form `> [!TYPE] Title`
- [x] Prefer a block-level plugin or block-token transformation for callout support instead of ad hoc HTML rewriting
- [x] Render inline code as visually distinct token-like elements
- [x] Render fenced code blocks as dedicated code sections with clear separation from prose
- [x] Render tables with readable structure, borders, and overflow handling inside the preview
- [x] Tests covering Phase D
- [x] Validation vector for Phase D
- [ ] execute section "4. Quality Gate"

### 3.5. Phase E - Webview Hardening and Resource Safety

- [x] Add a Content Security Policy appropriate for the preview HTML shell
- [x] Serve local preview resources through `webview.asWebviewUri(...)` where applicable
- [x] Keep click handling and state wiring compatible with VS Code webview constraints
- [x] Preserve minimal preview state needed for same-panel continuity without introducing a duplicate governed metadata cache
- [x] Fail safely when preview resources or messages are invalid
- [x] Tests covering Phase E
- [x] Validation vector for Phase E
- [ ] execute section "4. Quality Gate"

### 3.6. Phase F1 - Remove Native Markdown Preview Integration

- [x] Remove `extendMarkdownIt` / `markdownApi` and the `wikilinkPlugin` bridge from `extension.ts`
- [x] Remove `markdown.markdownItPlugins`, `markdown.previewScripts`, `markdown.previewStyles`, and `onLanguage:markdown` from `package.json`
- [x] Delete `media/markdown-preview.js` and `media/markdown-preview.css`
- [x] Migrate `parseGovernedStem` into `document-viewer/wikilinkNavigation.ts` and remove `src/wikilinkPlugin.ts`
- [x] Update all imports that previously pointed to `wikilinkPlugin.ts`
- [x] Remove or update tests that guarded the native preview integration
- [x] Tests covering Phase F1
- [x] Validation vector for Phase F1
- [x] execute section "4. Quality Gate"

### 3.7. Phase F - Flow Integration

- [x] Confirm tree selection opens the extension-owned preview and not a native fallback
- [x] Keep governed document resolution logic shared rather than duplicated between sidebar and preview flows
- [x] Confirm the preview remains scoped to governed documents and does not claim arbitrary workspace Markdown files
- [x] Tests covering Phase F
- [x] Validation vector for Phase F
- [ ] execute section "4. Quality Gate"

### 3.7. Phase Z - Wrap-up

- [x] Update README files on packages modified
- [x] Confirm RFC 00015 acceptance criteria are reflected by the implementation
- [ ] `xtask quality-lint` passes where applicable
- [ ] `xtask quality-test` passes where applicable
- [x] VS Code extension `pnpm run compile` passes
- [x] VS Code extension `pnpm test` passes

## 4. Quality Gate

- [x] VS Code extension `pnpm run compile` passes
- [x] VS Code extension `pnpm test` passes
- [ ] `cargo xtask lint --markdown` passes
- [ ] `cargo xtask vault check --fix` passes

## 5. Validation Vector

- [x] Opening a governed document through a Vector flow renders it in a reusable `WebviewPanel`
- [x] Governed wikilinks render as interactive pill-like elements and navigate inside the same preview panel
- [x] Wikilink resolution uses the governed lookup boundary instead of blind workspace-wide search
- [x] Callouts written as `> [!TYPE] Title` render as distinct callout blocks
- [x] Inline code, fenced code blocks, and tables render with the governed presentation contract defined by RFC 00015
- [x] The preview uses `markdown-it` with clear separation between parsing logic and renderer customization
- [x] Webview HTML respects CSP and webview-safe local resource resolution
- [x] The governed preview replaces native Markdown Preview as the primary governed reading path
- [ ] All phase checkboxes completed

## 6. Execution Notes

- `pnpm run compile` and `pnpm test` pass in `frontend/vscode/vector`.
- `cargo xtask lint --markdown` and `cargo xtask vault check --fix` remain unchecked in this workspace because `cargo` reports `no such command: xtask`.
