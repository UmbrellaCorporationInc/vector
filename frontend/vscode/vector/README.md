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
- Governed wikilink (both workspace-local `[[doc-id]]` and package-qualified `[[package/doc-id]]`) and frontmatter-link navigation inside the same preview panel
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

## 8. Instruction Capability

The extension communicates prompt content to agents through a UUID-scoped instruction file managed entirely by the extension. This replaces the former `<file>` path handoff.

### 8.1 Configuration contract

`.vector/agents.yaml` must declare a top-level `instructions-dir` property:

```yaml
instructions-dir: "${system-temp}/vector/instructions"
# Optional: override the default launch prompt (see section 8.7).
# prompt: 'Using Vector MCP, call get_instruction with id "<instruction-id>" and execute the returned instructions.'
agents:
    claude:
        type: cli
        command: claude '<instruction>'
```

`${system-temp}` is the only supported variable. TypeScript resolves it with `os.tmpdir()`; the MCP counterpart resolves it with `std::env::temp_dir()`. The value must begin with `${system-temp}`, must not contain `..` or unsupported variable expressions, and must not resolve outside the system temporary directory.

This is a **breaking change**. Configurations that use the old `<file>` or `<instruction-id>` placeholders, or that omit `instructions-dir`, fail validation with an actionable error toast and prevent terminal creation. There is no compatibility fallback.

### 8.2 Distinct configuration responsibilities

| File                      | Responsibility                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.agents/mcp_config.json` | Declares how the MCP server (`mcp-vector`) is started — command, arguments, and transport type. Contains no agent-execution or instruction-directory configuration. |
| `.vector/agents.yaml`     | Configures agents, profiles, and the `instructions-dir` where instruction files are written. This file is the source of truth for the instruction lifecycle.        |

### 8.3 Instruction lifecycle

For each agent invocation:

1. The extension resolves the prompt and substitutes all declared input values.
2. `.vector/agents.yaml` is loaded and validated.
3. The configured instruction directory is resolved.
4. A cryptographically random UUID is generated with `crypto.randomUUID()`.
5. The instruction directory is created when necessary.
6. A file named `vector-instruction-<canonical-uuid>.txt` is created exclusively with UTF-8 content and restrictive POSIX permissions (`0o600`) where supported (non-Windows).
7. The frontend-owned MCP directive is rendered with the UUID and substituted for `<instruction>` as one shell-safe argument. No second interpolation pass occurs.
8. The terminal is created with the active workspace root as explicit `cwd`.
9. The instruction file path is associated with the terminal for cleanup.

**Retry behaviour:** The MCP `get_instruction` tool does not delete the instruction file after a successful read. Repeated reads succeed for the full terminal lifetime, making agent retries safe.

**Cleanup:** Instruction files are deleted when the associated terminal closes, when the extension deactivates, or immediately after a launch failure after the file has been created. On activation, the extension removes only regular files matching the exact `vector-instruction-<uuid>.txt` filename shape that are older than 24 hours; it does not follow links, recurse, or delete unrelated files.

**Cleanup failures** are written to a local VS Code diagnostic output channel only. No telemetry is collected or emitted.

**Size bound:** The MCP `get_instruction` tool enforces a fixed, non-configurable 1 MiB limit. Instructions larger than 1 MiB are rejected with an actionable error; they must be decomposed.

### 8.4 Platform-specific filesystem guarantees

| Platform    | Guarantee                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows** | `get_instruction` opens the target with `FILE_FLAG_OPEN_REPARSE_POINT` to avoid reparse-point redirection, inspects handle attributes via `GetFileInformationByHandle` to reject reparse points and directories, and validates the opened handle's canonical location with `GetFinalPathNameByHandleW`. Hard links are not detectable with these APIs alone; creating one across the boundary requires elevated privileges under default Windows policies. |
| **Unix**    | `symlink_metadata` is used to detect and reject symbolic links before opening. A TOCTOU window exists between the metadata call and the open; `O_NOFOLLOW`-based elimination would require `libc`/`nix`. This is the strongest portable protection available in `std`.                                                                                                                                                                                     |

### 8.5 Workspace scope

The extension supports one governed project per VS Code window. The `workspaceRoot` selected during extension activation is the authoritative project root for agent actions, terminal creation, and instruction directory resolution. Mapping multiple governed projects inside one multi-root workspace is explicitly deferred and is not supported by this implementation.

### 8.6 Cross-language test matrix

TypeScript and Rust independently cover the same configuration validation cases using table-driven tests. Shared cross-language fixture infrastructure is intentionally deferred to a separate governed proposal and is not part of this implementation.

### 8.7 Configurable launch prompt

The optional root-level `prompt` field in `.vector/agents.yaml` controls the message sent to the agent terminal at launch.

**Built-in default** (used when `prompt` is absent):

```
Using Vector MCP, call get_instruction with id "<instruction-id>" and execute the returned instructions.
```

The default references `Vector MCP` and never uses the word `server`.

**Configured prompt** — when `prompt` is present, its value overrides the built-in default:

```yaml
prompt: 'Using Vector MCP, call get_instruction with id "<instruction-id>" and execute the returned instructions.'
```

**Placeholder substitution** — every `<instruction-id>` token in the resolved prompt is replaced with the UUID generated for that launch. This applies to both the built-in default and any configured prompt. A configured prompt that omits the placeholder is sent verbatim; it cannot reference the generated instruction UUID.

**Validation** — `prompt` must be a string. A non-string value (number, boolean, mapping, …) fails validation with an actionable error toast before any terminal is created.

**Backward compatibility** — existing `agents.yaml` files that omit `prompt` continue to launch agents using the built-in default without any modification.

## 9. Changelog

### 1.5.0

- **Instruction Capability (Breaking)** — Replaces the `<file>` path handoff with a UUID-scoped instruction file and the `<instruction>` placeholder. `.vector/agents.yaml` must declare `instructions-dir: "${system-temp}/vector/instructions"`. All agent commands must use `<instruction>` and must not use the obsolete `<file>` or `<instruction-id>` placeholders. The MCP server exposes a new `get_instruction` tool that reads the instruction file by UUID without exposing paths to the caller. See section 8 for the complete contract, platform guarantees, and migration guide.
- **Configurable Launch Prompt** — `.vector/agents.yaml` now accepts an optional root-level `prompt` string that overrides the built-in default launch message. The `<instruction-id>` placeholder in the configured or default prompt is replaced with the UUID of each generated instruction at launch time. Existing configurations that omit `prompt` continue to use the built-in default unchanged. See section 8.7.

### 1.4.5

- **Package-Qualified Wikilinks (`[[package/doc-id]]`)** — Resolve and open governed documents from synchronized package locations under `.vector-database/packages/`.
- **Sync Packages Command** — Adds a "Sync Packages" action in the tree view title menu that executes `vector-database package sync` in a VS Code terminal.
- **Identifier Parsing** — Centralized document identifier parsing supporting both workspace-local (`doc-id`) and package-qualified (`package/doc-id`) forms.

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

## 10. Non-Goals and Future Work

- Rich-text or WYSIWYG editing is intentionally out of scope.
- Full live preview for arbitrary editor navigation is not required.
- Additional preview history or richer reader interactions should come from follow-up RFCs.
