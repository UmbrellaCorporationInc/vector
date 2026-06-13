---
id: task-00067-update-mcp-tool-patch-doc
type: task
code: "00067"
slug: update-mcp-tool-patch-doc
title: Update MCP patch_doc Documentation Guidance
description: Clarify MCP patch_doc documentation so unified diff examples use 1-based hunk indices and apply_patch is presented as the recommended agent-facing format.
status: todo
created: 2026-06-13
updated: 2026-06-13
tags:
  - mcp
  - patching
  - documentation
related:
  - rfc-00037-extend-patch-doc-formats
supersedes: []
superseded_by: null
---

# Task 00067: Update MCP patch_doc Documentation Guidance

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: task-00067-update-mcp-tool-patch-doc
```

## 1. Prime Directive

> [!Prime Directive]
> Eliminate documentation ambiguity in the MCP `patch_doc` contract so agents stop learning the wrong unified-diff indexing rule and stop treating unified diff as the default authoring path.

This task refines the documentation around [[rfc-00037-extend-patch-doc-formats]]. The intended scope is documentation-only in the MCP layer unless the current generated tool text cannot express the corrected guidance without a small non-behavioral code change.

## 2. Specs

- **Primary files:** `mcp/vector/README.md`, `mcp/vector/src/tools/document.rs`
- **Expected output:** MCP-facing examples and tool descriptions that show unified diff hunks with 1-based indices and recommend `apply_patch` for agent-authored edits.
- **Dependencies:** none expected

## 3. Checklist

### 3.1. Phase A - Audit the Current MCP Guidance

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00067
  phase: Phase A
  language: rust, markdown
```

- [ ] Identify every MCP-facing `patch_doc` description or example that implies zero-based unified-diff hunk numbering or fails to state the preferred authoring format.
- [ ] Confirm whether the incorrect guidance lives only in static documentation, generated tool metadata, or both.
- [ ] Record the minimum file set needed to correct the guidance without widening the scope into runtime behavior changes.

### 3.2. Phase B - Correct Unified Diff Documentation

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00067
  phase: Phase B
  language: rust, markdown
```

- [ ] Update all MCP-facing unified diff examples so hunk headers use the real unified-diff convention with 1-based line indices.
- [ ] Remove any wording that can train callers to synthesize malformed hunk headers.
- [ ] Keep the examples minimal and valid for the governed-document path resolved by `doc_type`, `code`, and optional `package`.

### 3.3. Phase C - Make apply_patch the Recommended Path

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00067
  phase: Phase C
  language: rust, markdown
```

- [ ] Update MCP tool descriptions and README guidance to explicitly recommend omitted-format `apply_patch` for agent-authored edits.
- [ ] Preserve `format: "unified"` as a supported option for callers that already have source-control-native diffs.
- [ ] Ensure the wording distinguishes between the recommended default for agents and the still-supported explicit unified path.

### 3.4. Phase D - Validate the Published Contract

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00067
  phase: Phase D
  language: rust, markdown
```

- [ ] Regenerate or refresh any MCP-facing derived documentation if the repository flow requires it.
- [ ] Run the relevant documentation or crate quality gate needed to verify the updated MCP text.
- [ ] Confirm the final guidance is internally consistent across README text, tool metadata, and examples.

### 3.5. Phase Z - Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00067
  phase: Phase Z
  language: rust, markdown
```

- [ ] Update README files on packages modified.
- [ ] Keep the task in `todo` until the MCP-facing documentation and any derived outputs agree on both the 1-based unified-diff rule and the `apply_patch` recommendation.

## 4. Gaps, Flaws, and Tradeoffs

- **Gap:** The request assumes the issue is documentation-only; Phase A must verify that no generated tool text or tests still encode the wrong guidance.
- **Flaw:** If the MCP README and the tool metadata diverge, fixing only one source will recreate the inconsistency in the next generated output.
- **Tradeoff:** Recommending `apply_patch` more strongly reduces agent error rates, but it also makes the unified-diff path feel secondary even though it remains important for external tooling interoperability.
- **Flaw:** Unified diff examples are easy to get almost right while still being invalid; every example added here should be treated as contract surface, not illustrative filler.
- **Tradeoff:** Keeping the scope documentation-only avoids unnecessary runtime churn, but it leaves existing callers unchanged if they already learned the wrong convention from older docs.

## 5. Verification Notes

- On 2026-06-13, `patch_doc` was tested directly against this task document with `format: "unified"`.
- A zero-based-style hunk header failed:

```diff
@@ -5,1 +5,1 @@
-title: Update MCP patch_doc Documentation Guidance
+title: Update MCP patch_doc Documentation Guidance TEST ZERO BASED
```

- The tool returned a context mismatch and reported that document line 5 contained `slug: update-mcp-tool-patch-doc`, not `title: ...`.
- A one-based hunk header succeeded on the same target line:

```diff
@@ -6,1 +6,1 @@
-title: Update MCP patch_doc Documentation Guidance
+title: Update MCP patch_doc Documentation Guidance TEST ONE BASED
```

- Conclusion: `patch_doc` unified-diff hunks use real unified-diff line numbering, which is 1-based for existing lines in the target document.
- Staff assessment: the documentation must state this explicitly because an almost-correct zero-based example fails late as a patch application error, which is easy for agents to misdiagnose as a context problem instead of a numbering-rule problem.
