---
id: rfc-00015-extension-owned-governed-document-preview-for-vs-code
type: rfc
code: "00015"
slug: extension-owned-governed-document-preview-for-vs-code
title: Extension-Owned Governed Document Preview for VS Code
description: Defines a Vector-owned WebviewPanel preview for governed documents in VS Code with markdown-it rendering, governed wikilink navigation, callouts, tables, and code presentation.
status: implemented
created: 2026-05-08
updated: 2026-05-08
authors: []
tags:
  - vscode
  - frontend
  - preview
  - markdown
related:
  - adr-00001-adopt-an-extension-owned-governed-document-preview-for-vs-code
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00020-implement-rfc-00014-vs-code-governed-documents-sidebar-extension
  - rfc-00013-runtime-doc-validation-and-authoring-crate
supersedes: []
superseded_by: null
aliases:
  - "RFC 00015: Extension-Owned Governed Document Preview for VS Code"
---

# RFC 00015: Extension-Owned Governed Document Preview for VS Code

## 1. Problem

ADR 00001 already decided that governed documents in VS Code must move away from native Markdown Preview toward a Vector-owned preview surface. The remaining gap is delivery: the repository does not yet define the exact preview contract, rendering scope, navigation behavior, or presentation rules for that extension-owned reader.

That gap is now concrete in four ways:

- RFC 00014 and task 00020 were executed against a native Markdown Preview path that ADR 00001 has now superseded for governed reading flows.
- The current temporary preview bridge renders governed wikilinks as styled pills in native Markdown Preview, but it does not establish the accepted long-term preview surface.
- Governed documents require rendering features that must be predictable and testable under Vector ownership, including callouts, governed wikilinks, fenced code blocks, inline code emphasis, and tables.
- The user flow for governed wikilink navigation must reopen the target inside the same reader surface instead of forking into unrelated editor or preview behavior.

If this remains unspecified, the extension will keep a split architecture where sidebar navigation is Vector-owned but governed reading semantics still depend on temporary native-preview behavior that ADR 00001 explicitly rejects.

## 2. Proposal

Create an extension-owned governed document preview in `frontend/vscode/vector` using a reusable VS Code `WebviewPanel`.

The preview is read-only and applies only to governed documents opened through Vector-owned flows:

- sidebar tree selection
- governed document open commands
- governed wikilink clicks originating inside the governed preview

This RFC defines `WebviewPanel` as the accepted preview surface for governed documents. The panel must be reused for navigation so clicking a governed wikilink replaces the current governed preview content in the same panel rather than opening a second reader by default.

### 2.1. Rendering pipeline

The preview must:

- parse source Markdown with `markdown-it`
- render inside a Vector-owned HTML document served by the extension
- preserve a narrow integration boundary between rendering logic and governed document discovery or resolution
- avoid any dependency on native Markdown Preview contribution points for governed reading

The preview must not attempt to become a general Markdown renderer for unrelated workspace files.

`markdown-it` must be treated as a two-stage pipeline:

- parse raw Markdown into tokens
- render those tokens into HTML through controlled renderer rules

This matters because the governed preview needs both syntax extension and presentation control. Features that introduce governed syntax should be implemented as `markdown-it` parsing rules or plugins. Features that only restyle already-supported Markdown output should prefer renderer-rule customization.

### 2.2. Governed content features

The initial governed preview must support these presentation features:

- governed wikilinks using the existing governed stem contract such as `[[rfc-00014-vs-code-governed-documents-sidebar-extension]]`
- callouts written in the governed Markdown form `> [!TYPE] Title`
- fenced code blocks
- inline code such as `` `30 Entities` ``
- Markdown tables

Support means the rendered output is structurally readable, visually distinct, and stable enough for governed documentation use. This RFC does not require full parity with Obsidian rendering semantics beyond the features listed above.

Expected `markdown-it` implementation model:

