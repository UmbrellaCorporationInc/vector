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
2. If the document is a task for another document A, update the front matter in document A to include reference to the task
2. Use the path returned by `create_doc_prompt` to write the document content.
3. Derive content from what you already know in the conversation. Only read other files if a specific piece of information is genuinely unknown and required for the content.
4. Add an agent-inline-action after the main header of the document with this style, it should live with the other actions the template proposes. **Don't do this if the document-type is prompt, template or ai-rule**

```vector-agent-inline-action
label: Start a discussion about this document
prompt-field: prompt-message
profile: code
prompt: prompts-00010-start-discussion-action
input:
  document: <type>-<code>-<slug>
```

5. Every reference to another document must use wikilinks with the stem, except in the frontmatter, don't use wikilinks in the frontmatter, and don't reference the document [[prompts-00005-create-document]]
6. Update the frontmatter of the document base of this created document using the format "<type>-<code>-<slug>", the same for this new document assign in the frontmatter the id of the document base of this created document
7. Run `validate_fix` (vector MCP) after writing.
8. Stop — report the file path to the user.

If the document is technical:

1. follow `ai-rule-00004-user-decision-validation` for any technical decisions that arise during content creation.
2. Give the user at the end of the document generated your critical opinion as staff. Be objective and critical

**Never** bootstrap a governed document by manually writing a Markdown file before calling `create_doc_prompt`.

# 3. Validation vector

- All document content must follow `ai-rule-00002-english-communication`.
