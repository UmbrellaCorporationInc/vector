---
id: task-00007-implement-rfc-00007-runtime-core-plugin-primitives
type: task
code: "00007"
slug: implement-rfc-00007-runtime-core-plugin-primitives
title: Implement RFC-00007 Runtime Core Plugin Primitives
description: Implement the plugin primitive contracts defined in RFC-00007, limited to the FlowOperation sender refinement and the PluginSender, PluginReceiver, and PluginOperation boundaries.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - plugin
  - async
related:
  - rfc-00007-runtime-core-plugin-primitives
  - rfc-00005-runtime-core-operation-and-event-flow-primitives
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
---

# Task 00007: Implement RFC-00007 Runtime Core Plugin Primitives

## 1. Prime Directive

`runtime-core` needs a coherent plugin execution boundary, but only at the operation-contract layer. This task implements the plugin primitives accepted in [[rfc-00007-runtime-core-plugin-primitives]] while keeping plugin identity, introspection, selection, and orchestration outside `runtime-core`.

## 2. Specs

- **Module:** `runtime/core`
- **Dependencies:** `rfc-00003` channel contracts must be implemented before Phase B. `rfc-00005` operation contracts must be refined in Phase A before plugin operation traits can be finalized.

## 3. Checklist

### 3.1. Phase A - FlowOperation sender type parameter refinement

Refine `FlowOperation` to accept `S: Sender<Output>` as an explicit third type parameter.

- [x] Add `S: Sender<Output>` as a third type parameter to `FlowOperation<Input, Output, S>`
- [x] Refine any receiver-driven flow shape accepted by the current implementation to use the same sender type parameter
- [x] Verify `FlowOperation` can express plugin-oriented `1:N` flow execution with cancel-aware senders
- [x] Verify plain sender-based flow execution remains valid after the refinement
- [x] Execute section "4. Quality Gate"

### 3.2. Phase B - PluginSender and PluginReceiver

Add the named cancel-aware aliases for plugin-oriented sender and receiver boundaries.

- [x] Define `PluginSender<T>` as a named alias over `CancelableSender<T>`
- [x] Define `PluginReceiver<T>` as a named alias over `CancelableReceiver<T>`
- [x] Verify `PluginSender<T>` is usable anywhere `CancelableSender<T>` is accepted
- [x] Verify `PluginReceiver<T>` is usable anywhere `CancelableReceiver<T>` is accepted
- [x] Execute section "4. Quality Gate"

### 3.3. Phase C - PluginOperation

Add the plugin operation trait as a sub-trait of `FlowOperation`.

- [x] Define `PluginOperation` as a sub-trait of `FlowOperation<Self::Input, Self::Output, Self::Sender>`
- [x] Add associated type `Input: Send + 'static`
- [x] Add associated type `Output: Send + 'static`
- [x] Add the plugin-oriented sender constraint for the output boundary
- [x] Verify `PluginOperation` is usable anywhere `FlowOperation<Self::Input, Self::Output, Self::Sender>` is accepted
- [x] Execute section "4. Quality Gate"

### 3.4. Phase D - Macro-assisted operation authoring

Automate the creation of `PluginOperation` wrappers and metadata from external async functions.

- [x] Define a macro `declare_plugin_operations!` to wrap external async functions into structs
- [x] Generate `PluginOperation` wrappers from valid async function declarations
- [x] Keep the macro manifest-oriented with no business logic
- [x] Support multiple operations in a single macro invocation
- [x] Reject duplicate operation names within one declaration block when the macro can detect them
- [x] Execute section "4. Quality Gate"

### 3.5. Phase E - Public API integration

Expose the accepted plugin primitive surface from `runtime-core`.

- [x] Re-export `PluginOperation`, `PluginReceiver`, and `PluginSender`
- [x] Update `runtime/core` README to document the operations-only plugin primitive surface
- [x] Verify manual `PluginOperation` implementations remain possible without relying on the macro path
- [x] Execute section "4. Quality Gate"

### 3.6. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update `runtime/core` README to document the plugin primitive surface

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `FlowOperation` accepts `S: Sender<Output>` as a type parameter and existing implementors compile
- [x] `PluginSender<T>` and `PluginReceiver<T>` are usable as cancel-aware sender and receiver boundaries
- [x] `runtime-core` exposes no `Plugin`, `PluginId`, or `OperationToken` boundary as part of this task
- [x] Macro-generated operation authoring creates valid `PluginOperation` wrappers consistent with declared async functions
- [x] Manual `PluginOperation` implementations remain valid without requiring the macro path
- [x] All quality gates pass
