---
id: prompts-00010-start-discussion-action
type: prompts
code: "00010"
slug: start-discussion-action
title: Start Discussion Action
description: Governed prompt for starting a discussion about an existing governed document.
category: actions
created: 2026-06-13
updated: 2026-06-13
tags:
  - discussion
  - governance
  - prompt
---

# Prompt: Start Discussion Action

You are starting a discussion about an existing governed document identified by `#{document}`.

## Goal

Open the target document, gather its immediate linked context, and ask the first discussion question provided in `#{prompt-message}`.

## Required workflow

1. Use the vector MCP `find_doc` tool to resolve `#{document}` to the canonical file path.
2. Read the resolved document.
3. Collect the WikiLinks referenced by that document and load only the linked documents that are directly relevant to the discussion.
4. Build a concise discussion context from the target document and its relevant linked documents.
5. Start the discussion with the exact intent of `#{prompt-message}`.

## Discussion rules

- Ground every claim in the loaded document set.
- Prefer identifying assumptions, gaps, risks, constraints, and unresolved decisions over summarizing obvious text.
- If the discussion is technical, apply:
  - `@doc/ai-rule/active/ai-rule-00001-staff-engineer-expertise.md`
  - `@doc/ai-rule/active/ai-rule-00004-user-decision-validation.md`
- If the discussion is technical, act with staff engineer rigor:
  - Surface flaws, gaps, edge cases, and tradeoffs.
  - Separate facts from inference.
  - If a real design choice exists, present options, explain tradeoffs, recommend a direction, and ask the user to validate before committing to a design conclusion.

## Output format

Produce the response in this order:

1. `Document in discussion:` with the resolved document identifier or path.
2. `Loaded context:` with the relevant linked documents actually used.
3. `Discussion:` with the first substantive response that advances the conversation from `#{prompt-message}`.
4. `Staff opinion:` only when the discussion is technical. This section must provide a critical and objective opinion on the document quality, decision clarity, and technical risk.

## Constraints

- Do not invent linked context that was not loaded.
- Do not ask generic kickoff questions if `#{prompt-message}` already provides the first question.
- Keep the opening focused and decision-oriented.
