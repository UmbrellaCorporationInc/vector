---
id: rfc-00021-chat-input-file-mentions-in-form
type: rfc
code: "00021"
slug: chat-input-file-mentions-in-form
title: Chat Input File Mentions In Form
description: Replace the plain textarea-backed `chat-input` field with a mention-aware editor that can insert governed workspace file references for agent prompts.
status: implemented
created: 2026-05-16
updated: 2026-05-18
authors:
  - fernandojerez
tags:
  - vscode
  - forms
  - chat-input
  - agents
related:
  - rfc-00018-enhanced-markdown-code-blocks
supersedes: []
superseded_by: null
aliases:
  - "RFC 00021: Chat Input File Mentions In Form"
---

# RFC 00021: Chat Input File Mentions In Form

## Implementation update

Task `00037` delivered the initial document-viewer-scoped mention editor, and task `00039` completed the accepted CodeMirror 6 migration for the editable runtime. The accepted direction did not materially diverge, but examples and module references in this RFC now use the current kebab-case `document-viewer/form-editor/` path.

## 1. Problem

The current `chat-input` field in the VS Code governed document viewer renders as a plain `<textarea>` with no structured editing behavior. That implementation is sufficient for free-form text, but it blocks an important workflow for agent-driven prompts:

- users cannot reference workspace files inline from the prompt authoring surface
- there is no `@` trigger to discover files while typing
- the extension cannot distinguish plain text from an intentional file attachment or file reference
- agent prompts receive only a raw string, which prevents downstream tooling from treating referenced files as first-class context

This gap is especially visible in create and execution flows where the user wants to say things like "review `@frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts`" without manually typing long paths.

The current webview architecture also creates a constraint: the viewer cannot literally embed the native VS Code file search widget inside the textarea. Any file lookup must be mediated by the extension host and exposed back into the webview as a bounded interaction contract.

## 2. Proposal

Replace the current textarea-backed `chat-input` implementation with a mention-aware plain-text editor that supports file references triggered by `@`, Markdown authoring cues, and dynamic height growth.

### 2.1. User experience

When the user types `@` inside a `chat-input` field:

1. the editor opens a file mention suggestion surface
2. suggestions are resolved from the current VS Code workspace through the extension host
3. the user can filter by typing part of the file name or relative path
4. selecting a result inserts a stable file mention token into the editor
5. the rendered text remains readable as plain text while the submission payload preserves structured metadata for each mention

Example authoring experience:

```text
Please review @frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts
and compare it with @doc/rfc/implemented/rfc-00018-enhanced-markdown-code-blocks.md
```

The same editor may also contain lightweight Markdown authoring such as:

```text
## Subtitle

Please review **this file** and summarize the risks.
```

The editor should visually style Markdown syntax while keeping the underlying value as plain Markdown text.

### 2.2. Data contract

The submitted `chat-input` value must preserve both:

- the user-visible prompt text
- a structured list of mentioned files

Recommended payload shape:

```json
{
  "text": "Please review @frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts",
  "mentions": [
    {
      "type": "file",
      "label": "formRenderer.ts",
      "path": "frontend/vscode/vector/src/document-viewer/form-editor/formRenderer.ts"
    }
  ]
}
```

This lets downstream agent execution choose between:

- serializing mentions back into plain text for legacy flows
- attaching mentioned file paths as explicit context in future agent integrations

Markdown markers such as headings, emphasis, lists, and fenced code blocks must remain part of the `text` field exactly as authored.

For the **first iteration**, the current agent execution flow must consume **only** the plain text content field. The structured `mentions` payload may be emitted by the editor, but it is stored as forward-compatible metadata and is **not** interpreted by the existing execution path yet.

### 2.3. Editor choice

This RFC recommends **CodeMirror 6** as the default implementation for `chat-input`.

The implementation should use:

- a minimal CodeMirror setup focused on plain text editing
- Markdown-aware syntax styling for common prompt structures such as headings, emphasis, lists, inline code, and fenced code blocks
- a custom extension for `@` mention detection and insertion
- extension-host-backed file search instead of a browser-only fuzzy index
- dynamic height growth so the editor expands with content instead of forcing a fixed textarea size
- graceful fallback to plain text behavior when mention resolution fails

This is intentionally **not** a rich text editor proposal. The field remains a plain-text prompt editor with structured inline file references and Markdown source text. Styling is a visual aid for authoring, not a WYSIWYG document model.

### 2.4. Markdown styling model

The editor may visually distinguish Markdown constructs while preserving the raw source text.

Examples:

- `## Subtitle` may render with stronger heading styling
- `**bold**` may render with emphasized visual weight
- `` `inline code` `` may render with code styling
- fenced code blocks may render with code block chrome inside the editor

Non-goal:

- the editor must not hide or replace Markdown syntax in a way that makes the stored value diverge from what the user typed
- the first implementation should not become a rich text or prose editor

