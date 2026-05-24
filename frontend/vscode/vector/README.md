# `vector` VS Code Extension

## 1. Objective

Exposes governed project documents through a dedicated VS Code sidebar and a Vector-owned governed preview panel. Consumers are developers working inside a Vector-governed repository who need editor-native navigation and deterministic read-only rendering for documents under `doc/`.

## 2. Boundaries

### In scope

- Dashboard discovery from `.vector/dashboards/` and root-level sidebar navigation
- Dedicated dashboard viewer with layout-aware section filtering and click-through navigation
- Governed document discovery and sidebar navigation per document type
- Per-type `Search` and `Refresh` view actions, plus native `Collapse All`
- Command-driven filter/list flows without a dedicated toolbar button
- Opening governed documents in a reusable Vector-owned `WebviewPanel`
- Governed wikilink and frontmatter-link navigation inside the same preview panel
- Governed Markdown rendering with `markdown-it`, callouts, tables, and code presentation
- Syntax highlighting for fenced code blocks via `highlight.js`
- `vector-form` blocks: inline forms with `input` (single-line) fields and a document-viewer-scoped `chat-input` editor for multi-line prompts, file mentions, and Markdown-aware authoring cues
- `vector-open-doc` blocks: open a target document with `#{}` variable substitution (view-only)
- `vector-agent-button` and `vector-agent-action` blocks: trigger CLI agents via spawned VSCode terminals
- `vector-agent-inline-action` blocks: trigger CLI agents through an inline overlay that collects extra user context before spawning; supports a configurable `prompt-field` key to control the field name injected into the agent payload (defaults to `prompt-message`)

### Out of scope

- Authoring or editing governed documents from the sidebar
- Frontmatter mutation or inline metadata editing
- Non-governed Markdown discovery
- Custom-editor replacement for arbitrary workspace Markdown files
- Obsidian-specific UI behavior

### Dependencies

| Dependency                        | Role                                                   |
| --------------------------------- | ------------------------------------------------------ |
| VS Code extension API (`^1.90.0`) | View containers, tree views, commands, `WebviewPanel`  |
| CodeMirror 6                      | Plain-text `chat-input` editing, selection, and sizing |
| `markdown-it`                     | Governed Markdown parsing and renderer customization   |
| `highlight.js`                    | Client-side syntax highlighting for fenced code blocks |
| `js-yaml`                         | YAML parsing for `vector-open-doc` and agent blocks    |

## 3. Local Development

**Prerequisites:** Node.js ≥ 20, pnpm 10.33.3, VS Code.

```sh
cd frontend/vscode/vector
pnpm install
pnpm run compile
```

Open the `frontend/vscode/vector/` folder in VS Code and press **F5** to launch the extension in a development host.

Run tests:

```sh
pnpm test
```

## 4. Packaging

Build the `.vsix` package:

```sh
cd frontend/vscode/vector
pnpm run package
```

This produces a `vector-<version>.vsix` file in the package directory.

To install the packaged extension locally in VS Code:

```sh
code --install-extension vector-<version>.vsix
```

Or open VS Code → Extensions → `...` → **Install from VSIX…** and select the generated file.

> **Note:** Runtime-imported packages must be declared under `"dependencies"` so `vsce` can bundle them into the generated `.vsix`. This extension uses `js-yaml` at activation time, so packaging must include runtime dependencies.

## 5. Activation

The extension activates when the opened workspace contains `.vector/document-types.yaml`. This file is the source of truth for governed document types and their layout configuration.

## 6. Preview Architecture

Governed reading flows are extension-owned:

- The tree view and Vector commands open governed documents in one reusable `WebviewPanel`.
- The preview is read-only and scoped to governed documents opened through Vector flows.
- Wikilinks and frontmatter document references resolve through the same governed lookup boundary used by the sidebar.
- Preview resources are loaded through webview-safe local URIs with a restrictive Content Security Policy.
- Interactive prompt editing remains scoped to `document-viewer/`; the dedicated `chat-input/`, `form-editor/`, and `document-actions/` modules do not introduce a top-level editor platform.
- Editable `chat-input` fields run on a dedicated CodeMirror 6 runtime in `media/chat-input-runtime.js`, so prompt state, selection, mention decorations, and auto-grow measurement no longer depend on `contenteditable` DOM rewrites in `preview.js`.
- `chat-input` submissions preserve structured file-mention metadata for future use while the current agent execution path continues to consume only plain text content.

## 7. Naming Contracts

- Hash-brace substitution variables accepted by the extension are kebab-case only. Valid examples include `#{doc-type}`, `#{file-path}`, and `#{document-type}`.
- Underscore-containing placeholders such as `#{doc_type}` or `#{document_type}` are intentionally left unresolved and are treated as invalid contract usage.
- `.vector/*.yaml` schema field names are also kebab-case only. Extension-side YAML readers reject invalid schema fields defensively, while repository-wide `runtime-doc validate` remains the authoritative failure path.

## 8. Changelog

### 1.4.2

- **`vector-agent-inline-action` blocks** — new fence variant that renders an inline overlay collecting extra user context before spawning an agent. An optional `prompt-field` key controls the name of the field injected into the agent payload; omitting it falls back to the existing `prompt-message` default.

### 1.4.1

- **Robust Agent Availability Check** — Fixed `isCommandInPath` to spawn a login shell subprocess (`pwsh` on Windows, `sh` otherwise) to check command availability in the user's full shell path, supporting user-scoped package managers.
- **Improved QuickPick Feedback** — Replaced silent returns with informative error messages when selecting an unavailable agent from the quick pick list.

### 1.2.22

- **Navigation History Integration** — Replaced custom file-open mechanisms with native VS Code document APIs. Workspace navigation buttons (Go Back / Go Forward) and their associated keybindings now work correctly after clicking links or navigating via the sidebar.
- **Decommissioned Legacy Preview Controller** — Removed the custom `GovernedPreviewController` in favor of standard VS Code tab and editor management.

### 1.1.22

- Editable `chat-input` now runs on a dedicated CodeMirror 6 runtime instead of the legacy `contenteditable` plus DOM-rewrite loop.
- The extension continues to submit plain text for first-iteration agent execution while preserving structured mention metadata for future integrations.

### 0.9.18

- **Syntax highlighting** — fenced code blocks with a language identifier are highlighted client-side via `highlight.js`. Plain ` ``` ` blocks are unaffected.
- **`vector-form` blocks** — inline forms inside governed documents. Supports `input` (single-line) and `chat-input` fields. Editable `chat-input` uses a document-viewer-scoped editor with extension-backed `@` file mentions, Markdown-aware styling, and bounded auto-grow behavior; fields pre-filled via `#{}` substitution remain read-only. All forms in a document are collected when any action is triggered; later fields override earlier ones for the same key.
- **`vector-open-doc` blocks** — trigger link that opens a target document in the preview panel and performs `#{}` variable substitution before rendering. The source document is never modified on disk.
- **`vector-agent-button` and `vector-agent-action` blocks** — trigger CLI agents defined in `.vector/agents.yaml`. On click, the available agents in the configured profile are presented via VSCode QuickPick. Selecting an agent resolves the prompt file, merges form fields with block-level `input` (form overrides static values), writes a temp file, replaces every `<file>` placeholder in the configured agent command with that temp file path, and spawns a named VSCode terminal running the resolved command. Unresolved `#{}` variables produce a warning. Temp files are cleaned up on terminal close or extension deactivation.

## 9. Non-Goals and Future Work

- Rich-text or WYSIWYG editing is intentionally out of scope.
- Full live preview for arbitrary editor navigation is not required.
- Additional preview history or richer reader interactions should come from follow-up RFCs.
