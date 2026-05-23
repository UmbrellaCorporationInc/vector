---
id: prompts-00004-execute-task-phase
type: prompts
code: "00004"
slug: execute-task-phase
title: Execute Task Phase
description: Action prompt for executing a single phase of a governed task with validation requirements.
category: actions
created: 2026-05-11
updated: 2026-05-11
tags: []
---

# Prompt: Execute Task Phase

Execute task `#{task}` and only phase `#{phase}`.

Use `#{language}` for the implementation and all generated content.

## Instructions
1. Execute only the requested phase. Do not continue into earlier or later phases unless they are strictly required to complete `#{phase}`.
2. Use the vector MCP `find_doc` tool to locate the task document for `#{task}`.
3. Read the task document and identify its related wikilinks.
4. Use the vector MCP `find_doc` tool to locate and review the related documents referenced by those wikilinks when they are relevant to `#{phase}`.
5. Implement the work required to complete only phase `#{phase}`.

## Completion Requirements
1. Add or update tests that cover the implemented behavior.
2. Ensure all relevant tests pass before finishing.
3. Mark the implemented task items as completed in the task document once the work for `#{phase}` is finished.
4. Use the vector MCP `language_quality_gate` tool with the language list `#{lang}`.

## Output
Report the completed changes, the tests that were run, and the result of the language quality gate.