- governed wikilinks should be implemented through an inline parsing rule or inline-oriented plugin because `[[target]]` is custom inline syntax
- callouts should be implemented through a block-level plugin or block-token transformation because they originate from quoted block structure
- fenced code blocks, inline code, and tables should reuse native `markdown-it` parsing support first, then apply renderer customization only where governed styling requires it

The extension should prefer native CommonMark behavior when `markdown-it` already parses the feature correctly, instead of replacing built-in parsing with custom syntax handlers.

### 2.3. Wikilink behavior

Governed wikilinks must be resolved by the same governed lookup boundary already used by the sidebar flow.

Resolution contract:

1. Parse the wikilink target as a governed file stem.
2. Extract document type and code from that stem.
3. Resolve the target document within the governed repository layout.
4. Re-render the resolved target inside the same `WebviewPanel`.

The preview must invoke a VS Code command owned by the extension for wikilink navigation. The command is the control boundary between HTML click events and extension-side document resolution.

At the `markdown-it` layer, governed wikilinks should be parsed into dedicated output that preserves the governed target stem in a structured attribute, rather than emitting opaque HTML that loses resolution metadata. In practice this means the parser or plugin should capture the wikilink target during inline parsing, and the renderer should emit stable HTML attributes that the webview can bind to command dispatch.

The preview must not:

- use blind workspace-wide filename search as its primary resolution strategy
- hand off governed wikilink clicks to the native Markdown Preview
- open a new panel for every successful governed wikilink click by default

### 2.4. Visual contract

The preview should preserve the reading cues already introduced by the temporary native-preview bridge where they remain useful, but move them under extension ownership.

Required visual intent:

- governed wikilinks render as boxed white pills with strong contrast against surrounding text
- inline code renders as a visually distinct token-like chip rather than plain monospace text
- callouts render as block containers with a visible title treatment and type emphasis
- fenced code blocks render as dedicated blocks with clear separation from prose
- tables render with borders, spacing, and overflow behavior that remains readable inside the panel

This RFC defines visual behavior as a governed UX contract, not as a promise of pixel parity with the temporary native-preview stylesheet.

Where the syntax is already parsed by `markdown-it`, visual treatment should be applied through renderer rules or post-render CSS, not by inventing duplicate parsing logic. That includes:

- inline code emphasis
- fenced code block wrappers or classes
- table classes, wrappers, or overflow containers

This keeps parsing concerns separate from presentation concerns and reduces plugin fragility.

### 2.5. Panel lifecycle

The extension must maintain one reusable governed preview panel per VS Code window unless a future RFC deliberately expands the model.

Expected lifecycle:

- opening a governed document creates the panel if it does not exist
- opening another governed document reuses the same panel and replaces its content
- clicking a governed wikilink reuses the same panel and replaces its content
- closing the panel releases preview state cleanly

The extension may keep minimal state for the currently displayed document and panel identity, but it must not duplicate governed document metadata in a second long-lived cache when sidebar or shared resolution state can already provide it.

The implementation flow should be modeled around three extension responsibilities:

- subscription: observe the relevant VS Code events that should refresh or replace preview content
- provider: obtain the current governed document source and the metadata needed to resolve links or local assets
- renderer: transform the source with `markdown-it` and publish the resulting HTML into the `WebviewPanel`

This separation is preferred over mixing VS Code event handling, document lookup, and HTML generation in one command handler because each concern evolves independently and should be testable in isolation.

All preview implementation files live under `src/document-viewer/` inside the extension package. This folder is the accepted boundary for preview-specific code and is intentionally separate from `src/documentDiscovery.ts`, `src/governedDocumentProvider.ts`, and `src/wikilinkPlugin.ts`, which remain at the root of `src/` because they serve both the sidebar tree and the preview flows.

The module exposes a single public surface through `src/document-viewer/index.ts`. Callers outside the folder import only from this index. Files within `document-viewer/` may import each other directly.

Expected initial layout of `document-viewer/`:

```
src/document-viewer/
├── index.ts                       re-exports the public surface of the module
├── governedPreviewController.ts   WebviewPanel lifecycle, provider, and render coordination
└── previewHtml.ts                 HTML shell builder and escapeHtml utility
```