### 2.5. Dynamic height and form layout

The editable `chat-input` must grow dynamically as the user types, similar to modern code-assistant chat inputs.

Requirements:

- the editor starts at a comfortable minimum height
- the editor expands vertically with content up to a bounded maximum height
- once the maximum height is reached, the internal editor surface scrolls
- the surrounding `.vector-form` and `.vector-form-field` CSS grid layout must accommodate the taller editor without clipping or overlap
- grid rows must size naturally from content rather than depending on a fixed `rows="10"` textarea contract

This preserves the current form layout approach while making `chat-input` feel adaptive rather than static.

### 2.6. Host and webview responsibilities

The webview is responsible for:

- rendering the editor
- detecting mention triggers
- styling Markdown authoring cues
- requesting file suggestions
- rendering and inserting selected mentions
- measuring content height and growing the editor within its layout bounds
- preserving selection, undo/redo, and caret position

The extension host is responsible for:

- resolving searchable workspace file candidates
- applying workspace-specific filtering rules when needed
- returning bounded suggestion results to the webview
- translating mention payloads into the format expected by agent execution

### 2.7. Compatibility expectations

- existing `vector-form` documents continue to use the same `chat-input("Label")` DSL
- documents that do not use `@` mentions remain valid and behave as plain prompt inputs
- documents that use Markdown in `chat-input` remain plain text prompts and do not require a renderer migration
- current agent execution flows must continue to work by falling back to the `text` field when structured mentions are not yet consumed downstream
- in the first iteration, current agent execution flows consume only the `text` or `content` field and ignore `mentions`
- read-only `chat-input` rendering remains read-only and does not instantiate the interactive editor

### 2.8. Code organization

The implementation should remain inside `document-viewer/` for now, because both `form-editor` and `chat-input` are currently document-viewer-specific capabilities rather than reusable top-level UI platforms.

Recommended ownership split:

- `document-viewer/` remains responsible for rendering governed documents and placing form field containers into the webview
- `document-viewer/form-editor/` remains the home of generic document-form rendering behavior
- `document-viewer/chat-input/` owns the specialized interactive editor implementation for editable `chat-input` fields
- extension-host integration points may expose `chat-input` services or adapters where needed, while still treating them as part of the document viewer domain

Rationale:

- `chat-input` is growing in complexity, but it still applies only to governed document forms
- keeping it under `document-viewer/` preserves a clear domain boundary and avoids introducing a premature top-level module
- a dedicated `document-viewer/chat-input/` folder still gives the feature room to grow without overloading `form-editor/`

Future refactoring remains possible if the editor becomes shared outside governed document flows, but this RFC does not require that extraction now.

## 3. Alternatives Considered

### 3.1. CodeMirror 6

**Summary:** Lightweight programmable text editor with a strong extension model.

**Pros**

- well suited for plain-text editors with custom inline behavior
- significantly smaller and less opinionated than Monaco
- first-class control over decorations, cursor behavior, transactions, and autocomplete
- can add Markdown-aware styling without committing to a full rich-text model
- good fit for a webview where we want one focused feature instead of a full IDE

**Cons**

- adds a new runtime dependency family
- requires custom work for mention UX, serialization, and styling
- dynamic sizing and Markdown presentation still need custom implementation decisions
- does not automatically inherit native VS Code search behavior

### 3.2. Monaco Editor

**Summary:** Full editor platform that more closely resembles the VS Code editing model.

**Pros**

- familiar editing feel for VS Code users
- strong text model APIs and suggestion support
- can support Markdown-aware authoring presentation
- future-friendly if the viewer later grows into a more code-heavy editor surface

**Cons**

- heavier bundle and integration cost for a simple form field
- likely overpowered for plain prompt authoring
- increases maintenance cost in a webview where we do not need full language tooling

### 3.3. Enhanced `<textarea>` With Overlay / Popover

**Summary:** Keep the native textarea and implement mention detection plus a floating picker around it.

**Pros**

- lowest dependency cost
- minimal migration from the current renderer
- keeps browser-native form semantics

**Cons**

- fragile caret measurement and token rendering
- dynamic auto-growth plus styled Markdown cues become increasingly hacky
- difficult to keep selection, undo, and token boundaries reliable
- becomes increasingly hard to maintain as soon as mentions need richer editing behavior

### 3.4. `contenteditable` / Lexical / Slate / ProseMirror

**Summary:** Use a rich text editor stack or a low-level document editor framework.

**Pros**

- flexible token rendering
- strong support for custom inline nodes
- future path for richer prompt chips, attachments, and mixed content

**Cons**

- too much abstraction for a plain-text prompt use case
- higher complexity in serialization and keyboard behavior
- invites accidental scope growth into rich-text editing, which this product does not need today

### 3.5. Native VS Code QuickPick Outside The Input

