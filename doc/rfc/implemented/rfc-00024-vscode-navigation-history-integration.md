---
id: rfc-00024-vscode-navigation-history-integration
type: rfc
code: "00024"
slug: vscode-navigation-history-integration
title: VSCode Navigation History Integration for Extension File Opens
description: Replace the custom file-open command in the Vector extension with the VSCode-native document open API so that workspace navigation buttons (back/forward) work correctly.
status: implemented
created: 2026-05-18
updated: 2026-05-18
authors: []
tags:
  - vscode
  - extension
  - user-experience
related: []
supersedes: []
superseded_by: null
aliases:
  - "RFC 00024: VSCode Navigation History Integration for Extension File Opens"
---

# RFC 00024: VSCode Navigation History Integration for Extension File Opens

## 1. Problem

The Vector VSCode extension opens files using a custom command instead of the
VSCode-native document open mechanism (`vscode.workspace.openTextDocument` +
`vscode.window.showTextDocument`, or the built-in `vscode.open` command).

VSCode only pushes an entry onto the editor navigation history (the stack that
powers the **Go Back** / **Go Forward** toolbar buttons and their default
key bindings `Alt+Left` / `Alt+Right`) when a document is opened through its
own API surface. Because the extension bypasses that surface, every file
navigation triggered by the extension is invisible to VSCode's history
mechanism.

**Concrete pain:**

- After clicking a link or reference inside the extension, the user cannot
  press **Go Back** to return to where they were.
- The workspace navigation buttons in the top-left of the editor remain greyed
  out (disabled) after an extension-driven navigation.
- The experience is inconsistent with every other VSCode feature and extension
  the user works with.

## 2. Proposal

Keep the existing custom file-open command as the public interface. Replace
only its internal implementation with the VSCode-native mechanism so that all
existing call sites — inside the extension and any external callers (user
keybindings, other extensions referencing the command ID) — require no
changes.

### 2a. Internal implementation — `openTextDocument` + `showTextDocument`

The command's implementation body becomes:

```typescript
const doc = await vscode.workspace.openTextDocument(uri);
await vscode.window.showTextDocument(doc, {
  preview: false,          // open as a persistent tab, not a preview
  selection: targetRange,  // optional: jump to a specific range
});
```

This gives full control over column placement, selection range, and preview
vs. permanent tab, while routing through the VSCode API surface that pushes
entries onto the navigation history stack.

### 2b. Secondary mechanism — `vscode.open` built-in command

```typescript
await vscode.commands.executeCommand('vscode.open', uri, {
  selection: targetRange,  // optional
});
```

Available as a fallback for any new call sites that need no column or preview
control. **Not used to replace the existing custom command.**

### Why keep the command instead of removing it

The original approach replaced all call sites and deleted the custom command.
That is cleaner in isolation but carries two risks:

1. External callers (user keybindings, other extensions) break silently if
   the command ID disappears — this is the open question raised in Section 6.
2. Every internal call site must be updated and re-tested individually.

By keeping the command as a stable, intentional shim around option 2a, we
fix the root cause without touching the public surface. The indirection layer
is now load-bearing, not accidental.

### Migration steps

1. Open the custom command's registration and replace its implementation body
   with `openTextDocument` + `showTextDocument`, mapping any existing options
   (column, selection, preview flag) to `TextDocumentShowOptions`.
2. Do **not** remove the command registration — it is the stable public API.
3. Smoke-test: invoke the command via the extension UI and via the command
   palette, then verify **Go Back** (`Alt+Left`) returns to the previous
   editor location.

## 3. Alternatives Considered

- **Keep the custom command, emit a navigation event manually:** VSCode does
  not expose a public API to push entries onto the navigation stack
  programmatically. Internal APIs (`_workbench.action.navigateBack`) are
  unstable and break across releases. Discarded.

- **Wrap the custom command to call `showTextDocument` internally:** This is
  functionally identical to the proposal but keeps an unnecessary indirection
  layer. Discarded in favor of removing the custom command entirely.

- **Use `vscode.commands.executeCommand('vscode.open', ...)` everywhere:**
  Simpler, but loses the ability to control column, preview mode, and
  selection range at call sites that need it. Discarded as the sole strategy;
  retained as the secondary mechanism for simple call sites.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| Navigation back/forward works as users expect | Requires auditing all existing call sites |
| No private/unstable API usage | Minor refactor effort across the extension |
| Consistent with every other VSCode extension | `showTextDocument` is async — callers must be updated to `await` properly |
| `TextDocumentShowOptions` gives full control over column, preview, selection | Preview-tab behavior differs slightly from a permanent-tab open (must set `preview: false` where needed) |

## 5. Acceptance Criteria

- [ ] Every file opened by the extension via the old custom command is replaced
      with `openTextDocument` + `showTextDocument` or `vscode.open`.
- [ ] The custom open command is removed from the extension's command
      registration if it has no remaining callers outside the extension itself.
- [ ] After any extension-driven navigation, the VSCode **Go Back** button
      (and `Alt+Left`) returns the user to the previous editor location.
- [ ] The workspace navigation buttons are no longer permanently disabled after
      extension use.
- [ ] Existing behavior (correct file opened, cursor at correct position,
      correct editor column) is preserved at all call sites.
- [ ] No use of internal or unstable VSCode APIs.

## 6. Open Questions

- Are there call sites inside the extension that intentionally suppress VSCode
  history (e.g., silent background pre-fetches)? Those should stay on a
  non-display path and must not call `showTextDocument`.
- Should we standardize on `preview: false` everywhere, or do some call sites
  benefit from preview-tab behavior (single-click explore)?
- Is the custom command also called from outside the extension (e.g., from
  user key bindings or other extensions)? If so, the command must be kept as a
  thin shim that delegates to `showTextDocument` rather than being deleted.
