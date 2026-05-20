---
id: prompts-00009-typescript
type: prompts
code: "00009"
slug: typescript
title: TypeScript Quality Gate
description: Quality-gate prompt for TypeScript work that must be completed after implementation.
category: quality-gate
created: 2026-05-18
updated: 2026-05-18
tags: []
---

# Prompt: TypeScript Quality Gate

After completing the TypeScript task implementation, run the following quality gates in this order:

1. Run `pnpm typecheck`.
   Fix every reported error before continuing.
2. Run `pnpm lint`.
   Fix every reported error before continuing.
3. Run `pnpm format:check`.
   Fix every reported error before continuing.
4. Run `pnpm test`.
   All tests must pass before finishing.

If any check fails, fix the issue and rerun the failed command until all quality gates pass.