**Summary:** Keep the textarea simple and provide a separate command or button that opens a native QuickPick to insert a file path.

**Pros**

- leverages VS Code-native search affordances
- simpler than building an inline mention list in the webview
- clear fallback if inline mention UX proves too expensive

**Cons**

- breaks typing flow because the user leaves the input interaction
- does not feel like modern chat-style authoring
- weaker mental model than typing `@` directly where the reference should appear

## 4. Tradeoffs

| Pro | Con |
|---|---|
| File mentions become first-class prompt context instead of fragile manually typed paths | Introduces editor complexity into a surface that is currently just HTML form rendering |
| Users get a faster and more discoverable chat authoring workflow | Requires a webview-to-extension-host protocol for search and insertion |
| Markdown styling can make prompts easier to scan and structure while keeping source plain text | Styled Markdown must not drift into misleading WYSIWYG behavior |
| Structured mention metadata creates a path to richer agent integrations later | The initial implementation must maintain backward compatibility with string-only agent flows and therefore ignores mentions at runtime for now |
| CodeMirror keeps the feature focused and lightweight relative to Monaco | CodeMirror still adds new bundle size, styles, and maintenance overhead |
| Dynamic growth better matches modern chat UX and uses space more efficiently | Auto-grow inside a CSS grid needs careful height limits and overflow behavior |
| Inline `@` authoring is more natural than a separate file picker command | Inline suggestion UX must be carefully designed for keyboard navigation and accessibility |
| A dedicated `document-viewer/chat-input/` module keeps responsibilities clearer as the feature grows | Splitting `chat-input` from `form-editor` adds one more internal boundary to maintain inside `document-viewer/` |

## 4.1. Recommendation

The recommended direction is:

1. adopt **CodeMirror 6**
2. keep the editor **plain text and Markdown-source based**, not rich text
3. add **Markdown-aware visual styling** for common prompt syntax
4. implement **file mentions as structured inline metadata**
5. resolve suggestions through the **extension host**, not a browser-local file index
6. support **dynamic vertical growth** inside the existing form grid
7. preserve a **string-only execution contract** for all current agent execution paths in the first iteration
8. keep `mentions` as forward-compatible metadata for future uses not yet defined

This strikes the best balance between user experience and implementation risk.

My opinion is that **CodeMirror is the right default unless we already know the product will soon need full code-editor capabilities inside forms**. It is also a good fit if we want Markdown to feel nicer while still storing plain text. Monaco would only be worth the cost if `chat-input` is evolving into a much broader editor platform. A native textarea plus overlays is tempting for speed, but it will likely become brittle once we need reliable mention editing, cursor movement across mentions, auto-grow behavior, or future attachment types.

## 5. Acceptance Criteria

- [ ] `chat-input` no longer renders as a plain textarea for editable fields.
- [ ] Typing `@` inside an editable `chat-input` opens a file suggestion experience.
- [ ] Suggestion results are sourced from the workspace through the extension host.
- [ ] Selecting a suggestion inserts a readable inline file mention at the cursor position.
- [ ] Common Markdown patterns such as headings, emphasis, and inline code receive visual styling during authoring while preserving raw Markdown text.
- [ ] Submitted form data preserves both plain text content and structured file mention metadata.
- [ ] In the first iteration, agent execution consumes only the plain text `text` or `content` field.
- [ ] In the first iteration, structured `mentions` metadata is ignored by the runtime execution path without breaking prompt submission.
- [ ] Read-only `chat-input` fields continue to render without an interactive editor.
- [ ] Editable `chat-input` grows dynamically with content up to a bounded maximum height.
- [ ] The existing `vector-form` CSS grid layout expands naturally with the editor height and does not clip or overlap neighboring fields.
- [ ] The implementation is organized under `document-viewer/`, with a dedicated `document-viewer/chat-input/` module rather than being fully embedded inside `document-viewer/form-editor/`.
- [ ] Keyboard navigation for the suggestion surface supports arrow keys, enter, escape, and backspace behavior around mentions.
- [ ] The viewer degrades gracefully when search results fail or the host cannot resolve files.
- [ ] Tests cover mention insertion, mention deletion, Markdown styling, auto-grow behavior, payload serialization, and fallback plain text behavior.

## 6. Open Questions

- Should file mentions serialize as display text plus metadata, or as a hidden token model that is expanded only at submit time?
- Should the suggestion source include all workspace files or only governed-document-safe paths and selected source directories?
- Should mention insertion use relative workspace paths, document identifiers when available, or both?
- Should clicking a rendered mention reopen the target file or only keep it as prompt context?
- Should the first implementation support only file mentions, or reserve the model now for future entities such as documents, symbols, or agents?
- Which Markdown features should receive first-pass styling support: headings and emphasis only, or also lists, blockquotes, and fenced code blocks?
