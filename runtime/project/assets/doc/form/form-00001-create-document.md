---
id: form-00001-create-document
type: form
code: "00001"
slug: create-document
title: Create Document
description: Form used to create a new document in the system.
created: 2026-05-15
updated: 2026-05-15
tags: []
related: []
---

# Create Document

```vector-form
document-name = input("Document name:")
message = chat-input("Prompt:")
```


```vector-agent-button
label: Create document
profile: create-doc
prompt: prompts-00005-create-document
input: 
  document-type: #{document-type}
```