Later phases will extend this layout with additional files as rendering features are added:

```
src/document-viewer/
├── index.ts
├── governedPreviewController.ts
├── previewHtml.ts
├── markdownRenderer.ts            Phase B — markdown-it construction and renderer rules
├── wikilinkNavigation.ts          Phase C — wikilink click dispatch and panel navigation
└── calloutPlugin.ts               Phase D — governed callout block plugin
```

No file outside `document-viewer/` should import its internal files directly. All external consumers must go through `index.ts`.

Expected refresh triggers include:

- opening a governed document through a Vector command
- clicking a governed wikilink inside the preview
- changing the source document when the preview is tracking the currently open governed file

The RFC does not require full live-preview behavior for arbitrary editor navigation, but it does allow the implementation to subscribe to VS Code document-change events when the governed preview is bound to an active source document.

### 2.6. Integration boundary

This RFC does not approve reimplementing governed repository rules inside the webview renderer.

The preferred architecture is:

- governed document discovery and resolution remain in reusable extension-side modules
- the preview renderer receives resolved document content plus the minimum metadata needed for display and navigation wiring
- HTML emitted into the `WebviewPanel` remains presentation-focused

If additional adaptation is required between sidebar logic and preview logic, the extension should introduce a narrow internal adapter rather than duplicate repository governance rules in multiple places.

The preview implementation should keep these responsibilities separate:

- document resolution modules decide what file a governed wikilink points to
- `markdown-it` plugins and rules decide how Markdown syntax becomes structured HTML
- webview UI code decides how rendered HTML is styled and how click events are routed back to extension commands

When debugging new governed syntax, token inspection through `md.parse(source, env)` should be treated as the primary diagnostic step before changing renderer output or CSS.

The `WebviewPanel` integration must also respect VS Code webview constraints:

- the generated HTML should define a Content Security Policy appropriate for the resources the preview loads
- local extension resources such as CSS, scripts, or preview assets should be exposed through `webview.asWebviewUri(...)`
- preview state such as the current document identity, and optionally scroll continuity, may be preserved through webview state mechanisms when the implementation needs it

These are implementation constraints, not optional polish. A governed preview that ignores CSP or webview-safe resource resolution is architecturally incomplete.

### 2.7. Implementation guidance snippets

This RFC is normative first, but it may include small non-normative snippets when they clarify the intended extension pattern for implementation agents.

The purpose of those snippets is to guide structure, not to freeze exact code.

Illustrative integration shape (`document-viewer/governedPreviewController.ts`):

```ts
function renderGovernedPreview(source: string): string {
    const md = createGovernedMarkdownIt();
    return md.render(source);
}
```

```ts
function registerGovernedPreviewSubscriptions(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.workspace.onDidChangeTextDocument((event) => {
            // Refresh only when the preview is bound to this governed source document.
        }),
    );
}
```

```ts
function updatePreview(panel: vscode.WebviewPanel, documentText: string): void {
    const html = renderGovernedPreview(documentText);
    panel.webview.html = buildPreviewHtml(panel.webview, html);
}
```

Illustrative `markdown-it` extension shape (`document-viewer/markdownRenderer.ts`, Phase B):

```ts
function createGovernedMarkdownIt(): MarkdownIt {
    return markdownIt()
        .use(governedWikilinkPlugin)
        .use(governedCalloutPlugin);
}
```

```ts
md.renderer.rules.code_inline = (tokens, idx) => {
    const content = escapeHtml(tokens[idx]!.content);
    return `<code class="vector-inline-code">${content}</code>`;
};
```

```ts
md.renderer.rules.table_open = () => '<div class="vector-table-wrap"><table class="vector-table">';
md.renderer.rules.table_close = () => "</table></div>";
```

Snippets like these are appropriate in this RFC because they reduce ambiguity around:

- where `markdown-it` is instantiated
- where plugins are registered
- where renderer customization belongs
- where webview refresh logic belongs

