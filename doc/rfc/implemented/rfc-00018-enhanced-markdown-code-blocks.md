---
id: rfc-00018-enhanced-markdown-code-blocks
type: rfc
code: "00018"
slug: enhanced-markdown-code-blocks
title: "Enhanced Markdown Code Blocks: Syntax Highlighting and Custom Components"
description: Extend the markdown viewer to support syntax highlighting on standard code blocks and to render custom component blocks organized into two VSCode extension modules — form_editor (vector-form) and document_actions (vector-agent-button, vector-agent-action, vector-open-doc) — with document instantiation and agent execution via CLI.
status: implemented
created: 2026-05-11
updated: 2026-05-11
authors:
  - fernandojerez
tags:
  - markdown
  - viewer
  - syntax-highlight
  - agents
  - components
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00018: Enhanced Markdown Code Blocks"
---

# RFC 00018: Enhanced Markdown Code Blocks: Syntax Highlighting and Custom Components

## 1. Problem

The markdown viewer currently renders all fenced code blocks as plain text. This creates two distinct gaps:

1. **No syntax highlighting** — blocks that declare a language (e.g., ` ```rust `) are displayed without color or token differentiation, reducing readability for technical documents.
2. **No custom block components** — the project needs to embed interactive UI elements (forms, agent-trigger buttons, document launchers) directly inside markdown documents. There is no mechanism today to extend the renderer for custom fence tags.

## 2. Proposal

### 2.1 Syntax Highlighting for Standard Code Blocks

When a fenced code block declares a language identifier (e.g., ` ```rust `, ` ```typescript `), the viewer applies token-based syntax highlighting to the rendered output.

- Highlighting is applied only when a language identifier is present; plain ` ``` ` blocks are unchanged.
- Language detection is case-insensitive.
- A library such as `highlight.js` or `shiki` handles tokenization and theming.

---

### 2.2 Document Variable Substitution (`#{}`)

`#{}` is a **document instantiation mechanism**, not a field type. Any document may contain `#{variable}` placeholders. When the document is opened — either directly or via `vector-open-doc` — the caller supplies an `input` map and all `#{variable}` occurrences in the rendered content are replaced with the corresponding values before display.

This substitution happens once at open time. The resulting document is a concrete instance; no `#{}` tokens remain visible in the rendered output.

---

### 2.3 Custom Component Code Blocks

Blocks with one of the following fence identifiers are **not** rendered as code — they are parsed and rendered as interactive UI components.

#### 2.3.1 `vector-form`

Renders an input form. The fence body uses a lightweight key-value DSL (not YAML). Each line declares a field:

```
variable = <type>(label)
```

**Supported field types:**

| Syntax | Rendered as |
|---|---|
| `key = input("Label")` | Single-line text input |
| `key = chat-input("Label")` | Multi-line chat-style textarea |

If the document was opened with `vector-open-doc`, any `#{variable}` placeholders in the block body were already substituted before the form is rendered; a literal value in place of `#{value}` is treated as a pre-filled, read-only field.

A document may contain any number of `vector-form` blocks. Forms do not submit independently. When any action in the document is triggered, the viewer collects field values from **all** `vector-form` blocks in the document and merges them in document order — a field defined in a later form overwrites the same key from an earlier form.

#### 2.3.2 `vector-agent-button`

Renders a prominent action button. The fence body is YAML:

```yaml
label: Execute
profile: create-doc
prompt: prompt-00003-create-doc
input:
  phase: Phase A
  document-type: rfc
  language: rust
```

**Fields:**

| Field | Required | Description |
|---|---|---|
| `label` | yes | Text displayed on the button |
| `profile` | yes | Agent profile key from `.vector/agents.yaml` |
| `prompt` | yes | Prompt document identifier to resolve |
| `input` | no | Static key-value pairs injected into prompt `#{}` variables |

When clicked, the button presents the agents available in the resolved profile (see §2.4). Selecting an agent executes the flow (see §2.6).

#### 2.3.3 `vector-agent-action`

Renders a flat, inline action link (visually lighter than a button). Fence body is identical in structure to `vector-agent-button`:

```yaml
label: Create document
profile: create-doc
prompt: prompt-00003-create-doc
input:
  phase: Phase A
  document-type: rfc
  language: rust
```

Behavior on click is identical to `vector-agent-button`; the only difference is visual weight.

#### 2.3.4 `vector-open-doc`

Opens a target document in the viewer and instantiates it by substituting `#{}` placeholders with the provided `input` values. The fence body is YAML:

```yaml
label: Create document
doc: form-00001-create-doc
input:
  document-type: rfc
  language: rust
```

**Fields:**

| Field | Required | Description |
|---|---|---|
| `label` | yes | Text displayed on the trigger link or button |
| `doc` | yes | Document identifier to open (resolved via `find_doc` logic) |
| `input` | no | Key-value pairs substituted into `#{}` placeholders of the target document |

On click, the viewer opens the target document and performs variable substitution before rendering. The document itself is never modified on disk; substitution is view-only.

---

### 2.4 Agent Profile Resolution (shared in `document_actions/`)

Agent profiles and their CLI definitions live in `.vector/agents.yaml`:

```yaml
agents:
  claude:
    type: cli
    command: claude
  codex:
    type: cli
    command: codex

profiles:
  create-doc: [claude, codex]
  code: [claude, codex, opencode, gemini]
```

When the user activates a button or action, the viewer resolves the profile to its agent list and presents each available agent as a selectable option (e.g., a dropdown or button group inline with the trigger). Agents whose command is not found in `PATH` are shown as disabled with a "not installed" indicator.

---

### 2.5 VSCode Extension Module Structure

The custom block components are implemented inside the Vector VSCode extension, split into two modules:

#### `form_editor/`

Owns everything related to form rendering and the form DSL:

- Parser for the `key = <type>(label)` DSL.
- Renderer for `vector-form` blocks (field components: `input`, `chat-input`).
- State management for collecting form field values.

#### `document_actions/`

Owns all blocks that trigger an action from a document:

| Block | Responsibility |
|---|---|
| `vector-agent-button` | Prominent button — resolves profile, renders agent picker, triggers CLI execution |
| `vector-agent-action` | Flat inline action link — same logic as button, different visual weight |
| `vector-open-doc` | Opens a target document and instantiates it via `#{}` substitution |

The agent execution pipeline (§2.6), profile resolution, temp file management, and terminal spawning are shared utilities within `document_actions/`.

---

### 2.6 Agent Execution (CLI Type)

For `type: cli` agents, execution proceeds as follows:

1. **Resolve the prompt file** — locate the document matching the `prompt` identifier (e.g., `prompt-00003-create-doc`) under `doc/`, using the same resolution strategy as `find_doc`.
2. **Merge variables** — build the final variable map using the following priority (highest to lowest):
   1. Form fields collected from all `vector-form` blocks in document order — later forms override earlier ones for the same key.
   2. Static `input` values declared in the action block — lowest priority, overridden by any form field with the same key.
3. **Substitute variables** — replace all `#{variable}` occurrences in the prompt file content with the merged values. Variables with no resolved value are left as empty strings and a warning is surfaced to the user.
4. **Write temp file** — write the resolved prompt string to a uniquely named temporary file (e.g., `%TEMP%\vector-prompt-<uuid>.txt`).
5. **Spawn terminal** — open a named VSCode terminal (`Vector: <agent> — <label>`) and run:
   ```
   <agent.command> < <tmp_file>
   ```
6. **Keep terminal open** — the terminal persists so the user can continue refining the result interactively with the agent.
7. **Cleanup** — the temp file is deleted when the terminal session ends or the extension deactivates.

---

## 3. Alternatives Considered

- **Embed forms and buttons as raw HTML inside markdown:** Discarded because the project's markdown is stored in a vault and must remain portable. Raw HTML creates renderer lock-in and is harder to parse for future tooling.
- **Separate side-panel UI for agent triggers:** Discarded because coupling the trigger to the document context (visible inline) makes the authoring workflow faster and more discoverable.
- **Mutate the document on disk when opening with `vector-open-doc`:** Discarded because the source document is a reusable template; instantiation must be view-only so the same document can be opened multiple times with different inputs.
- **WebSocket / HTTP agent integration:** Deferred — CLI is the only supported agent type in this RFC. A future RFC can extend `agents.yaml` with `type: http` or `type: mcp` entries without breaking this design.

## 4. Tradeoffs

| Pro | Con |
|---|---|
| Syntax highlighting is a zero-configuration improvement for all existing docs | Adds a tokenizer dependency (highlight.js / shiki) to the viewer bundle |
| Custom blocks are declared inline — no separate config file per document | Custom fence identifiers are non-standard; tools outside this project ignore them silently |
| `#{}` substitution is view-only — source documents double as reusable templates | Substitution state is ephemeral; there is no history of which values were used to open a document |
| Agent profile abstraction means switching agents requires no document edits | Profile resolution fails silently if `.vector/agents.yaml` is absent or malformed |
| CLI stdin approach works with any agent that reads from stdin | Agents that do not support stdin piping require a different invocation strategy |
| Spawned terminal stays open for interactive refinement | Multiple clicks spawn multiple terminals — requires a guard or "already running" UX state |

## 5. Acceptance Criteria

**Syntax highlighting**
- [ ] Fenced code blocks with a recognized language identifier render with syntax highlighting.
- [ ] Fenced code blocks without a language identifier are unaffected.

**`form_editor` module**
- [ ] `vector-form` blocks render as a form with `input` (single-line) and `chat-input` (multi-line) field types.
- [ ] `vector-form` fields whose value was pre-substituted via `#{}` are rendered as read-only.
- [ ] A document may contain multiple `vector-form` blocks; all are collected when any action is triggered.
- [ ] Form fields are merged in document order — a later form's field overwrites the same key from an earlier form.
- [ ] The merged form values take precedence over the action block's static `input` for the same key.

**`document_actions` module**
- [ ] `vector-agent-button` blocks render as a styled button displaying the configured `label`.
- [ ] `vector-agent-action` blocks render as a flat action link displaying the configured `label`.
- [ ] `vector-open-doc` blocks render as a trigger that opens the target document.
- [ ] Opening via `vector-open-doc` substitutes all `#{}` placeholders with the provided `input` values before rendering.
- [ ] The source document is never modified on disk by a `vector-open-doc` operation.
- [ ] Activating a button or action resolves the profile and shows the available agents.
- [ ] Agents not found in `PATH` are shown as disabled.
- [ ] Clicking an available agent resolves the prompt, merges variables (form fields override block `input`, later forms override earlier ones), and writes a temp file.
- [ ] A named VSCode terminal is spawned running `<command> < <tmp_file>`.
- [ ] The terminal remains open after the agent's initial response.
- [ ] Temp files are deleted on terminal close or extension deactivation.
- [ ] Variables in the prompt with no resolved value produce a warning visible to the user.
- [ ] `.vector/agents.yaml` parse errors surface a user-visible error instead of silently failing.

## 6. Open Questions

- **Multi-terminal guard:** If the user clicks the same button twice, should the second click reuse the existing terminal, focus it, or spawn a new one? Recommendation: focus the existing terminal to avoid duplicate sessions.
- **Stdin vs. flag invocation:** Some CLI agents may prefer `--prompt <file>` over stdin piping. Should `agents.yaml` support an optional `input_mode: stdin | file` per agent entry to handle both cases?
- **`vector-open-doc` navigation model:** Does opening a document replace the current view (like a link) or open in a new tab/panel? Needs UX decision before implementation.
