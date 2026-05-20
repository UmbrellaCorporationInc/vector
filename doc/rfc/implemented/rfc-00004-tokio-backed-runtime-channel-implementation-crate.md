---
id: rfc-00004-tokio-backed-runtime-channel-implementation-crate
type: rfc
code: "00004"
slug: tokio-backed-runtime-channel-implementation-crate
title: Tokio-Backed Runtime Channel Implementation Crate
description: Defines a dedicated runtime-channel crate that implements runtime-core channel contracts using Tokio primitives while keeping runtime-core transport-agnostic.
status: implemented
created: 2026-05-03
updated: 2026-05-03
authors: []
tags:
  - runtime
  - channels
  - tokio
  - architecture
related:
  - spec-00002-runtime-core-crate
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
supersedes: []
superseded_by: null
aliases:
  - "RFC 00004: Tokio-Backed Runtime Channel Implementation Crate"
---

# RFC 00004: Tokio-Backed Runtime Channel Implementation Crate

## 1. Problem

`runtime-core` currently owns both the channel contracts and one concrete channel implementation.
That mixes two responsibilities that should stay separate:

- `runtime-core` defines the transport-agnostic async boundary for channel-based runtime flow
- the current implementation defines one specific queueing and wake-up strategy

This is already creating pressure in the wrong direction. The channel primitive is expected to be
used across multiple runtime crates, which makes its implementation important infrastructure. If
that infrastructure remains inside `runtime-core`, the crate stops being a pure contract boundary
and becomes the default place for concrete runtime behavior.

The project also needs a more mature async backend than the current hand-rolled shared-state
implementation. The current design is small, but it duplicates concurrency logic that a runtime
primitive crate should not keep re-deriving if the project has already chosen Tokio-oriented async
execution elsewhere.

The repository needs one explicit decision about where the standard channel implementation lives,
how it is instantiated, and how `runtime-core` stays transport-agnostic while still enabling a
standard runtime-owned channel factory.

## 2. Proposal

Create a new crate under `runtime/channel/` named `runtime-channel`.

Ownership boundary:

- `runtime-core` continues to own `Sender<T>`, `Receiver<T>`, `CancelableSender<T>`,
  `CancelableReceiver<T>`, `CancelHandler`, `RuntimeError`, and `RuntimeResult`
- `runtime-core` stops owning the standard concrete channel implementation
- `CancelHandler` is a trait in `runtime-core`, not a concrete implementation
- `runtime-channel` owns the standard Tokio-backed implementation of the accepted channel contracts
- `runtime-channel` owns channel-oriented utilities and helpers that are specific to channel
  construction, configuration, and cancellation wake-up behavior

Implementation boundary:

- `runtime-channel` depends on `runtime-core`
- `runtime-channel` depends on `tokio`
- `runtime-core` must not depend on `tokio`
- the current concrete implementation in `runtime/core/src/channel.rs` and
  `runtime/core/src/cancel.rs` must be removed from `runtime-core`

Factory boundary:

- the standard connected-endpoint factories move out of `runtime-core`
- `runtime-channel` exposes `channel<T>() -> (impl Sender<T>, impl Receiver<T>)`
- `runtime-channel` exposes
  `cancelable_channel<T>() -> (impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>)`
- concrete endpoint types remain hidden behind `impl Trait`
- the public factory surface must include a capacity-aware configuration path owned by
  `runtime-channel`

Tokio backend boundary:

- base message transport uses bounded `tokio::sync::mpsc`
- channel capacity is part of the `runtime-channel` configuration boundary rather than an internal
  hard-coded constant
- the standard implementation must not default to unbounded channel transport
- capacity definition is part of this RFC and the initial crate implementation rather than a later
  follow-up
- cancellation state uses a shared flag that authors can observe synchronously through
  `is_cancelled()`
- cancellation wake-up uses `tokio::sync::watch` so pending awaits can be released when
  cancellation is signalled
- a cancellation flag alone is not sufficient for the standard implementation because it does not
  wake pending awaits

Semantic boundary:

- the accepted one-publisher, one-subscriber contract from RFC 00003 remains unchanged
- `CancelableSender<T>` remains substitutable as `Sender<T>`
- `CancelableReceiver<T>` remains substitutable as `Receiver<T>`
- cancellation must remain distinguishable from ordinary channel closure through the accepted
  `is_cancelled()` observation rule
- channel authors remain responsible for deciding the policy after cancellation is observed
- no public API may perform blocking work before the returned future is polled or awaited

Migration boundary:

- higher-level crates that only need channel traits depend only on `runtime-core`
- higher-level crates that need the standard runtime-owned channel implementation depend on
  `runtime-channel`
