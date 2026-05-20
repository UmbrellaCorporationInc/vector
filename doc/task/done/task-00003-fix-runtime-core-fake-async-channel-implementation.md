---
id: task-00003-fix-runtime-core-fake-async-channel-implementation
type: task
code: "00003"
slug: fix-runtime-core-fake-async-channel-implementation
title: Fix runtime-core fake async channel implementation
description: Replace the current std::sync::mpsc-backed blocking channel behavior with a truly async-first runtime-core channel boundary aligned with RFC 00002.
status: done
created: 2026-05-02
updated: 2026-05-02
tags:
  - runtime
  - channels
  - async
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
---

# Task 00003: Fix runtime-core fake async channel implementation

## 1. Prime Directive

Eliminate the current contract mismatch where `runtime-core` exposes async-shaped channel APIs while performing blocking `std::sync::mpsc` operations before the returned futures are polled.

## 2. Specs

- **Module:** `runtime/core/`
- **Dependencies:** Rust `std`, `thiserror`

## 3. Checklist

### 3.1. Phase A - Boundary decision and semantic alignment

- [x] Define the target async-first channel behavior for `Sender`, `Receiver`, `CancelableSender`, and `CancelableReceiver`
- [x] Decide whether the fix stays within `std` constraints or requires a follow-up RFC to change the dependency boundary
- [x] Document the accepted behavior for cancellation observation versus channel closure
- [x] Validation vector for Phase A completed
- [x] execute section "4. Quality Gate"

Phase A decision record:

- Keep the accepted dependency boundary unchanged: `runtime-core` remains limited to Rust `std` and `thiserror` for this fix.
- Do not introduce `tokio`, `futures`, `async-channel`, or any other third-party async primitive as part of this task.
- Replace the current `std::sync::mpsc`-backed implementation with an internal single-producer single-consumer async channel built from `std` primitives, shared state, and explicit wake-up behavior.
- Preserve the existing public contract shape: one base sender, one base receiver, one cancel-aware specialization for each, and one shared `CancelHandler`.
- Treat async-first as a semantic requirement, not a syntactic one: no send or receive path may perform blocking work before the returned future is polled.
- Preserve the RFC 00003 transport-agnostic boundary: the implementation may change internally, but the crate must not expose executor-specific or transport-specific policy.
- Preserve the single-publisher single-subscriber model accepted by RFC 00002 and RFC 00003.
- Accept that this task fixes the implementation contract mismatch first, not the full RFC 00002 surface gap.
- For ordinary receivers, `recv()` continues to resolve to the next item or `None` when the channel is definitively closed and empty.
- For cancel-aware receivers, cancellation must become observable without relying on blocking receive behavior.
- Because `CancelableReceiver<T>` must remain compatible with `Receiver<T>`, Phase C must remove cancellation ambiguity by adding explicit cancel-observation semantics without breaking base receiver substitution.
- If that explicit cancel-observation semantic cannot be added cleanly within the RFC 00003 contract shape, Phase C must produce a follow-up RFC note rather than silently weakening the behavior.

Phase A tradeoff note:

- Staying inside `std` avoids widening the dependency boundary and keeps the fix aligned with RFC 00002, but it requires more careful internal concurrency code than adopting an off-the-shelf async channel.
- Changing dependencies now would likely be easier mechanically, but it would hide the real architectural question behind a library choice and create RFC churn for a bug-fix task.

### 3.2. Phase B - Base channel implementation repair

- [x] Replace the current blocking base channel implementation with behavior that is truly async-first
- [x] Preserve the single-publisher single-subscriber boundary and public trait compatibility
- [x] Add or update tests proving that channel operations no longer perform blocking work before future polling
- [x] Validation vector for Phase B completed
- [x] execute section "4. Quality Gate"

Phase B implementation record:

- Replaced `std::sync::mpsc::SyncSender` + `Receiver` with shared `Arc<Mutex<Shared<T>>>` state containing a `VecDeque<T>`, drop flags, and a `recv_waker: Option<Waker>`.
- `SendFuture::poll` deposits the value and notifies the stored `Waker` — no blocking before poll.
- `RecvFuture::poll` dequeues a value if present, returns `None` if sender is dropped, otherwise stores the `Waker` and returns `Pending`.
- `Drop` implementations on `AsyncSender` and `AsyncReceiver` set the corresponding drop flag and wake any parked receiver.
- New tests `send_future_does_not_block_before_poll` and `recv_future_does_not_block_before_poll` confirm the async-first contract.

### 3.3. Phase C - Cancel-aware channel repair

- [x] Align `CancelHandler`, `CancelableSender`, and `CancelableReceiver` with the corrected async-first model
- [x] Remove ambiguity between observed cancellation and normal channel closure at the receiver boundary
- [x] Add or update tests for cancel-aware send and receive behavior under the repaired model
- [x] Validation vector for Phase C completed
- [x] execute section "4. Quality Gate"

Phase C implementation record:

- Replaced `std::sync::mpsc`-backed cancel-aware types with the same `Arc<Mutex<Shared<T>>>` pattern, plus a shared `Arc<AtomicBool>` cancellation flag.
- `CancelSendFuture::poll` checks the flag before depositing — returns `Cancelled` if set.
- `CancelRecvFuture::poll` checks the cancellation flag before dequeuing — returns `None` immediately if set.
- Cancellation is distinguishable from ordinary closure via `CancelableReceiver::is_cancelled()` — confirmed by new tests `recv_none_due_to_cancellation_is_distinguishable_from_closure` and `recv_none_due_to_closure_is_distinguishable_from_cancellation`.
- No follow-up RFC needed: the cancel-observation semantic fits cleanly within the RFC 00003 contract shape.

### 3.4. Phase D - Contract and documentation alignment

- [x] Update crate documentation to state the async-first constraint explicitly
- [x] Ensure public API documentation no longer implies blocking `std::sync::mpsc` semantics as the intended runtime model
- [x] Record any remaining RFC gaps that are out of scope for this task
- [x] Validation vector for Phase D completed
- [x] execute section "4. Quality Gate"

Phase D remaining RFC gaps (out of scope for this task):

- RFC 00002 surface gap: the full async-first boundary contract beyond channels (e.g. spawn, join, timer abstractions) is not addressed here.
- Backpressure policy: the current implementation uses an unbounded `VecDeque`. A bounded variant would require a sender-side pending path and explicit wake registration, which is out of scope.

### 3.5. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] No public channel contract performs blocking work before the returned future is polled
- [x] Base and cancel-aware channel boundaries remain compatible with RFC 00003
- [x] Cancellation observation is distinguishable from ordinary channel closure where the contract requires it
- [x] The crate documentation states async-first behavior as a foundational constraint
- [x] Phase A decision record remains consistent with the final implementation
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
