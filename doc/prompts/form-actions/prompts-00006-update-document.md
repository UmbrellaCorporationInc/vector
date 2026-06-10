---
id: prompts-00006-update-document
type: prompts
code: "00006"
slug: update-document
title: Update Document
description: Governed prompt for updating a section of a governed document from a header inline action with optional author context.
category: form-actions
created: 2026-05-18
updated: 2026-05-18
tags:
  - authoring
  - document
  - governance
---

# Prompt: Update Document

You are updating a governed document in this repository. The document stem is `#{document-stem}`.

The change applies under `#{document-header}`

## 1. Context

#{prompt-message}

## 2. Update Document Workflow

**Do this in order. Do not add steps.**

1. Use `find_doc` (vector MCP) to locate the governed document identified by `#{document-stem}`.
2. Read the document to understand its current content and structure.
3. Apply the update requested in the context above, staying within the section indicated by the author.
4. Use wikilinks to referenciate other governed documents, don't use wikiliinks in the frontmatter
5. Run `validate_fix` (vector MCP) after writing.
6. Stop — report the updated file path and the changes made.

**Never** rewrite sections unrelated to the author's request.

## 3. Input Contract

- `document-stem`: required — identifies the governed document to update.
- `prompt-message`: optional — author context describing what to update; execution must not depend on it being present.

## 4. Validation

- All document content must follow `ai-rule-00002-english-communication`.
