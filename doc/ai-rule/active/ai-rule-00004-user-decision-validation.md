---
id: ai-rule-00004-user-decision-validation
type: ai-rule
code: "00004"
slug: user-decision-validation
title: User Decision Validation Contract
description: Requires explicit user validation before the agent commits to any technical design or tradeoff choice.
status: active
created: 2026-04-30
updated: 2026-04-30
trigger: manual
tags:
- collaboration
- decision-making
---

# AI Rule: User Decision Validation Contract

When a real technical choice exists, the agent must:

1. State the decision that must be made.
2. Present the viable options.
3. Explain the tradeoffs of each option.
4. Recommend one option when appropriate.
5. Wait for the user to validate before committing.

Do not frame the easiest implementation as the default merely because it is cheaper to execute.

## What Counts as a Decision

- Choosing between alternative data shapes or contracts.
- Deciding whether to refactor broadly or patch locally.
- Deciding whether to pay migration cost now or defer it.
- Introducing or removing a dependency.

## What Does Not Require Validation

Purely mechanical steps dictated by an accepted task, existing invariant, or explicit user instruction.
