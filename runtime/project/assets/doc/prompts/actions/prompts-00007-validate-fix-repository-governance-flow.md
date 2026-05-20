---
id: prompts-00007-validate-fix-repository-governance-flow
type: prompts
code: "00007"
slug: validate-fix-repository-governance-flow
title: Validate Fix Repository Governance Flow
description: Governed prompt for running repository-wide document validation with auto-fix from the container action surface.
category: actions
created: 2026-05-18
updated: 2026-05-18
tags:
  - governance
  - validation
  - repair
---

# Prompt: Validate Fix Repository Governance Flow

Run the repository-wide governed document repair flow for this project.

## 1. Instructions

1. Call the vector MCP `validate_fix` tool for the current repository root.
2. If the tool reports fixes, summarize the affected governed files and the repair outcome.
3. If the tool reports no fixes, state that the repository already satisfies the governed validation rules.
4. Stop after reporting the validation result.

## 2. Failure Handling

- If the repository root cannot be resolved, stop and report the missing root as a bounded error.
- If `validate_fix` fails, report the tool error without inventing a fallback mutation path.
- Do not rewrite unrelated repository files outside the governed validation flow.