- no `common` crate is introduced for channel ownership

Dependency governance note:

- this RFC proposes `tokio` as an approved runtime dependency for `runtime-channel`
- this RFC does not approve `tokio` as a dependency of `runtime-core`
- any additional Tokio-adjacent dependency beyond `tokio` itself must be justified separately if it
  is not strictly required

## 3. Alternatives Considered

- **Keep the implementation in `runtime-core`:** Discarded because it keeps concrete queueing and
  wake-up policy inside the transport-agnostic contract crate.
- **Create a generic `common` crate for shared channels:** Discarded because `common` is an
  ownership bucket, not a clear architectural boundary.
- **Embed Tokio directly into `runtime-core`:** Discarded because it collapses the contract crate
  into one executor-oriented implementation choice.
- **Keep the hand-rolled `std` implementation as the long-term backend:** Discarded because the
  project expects heavy reuse of this primitive, and maintaining custom concurrency internals in the
  core contract crate is the wrong long-term maintenance trade.
- **Use only a cancellation flag with no wake-up primitive:** Discarded because it cannot release a
  pending await when cancellation is signalled.

## 4. Tradeoffs

| Pro | Con |
|-----|-----|
| `runtime-core` stays aligned with its accepted role as a transport-agnostic contract crate. | The workspace gains one more crate and one more dependency edge. |
| `runtime-channel` can use Tokio-native primitives instead of maintaining custom wake-up logic. | Crates that instantiate the standard channels will often depend on both `runtime-core` and `runtime-channel`. |
| The standard runtime-owned channel implementation remains explicit rather than hidden inside the contract crate. | The current RFC 00003 factory placement must be refined because factories no longer live in `runtime-core`. |
| `impl Trait` factories preserve abstraction without exposing concrete endpoint types. | Debugging concrete implementation details becomes slightly less direct for downstream users. |
| Tokio adoption is limited to the implementation crate instead of contaminating the entire runtime foundation. | The project becomes more opinionated about Tokio as the default runtime backend. |
| Bounded channels make backpressure policy explicit in the first standard implementation. | `runtime-channel` must own channel-capacity configuration semantics from day one. |
| `watch` allows cancellation to release pending awaits without forcing one cancellation policy on every caller. | The implementation carries both state and notification primitives instead of one minimal flag. |

## 5. Acceptance Criteria

- [ ] A new crate exists under `runtime/channel/` with crate name `runtime-channel`.
- [ ] `runtime-channel` depends on `runtime-core`.
- [ ] `runtime-channel` depends on `tokio`.
- [ ] `runtime-core` does not depend on `tokio`.
- [ ] `runtime-core` continues to expose the accepted channel contracts and the `CancelHandler` trait boundary.
- [ ] `runtime-core` no longer owns the standard concrete channel implementation.
- [ ] `runtime-channel` exposes `channel<T>() -> (impl Sender<T>, impl Receiver<T>)`.
- [ ] `runtime-channel` exposes `cancelable_channel<T>() -> (impl CancelHandler, impl CancelableSender<T>, impl CancelableReceiver<T>)`.
- [ ] Both factories hide concrete endpoint types behind `impl Trait`.
- [ ] The standard Tokio-backed implementation uses bounded `tokio::sync::mpsc`, not unbounded `mpsc`.
- [ ] Channel capacity is configurable through a `runtime-channel` configuration value.
- [ ] Initial crate documentation states how channel capacity is supplied and enforced.
- [ ] The standard implementation preserves the one-publisher, one-subscriber boundary.
- [ ] The standard implementation performs no blocking work before the returned future is polled or awaited.
- [ ] Cancellation state is observable through a shared flag.
- [ ] Cancellation wake-up uses `tokio::sync::watch` so pending awaits can be released when cancellation is signalled.
- [ ] Cancellation remains distinguishable from ordinary channel closure through the accepted `is_cancelled()` observation rule.
- [ ] `runtime-core` contains no concrete channel implementation logic or channel-behavior tests after the extraction.
- [ ] No `common` crate is introduced for runtime channel ownership.
- [ ] The workspace dependency policy records `tokio` as approved for `runtime-channel` only, unless a later RFC widens that scope.
- [ ] Follow-up tasking updates RFC 00003 or an equivalent channel contract document so factory ownership is no longer described as living in `runtime-core`.

## 6. Open Questions

- Should `runtime-channel` own only the standard channel factories, or should it later become the
  home for additional runtime-owned channel adapters?
- Should `runtime-channel` expose one default configuration constructor only, or also additional
  capacity-oriented constructors for callers that need more explicit control?
