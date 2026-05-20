---
id: task-00009-implement-rfc-00009-runtime-io-file-access-and-shell-command-execution
type: task
code: "00009"
slug: implement-rfc-00009-runtime-io-file-access-and-shell-command-execution
title: Implement RFC 00009 runtime IO file access and shell command execution
description: Implement the runtime-io crate from RFC 00009 with file, memory, text, path, and shell command IO boundaries over runtime-core sender and receiver contracts.
status: done
created: 2026-05-03
updated: 2026-05-03
tags:
  - runtime
  - io
  - file
  - path
  - shell
related:
  - rfc-00009-runtime-io-file-access-and-shell-command-execution
  - rfc-00006-runtime-core-control-observability-and-encoding-primitives
  - rfc-00003-runtime-core-channel-contracts-and-cancel-aware-specializations
  - rfc-00002-runtime-core-v1-boundary-and-async-first-contracts
supersedes: []
superseded_by: null
---

# Task 00009: Implement RFC 00009 runtime IO file access and shell command execution

## 1. Prime Directive

Create one concrete `runtime-io` crate that reuses `runtime-core` sender and receiver contracts for files, memory, UTF-8 text adaptation, paths, and shell process IO so higher-level crates stop inventing incompatible streaming and resource boundaries.

## 2. Specs

- **Module:** `runtime/io`
- **Dependencies:** Rust `std`, `thiserror`, `runtime-core`, selected async runtime backend

## 3. Checklist

### 3.1. Phase A - Crate skeleton and error surface

- [x] Add the new `runtime/io` crate to the workspace
- [x] Define the public module layout for file, memory, text, path, command, and error concerns without generic helper buckets
- [x] Define the typed `runtime-io` error surface for file, path, UTF-8 text, and process failures
- [x] Re-export the accepted public API from the crate root
- [x] Add tests covering crate-level error construction and public API visibility
- [x] execute section "4. Quality Gate"

### 3.2. Phase B - File-backed byte IO

- [x] Implement `FileReader` as a file-backed `Receiver<Bytes>`
- [x] Implement `FileWriter` as a file-backed `Sender<Bytes>`
- [x] Accept caller-supplied buffer sizes for both file-backed types
- [x] Implement `Drop` for `FileReader` and `FileWriter`
- [x] Add tests covering file open, read, write, end-of-stream, and buffered behavior
- [x] execute section "4. Quality Gate"

### 3.3. Phase C - Memory-backed byte IO

- [x] Implement `MemReader` as a memory-backed `Receiver<Bytes>`
- [x] Implement `MemWriter` as a memory-backed `Sender<Bytes>`
- [x] Accept caller-supplied buffer sizes for both memory-backed types when required by the accepted API
- [x] Expose the accepted memory extraction or inspection API for collected bytes
- [x] Add tests covering deterministic memory-backed read and write behavior
- [x] execute section "4. Quality Gate"

### 3.4. Phase D - UTF-8 text adapters

- [x] Implement `TextReader` as an adapter from `Receiver<Bytes>` to text output
- [x] Implement `TextWriter` as an adapter from text input to `Sender<Bytes>`
- [x] Use `Encoding` from `runtime-core` for UTF-8 conversion
- [x] Accept caller-supplied buffer sizes for both text adapters
- [x] Preserve incomplete trailing UTF-8 bytes until a valid text boundary can be formed or decoding fails
- [x] Add tests covering valid UTF-8, invalid UTF-8, split multi-byte sequences, and buffered text emission
- [x] execute section "4. Quality Gate"

### 3.5. Phase E - Path API

- [x] Implement the accepted path API over Rust path handling
- [x] Support path construction, joining, and conversion back to `std::path::Path`
- [x] Integrate the path API with file-backed readers and writers
- [x] Add tests covering stable path composition and file IO integration
- [x] execute section "4. Quality Gate"

### 3.6. Phase F - File convenience helpers

- [x] Implement full-buffer byte helpers over the accepted file-backed reader and writer model
- [x] Implement full-buffer text helpers over `TextReader` and `TextWriter`
- [x] Ensure convenience helpers remain adapters over the streaming boundary rather than a separate implementation path
- [x] Add tests covering full file text and byte round-trips
- [x] execute section "4. Quality Gate"

### 3.7. Phase G - Shell command builder and process handle

- [x] Implement `CommandBuilder` with explicit command, ordered arguments, optional working directory, and environment configuration
- [x] Keep stdin configuration out of the builder surface
- [x] Implement `CommandOutput` as a concrete process output type that satisfies `Receiver<Bytes>`
- [x] Implement `CommandInput` as a concrete process input type that satisfies `Sender<Bytes>`
- [x] Implement `CommandHandle` with `stdout`, `stderr`, `stdin`, `wait`, and `Drop`
- [x] Add tests covering process spawn, stdout and stderr consumption, stdin publication, completion, and cleanup behavior
- [x] execute section "4. Quality Gate"

### 3.8. Phase H - Public API integration and docs

- [x] Re-export the accepted `runtime-io` API from the crate root
- [x] Add README documentation for file, memory, text, path, and shell IO boundaries
- [x] Verify the public API introduces no shell parser, structured text parser, retry policy, scheduling policy, or repository-specific workflow logic
- [x] execute section "4. Quality Gate"

### 3.9. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [x] `xtask quality --format` passes
- [x] Update README files on packages modified

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [x] All phase checkboxes completed
- [x] `runtime-io` reuses `runtime-core` sender and receiver contracts instead of creating a second streaming abstraction family
- [x] `FileReader`, `MemReader`, and `CommandOutput` satisfy `Receiver<Bytes>`
- [x] `FileWriter`, `MemWriter`, and `CommandInput` satisfy `Sender<Bytes>`
- [x] `TextReader` and `TextWriter` apply UTF-8 buffering discipline only at the text adapter layer
- [x] Raw byte readers and writers remain valid for arbitrary binary content and process IO
- [x] File-backed implementations accept caller-supplied buffer sizes and implement `Drop`
- [x] `CommandHandle` exposes concrete process IO types and implements `Drop`
- [x] Full-buffer file helpers remain adapters over the streaming model rather than a separate IO stack
- [x] The public API introduces no shell parser, structured text parser, retry policy, scheduling policy, or repository-specific workflow logic
- [x] All quality gates pass
  - [x] `xtask quality-lint` passes
  - [x] `xtask quality-test` passes
