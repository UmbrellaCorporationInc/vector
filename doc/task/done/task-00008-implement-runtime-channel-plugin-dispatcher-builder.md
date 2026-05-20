---
id: task-00008-implement-runtime-channel-plugin-dispatcher-builder
type: task
code: "00008"
slug: implement-runtime-channel-plugin-dispatcher-builder
title: Implement runtime-channel plugin dispatcher builder
description: Implement the runtime-channel plugin dispatcher builder that prepares plugin operation execution using a connected cancelable output channel and optional observability instrumentation.
status: done
created: 2026-05-03
updated: 2026-05-04
tags:
  - runtime
  - channel
  - plugin
  - dispatch
related:
  - rfc-00008-runtime-channel-plugin-dispatcher-builder
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00006-runtime-core-control-observability-and-encoding-primitives
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
---

# Task 00008: Implement runtime-channel plugin dispatcher builder

## 1. Prime Directive

`runtime-channel` needs one standard builder that wires a selected `PluginOperation` to a connected cancel-aware output channel, optional observability listeners, and a returned cancellation handle without pushing orchestration policy into `runtime-core`.

## 2. Specs

- **Module:** `runtime/channel`
- **Dependencies:** `rfc-00003`, `rfc-00006`, `rfc-00007`, `rfc-00008`

## 3. Checklist

### 3.1. Phase A - Dispatcher builder surface

- [x] Define the concrete `PluginDispatcher` builder type in `runtime-channel`
- [x] Accept one selected `PluginOperation` at construction time
- [x] Accept one input value before build
- [x] Accept optional observability listener registration before build
- [x] Add tests covering basic builder configuration
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - Connected cancelable output channel wiring

- [x] Use the standard runtime-owned `cancelable_channel<T>()` backend inside `PluginDispatcher::build`
- [x] Return `(CancelHandler, PluginReceiver<T>)` from `build`
- [x] Ensure the returned receiver is connected to the sender endpoint passed into the selected operation
- [x] Ensure the returned cancel handler controls the connected output channel pair
- [x] Add tests covering connected output delivery and cancellation visibility
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Sender observability instrumentation

- [x] Add a sender wrapper that still satisfies `PluginSender<T>`
- [x] Emit `ObservabilityEvent::MessageSent` from sender instrumentation after successful publication
- [x] Keep instrumentation optional when no listeners are registered
- [x] Add tests covering observed and unobserved sender behavior
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - Dispatcher execution wiring

- [x] Run the selected `PluginOperation` with the connected sender endpoint during dispatcher build or immediate prepared execution
- [x] Emit dispatcher-owned lifecycle observability such as operation start and completion when configured
- [x] Keep lifecycle wiring outside the `PluginOperation` contract
- [x] Add tests covering operation execution success and lifecycle observability
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Failure behavior

- [x] Define dispatcher behavior when operation execution fails
- [x] Define dispatcher behavior when sender instrumentation cannot emit an observability event
- [x] Ensure listener-side failure does not retroactively fail output publication
- [x] Add tests covering operation failure and listener failure paths
- [x] execute section "4. Quality Gate"

### 3.6. Phase F - Public API integration

- [x] Re-export the accepted dispatcher builder from `runtime-channel`
- [x] Update `runtime/channel` README to document the dispatcher boundary and output channel topology
- [x] Verify the public API exposes no plugin identity, lookup, or registry concerns
- [x] execute section "4. Quality Gate"

### 3.7. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `PluginDispatcher` prepares one selected operation using a connected cancel-aware output channel
- [x] `PluginDispatcher::build` returns `(CancelHandler, PluginReceiver<T>)`
- [x] The returned receiver is connected to the sender endpoint passed into the selected operation
- [x] Sender-side observability uses a wrapper that still satisfies `PluginSender<T>`
- [x] Listener attachment is optional and does not change the `PluginOperation` contract
- [x] Listener-side failure after event dispatch does not retroactively fail output publication
- [x] The public API introduces no plugin identity, lookup, registry, or discovery concerns
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
