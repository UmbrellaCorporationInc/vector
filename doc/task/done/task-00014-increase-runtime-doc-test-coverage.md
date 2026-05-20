---
id: task-00014-increase-runtime-doc-test-coverage
type: task
code: "00014"
slug: increase-runtime-doc-test-coverage
title: Increase Runtime Doc Test Coverage
description: Add focused tests for uncovered runtime-doc configuration and loader branches to improve confidence without changing production behavior.
status: done
created: 2026-05-05
updated: 2026-05-05
tags:
  - runtime
  - doc
  - testing
related:
  - task-00013-build-runtime-doc-crate
supersedes: []
superseded_by: null
---

# Task 00014: Increase Runtime Doc Test Coverage

## 1. Prime Directive

Raise confidence in `runtime-doc` by covering real uncovered validation and loading branches instead of inflating the suite with duplicate happy-path tests.

## 2. Specs

- **Module:** `runtime/doc`
- **Dependencies:** existing test stack only

## 3. Checklist

### 3.1. Phase A - Coverage gaps in configuration and loader tests

- [x] Cover `DocumentTypeConfig::validate` for accepted and rejected configurations
- [x] Cover deserialization of optional `tags` fields
- [x] Cover loader rejection when `filename_pattern` is present
- [x] Cover `load_from_path` success and failure paths
- [x] Replace duplicated `create_doc` test logic with tests that execute `CreateDocOp`
- [x] Execute section "4. Quality Gate"

### 3.2. Phase Z - Wrap-up

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes
- [ ] `xtask quality --format` passes
- [ ] Update README files on packages modified when required

## 4. Quality Gate

- [x] `xtask quality-lint` passes
- [x] `xtask quality-test` passes

## 5. Validation Vector

- [ ] All phase checkboxes completed
- [x] Added tests cover rejected config branches, not only happy paths
- [x] Added tests do not modify production behavior
- [ ] All quality gates pass
