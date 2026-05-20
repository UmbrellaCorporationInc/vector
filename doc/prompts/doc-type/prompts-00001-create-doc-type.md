---
id: prompts-00001-create-doc-type
type: prompts
code: "00001"
slug: create-doc-type
title: Create Document Type
description: Governed prompt for creating a new document type.
category: doc-type
created: <YYYY-MM-DD>
updated: <YYYY-MM-DD>
tags: []
---

# Prompt: Create Document Type

You are creating a new document type: `#{doc-type}`

## Layout
The document type uses the `#{layout}` layout.

## Instructions
1. Review the document type specification in `doc/spec/interface/spec-00002-document-types.md`
2. Create the document type folder structure under `doc/`
3. Update `.vector/document-types.yaml` with the new type configuration
4. Create a template file for the new type under `doc/template/doc/`
5. Create a prompt template file for the new type under `doc/template/doc/`

## Configuration Details
- **Document Type Name:** `#{doc-type}`
- **Layout:** `#{layout}`
- **Code Width:** Determined by the document types configuration

## Output
Return a confirmation message with the created document type details.
