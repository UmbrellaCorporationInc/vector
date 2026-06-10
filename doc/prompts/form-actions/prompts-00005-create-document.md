---
id: prompts-00005-create-document
type: prompts
code: "00005"
slug: create-document
title: Create Document
description: Governed prompt for creating a new document from a requested document type, slug, and authoring message.
category: form-actions
created: 2026-05-18
updated: 2026-05-18
tags:
  - authoring
  - document
  - governance
---

# Prompt: Create Document

You are creating a governed document of type #{document-type} in this repository. The slug is #{document-name}. 

## 1. About document

#{message}

## 2. Create Document Workflow

**Do this in order. Do not add steps.**

1. Call `create_doc_prompt` immediately — no pre-work, no file exploration, no research first.
2. Use the path returned by `create_doc_prompt` to write the document content.
3. Derive content from what you already know in the conversation. Only read other files if a specific piece of information is genuinely unknown and required for the content.
4. Use wikilinks to referenciate other governed documents, don't use wikiliinks in the frontmatter
5. Run `validate_fix` (vector MCP) after writing.
6. Stop — report the file path to the user.

**Never** bootstrap a governed document by manually writing a Markdown file before calling `create_doc_prompt`.

# 3. Validation vector

- All document content must follow `ai-rule-00002-english-communication`.
