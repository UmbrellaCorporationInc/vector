---
id: prompts-00002-create-doc
type: prompts
code: "00002"
slug: create-doc
title: Create Document
description: Default governed prompt for authoring a newly bootstrapped document.
category: authoring
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---

# Prompt: Create Document

You are authoring a governed `#{doc-type}` document.

## Bootstrap Output
- **Code:** `#{code}`
- **Slug:** `#{slug}`
- **File Path:** `#{file-path}`

## Instructions
1. Open `#{file-path}`.
2. Replace any remaining template placeholders with concrete content for the requested document.
3. Preserve the governed frontmatter fields and keep `id`, `type`, `code`, and `slug` aligned with the bootstrapped file name.
4. Fill in `title`, `description`, body content, and any required type-specific metadata.
5. Do not create a second document for the same request.
6. Run `validate_fix` (vector MCP) after writing.

## Output
Update the bootstrapped file in place and report the authored document path.
