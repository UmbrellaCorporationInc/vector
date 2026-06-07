---
id: task-00057-fix-find-doc-mcp-tool
type: task
code: "00057"
slug: fix-find-doc-mcp-tool
title: Fix find_doc mcp tool
description: Transition the find_doc MCP tool to return a structured JSON response instead of a plain-text formatted response with prepended metadata lines, preventing patch generation issues.
status: done
created: 2026-06-07
updated: 2026-06-07
tags: []
related: []
supersedes: []
superseded_by: null
---

# Task 00057: Fix find_doc mcp tool

## 1. Prime Directive

> [!Prime Directive]
> The current `find_doc` MCP tool returns its output in a flat text format where metadata fields like `path:` and `package:` are prepended directly to the document content. This format pollutes the raw document content. When downstream agents attempt to use this tool output to generate or apply unified diffs using `patch_doc`, the presence of these fake metadata lines causes patching failures. This task transitions `find_doc` to return a clean structured JSON response, separating metadata from the actual document content.

## 2. Specs

- **Module:** `mcp-vector` (specifically `mcp/vector/src/tools/document.rs`)
- **Dependencies:** `serde_json`, `serde::Serialize`

## 3. Checklist

### 3.1. Phase A — Structured JSON Response for find_doc

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00057
  phase: Phase A
  language: rust
```

- [x] Define a serializable `FindDocResponse` struct in `mcp/vector/src/tools/document.rs` containing `path`, `package`, and `content`.
- [x] Modify the `find_doc` tool implementation to serialize `FindDocResponse` into a JSON string and return it.
- [x] Update the integration and unit tests in `mcp/vector/src/tools/document_test.rs` to deserialize and validate the JSON fields rather than checking substring contains on raw text.
- [x] Ensure all tests compile and pass successfully.

### 3.2. Phase Z — Wrap-up

```vector-agent-action
label: Execute Phase in Agent
profile: code
prompt: prompts-00004-execute-task-phase
input:
  task: task 00057
  phase: Phase Z
  language: rust
```

- [x] Update README files on packages modified
