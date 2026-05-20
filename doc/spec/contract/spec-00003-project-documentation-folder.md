---
id: spec-00003-project-documentation-folder
type: spec
code: "00003"
slug: project-documentation-folder
title: Project Documentation Folder
description: Defines the governed project documentation folder, coded document naming, and templates for project-level documents.
category: contract
created: 2026-05-01
updated: 2026-05-01
authors: []
tags:
  - documentation
  - project
  - governance
related:
  - spec-00001-repository-directory-structure
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00003: Project Documentation Folder"
---

# SPEC 00003: Project Documentation Folder

## 1. Purpose

This spec defines the contract for `doc/project/` as the governed folder for project-level documentation and `doc/template/project/` as the governed folder for project-level document templates.

This spec follows [[spec-00001-repository-directory-structure]].

## 2. Definition

`doc/project/` is the canonical folder for project-level governed documents.

`doc/template/project/` is the canonical folder for project-level governed templates.

Documents in `doc/project/` must use the naming pattern:

- `project-<code>-<slug>.md`

Where:

- `<code>` is a zero-padded numeric identifier
- `<slug>` is a unique lowercase kebab-case document name

`doc/project/` must contain governed documents such as:

- `project-0001-definition.md`
- `project-0002-principles.md`
- `project-0003-<lang>-dependencies.md`

Document responsibilities:

- `project-0001-definition.md` explains the goal of the project.
- `project-0002-principles.md` defines the design principles that guide the project.
- `project-0003-<lang>-dependencies.md` defines the allowed dependencies for one language in the project.

`doc/template/project/` must contain templates for governed project documents.

Templates in `doc/template/project/` must use the naming pattern:

- `template-<code>-<slug>.md`

Required templates:

- one template for the project definition document
- one template for the project principles document
- one template for language dependency governance documents

Dependency governance document structure:

- H1: `Dependencies`
- H2 sections must be numbered
- each dependency entry must use an H2 heading in the form `## <n>. <name>`
- each dependency entry must declare `Tags:`
- each dependency entry must declare `Scope:`
- each dependency entry must declare `Description:`

Definition document structure:

- the document must use one H1 title
- the remaining sections must use numbered H2 headings

Principles document structure:

- H1: `Project Principles`
- H2 sections must be numbered
- each principle entry must use an H2 heading in the form `## <n>. <name>`

## 3. Invariants

- Project-level governance documents must live under `doc/project/`.
- Files under `doc/project/` must use the `project-<code>-<slug>.md` naming pattern.
- File names under `doc/project/` must be unique.
- `project-0001-definition.md` must exist as the canonical project definition document.
- `project-0002-principles.md` must exist as the canonical project principles document.
- Dependency governance documents must be language-specific and must not mix multiple languages in one file.
- Each language dependency governance document must describe only allowed dependencies for its language.
- Project-level governed templates must live under `doc/template/project/`.
- Files under `doc/template/project/` must use the `template-<code>-<slug>.md` naming pattern.
- File names under `doc/template/project/` must be unique.
- Templates under `doc/template/project/` must follow the same header discipline as their governed document type.
- Project document templates must not use unnumbered H2 sections.

## 4. Examples

```text
doc/
|-- project/
|   |-- project-0001-definition.md
|   |-- project-0002-principles.md
|   `-- project-0003-rust-dependencies.md
`-- template/
    `-- project/
        |-- template-00004-project-definition-template.md
        |-- template-00005-language-dependency-governance-template.md
        `-- template-00006-project-principles-template.md
```

Valid dependency entry example:

```markdown
# Dependencies

## 1. thiserror

Tags: #rust #error-handling
Scope: runtime
Description: Standard error derivation crate allowed for shared runtime libraries.
```

Valid definition document example:

```markdown
# Project Definition

## 1. Goal

Describe the goal of the project.

## 2. Non-Goals

Describe what the project does not aim to solve.
```

Valid principles document example:

```markdown
# Project Principles

## 1. Decentralization

Describe the principle and why it matters to the project.
```

Valid dependency template example:

```markdown
# Dependencies

## 1. <dependency-name>

Tags: #<language> #<category>
Scope: <runtime | build | dev | test | cli>
Description: <Why this dependency is allowed and under what expectations.>
```

## 5. Open Questions

- None
