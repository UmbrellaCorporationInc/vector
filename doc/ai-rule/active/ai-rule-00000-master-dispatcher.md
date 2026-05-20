---
id: ai-rule-00000-master-dispatcher
type: ai-rule
code: "00000"
slug: master-dispatcher
title: Master Dispatcher
description: The only rule loaded by default — routes all other rules based on request context.
status: active
created: 2026-04-30
updated: 2026-04-30
trigger: always_on
tags:
- meta
- dispatcher
---

# AI Rule: Master Dispatcher

This is the **ONLY** rule loaded by default. Do **NOT** act blindly. When facing a request,
read the appropriate rule files from `doc/ai-rule/active/` before proceeding:

- **Any technical request:** Always read `ai-rule-00001-staff-engineer-expertise.md` first.
- **Technical choice with tradeoffs:** Also read `ai-rule-00004-user-decision-validation.md`.
- **Always-on -- language and tone:** Read `ai-rule-00002-english-communication.md`.
- **Always-on -- document creation:** Read `ai-rule-00003-documentation.md`.
