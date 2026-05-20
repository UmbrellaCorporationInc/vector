---
id: ai-rule-00003-documentation
type: ai-rule
code: "00003"
slug: documentation
title: Create Documents
description: Defines the strict, no-detour workflow for creating governed documents using the vector MCP.
status: active
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
trigger: always_on
tags:
  - documentation
  - prompts
  - mcp
---

# AI Rule: Create Document

## Supported document types

#{types}

## 1. Document creation instruction:

- Use `find_doc` (vector MCP) to check if the document already exists.
- If `find_doc` returns a path, **stop — do not create a new document**.

## 2. Create Document Workflow

**Do this in order. Do not add steps.**

1. Call `create_doc_prompt` immediately — no pre-work, no file exploration, no research first.
2. Use the path returned by `create_doc_prompt` to write the document content.
3. Derive content from what you already know in the conversation. Only read other files if a specific piece of information is genuinely unknown and required for the content.
6. Run `validate_fix` (vector MCP) after writing.
5. Stop — report the file path to the user.

**Never** bootstrap a governed document by manually writing a Markdown file before calling `create_doc_prompt`.

# 3. Validation vector

- All document content must follow `ai-rule-00002-english-communication`.
