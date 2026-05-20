---
id: prompts-00008-rust
type: prompts
code: "00008"
slug: rust
title: Rust Quality Gate
description: Quality-gate prompt for Rust work that must be completed after implementation.
category: quality-gate
created: 2026-05-11
updated: 2026-05-11
tags: []
---

# Prompt: Rust Quality Gate

After completing the Rust task implementation, run the following quality gates in this order:

1. Run `xtask quality-lint`.
   Fix every reported error before continuing.
2. Run `xtask quality-test`.
   All tests must pass and coverage must not be lower than 70%.
3. Run `cargo fmt --all`.

Do not use `cargo xtask ...`. Use `xtask ...` directly.

If any check fails, fix the issue and rerun the failed command until all quality gates pass.