They should remain short and structural. Full implementation detail belongs in the follow-up task or code.

### 2.8. Scope boundaries

In scope:

- extension-owned `WebviewPanel` preview
- `markdown-it` rendering
- governed wikilink navigation through an extension command
- callout rendering
- fenced code block rendering
- inline code emphasis
- table rendering
- same-panel navigation for governed preview flows

Out of scope:

- replacing VS Code's default Markdown editor for arbitrary workspace files
- editable rich-text or WYSIWYG behavior
- authoring commands for new governed documents
- reverse synchronization from preview scroll position to source editor
- full Obsidian plugin compatibility
- arbitrary external link interception beyond normal safe browser behavior

## 3. Alternatives Considered

- **Continue extending native Markdown Preview:** Discarded because ADR 00001 already rejects that path for governed reading flows, and the existing bridge is only a temporary compatibility step.
- **Custom editor registration instead of `WebviewPanel`:** Discarded for now because a custom editor raises lifecycle and registration complexity without materially improving the first accepted governed-reading workflow.
- **Open a new preview panel for each navigation event:** Discarded because it fragments the governed reading flow, increases panel sprawl, and weakens the meaning of "open in the same reader."

## 4. Tradeoffs

| Pro                                                                                                          | Con                                                                                                   |
|--------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------|
| Full control over rendering and governed navigation behavior.                                                | Vector now owns preview rendering, styling, and lifecycle maintenance.                                |
| Same-panel wikilink navigation creates a deterministic reading flow for governed documents.                  | The implementation must manage explicit state transitions that native preview handled implicitly.     |
| `markdown-it` keeps the renderer predictable and extensible for governed syntax.                             | Callout support and richer Markdown presentation require custom plugin or renderer work.              |
| The preview can preserve important visual affordances such as wikilink pills and highlighted inline code.    | Visual polish becomes part of extension quality, not a free benefit from VS Code defaults.            |
| The architecture keeps governed document resolution inside extension-owned boundaries instead of HTML hacks. | Messaging between webview content and extension commands adds bridge complexity and new test surface. |

## 5. Acceptance Criteria

- [ ] `frontend/vscode/vector` exposes a Vector-owned governed document preview implemented with a reusable `WebviewPanel`.
- [ ] Opening a governed document through a Vector flow renders it in the extension-owned preview rather than relying on native Markdown Preview hooks.
- [ ] The preview uses `markdown-it` as its rendering pipeline.
- [ ] The implementation distinguishes parsing concerns from rendering concerns, using `markdown-it` plugins or rules for governed syntax and renderer customization for governed presentation.
- [ ] The implementation separates subscription, provider, and renderer responsibilities for preview lifecycle management.
- [ ] Governed wikilinks render as interactive pill-like elements in the preview.
- [ ] Clicking a governed wikilink invokes an extension-owned VS Code command.
- [ ] The command resolves the target document by governed stem, using the same governed lookup boundary as sidebar navigation.
- [ ] A successful governed wikilink navigation re-renders the target inside the same preview panel.
- [ ] Callouts written as `> [!TYPE] Title` render as distinct callout blocks.
- [ ] Inline code renders with visible emphasis beyond plain body text.
- [ ] Fenced code blocks render as dedicated code sections.
- [ ] Markdown tables render in a readable layout inside the preview.
- [ ] Webview resources and security constraints are handled through VS Code-compatible CSP and `asWebviewUri(...)` usage where applicable.
- [ ] The preview remains limited to governed documents opened through Vector-owned flows.
- [ ] The extension does not treat native Markdown Preview integration as the primary governed reading path.

## 6. Open Questions

- Should the preview keep lightweight back and forward navigation history inside the same panel, or should navigation remain strictly last-document-wins for the first iteration?
- Should callout styling be type-specific from the first delivery, or is one generic callout container acceptable until a later RFC refines visual taxonomy?
- Should the preview open the source Markdown document beside the panel on first open, or remain preview-only unless the user explicitly requests source?
