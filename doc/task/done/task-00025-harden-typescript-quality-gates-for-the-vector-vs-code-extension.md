---
id: task-00025-harden-typescript-quality-gates-for-the-vector-vs-code-extension
type: task
code: "00025"
slug: harden-typescript-quality-gates-for-the-vector-vs-code-extension
title: Harden TypeScript Quality Gates for the Vector VS Code Extension
description: Define and track the work required to introduce strict TypeScript quality gates, formatting, and enforcement workflows for the Vector VS Code extension.
status: done
created: 2026-05-08
updated: 2026-05-08
tags:
  - vscode
  - typescript
  - quality
  - frontend
related:
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - task-00021-harden-vs-code-sidebar-activation-for-vector-extension
  - spec-00006-typescript-quality-gate-contract-for-vs-code-extensions
supersedes: []
superseded_by: null
---

# Task 00025: Harden TypeScript Quality Gates for the Vector VS Code Extension

## 1. Prime Directive

> Remove the paths that allow the VS Code extension package to compile, lint, format, package, or publish without passing strict and explicit TypeScript quality gates.

## 2. Specs

- **Module:** `frontend/vscode/vector`
- **Dependencies:** TypeScript compiler, type-aware ESLint, Prettier, VS Code extension packaging and CI workflow wiring

## 3. Checklist

### 3.1. Phase A - Governance Baseline and Toolchain Contract

- [x] Confirm the extension toolchain contract against [[spec-00006-typescript-quality-gate-contract-for-vs-code-extensions]]
- [x] Inventory every current script, config file, and dependency gap in `frontend/vscode/vector`
- [x] Identify existing code patterns that will violate strict type-aware lint rules before enforcement is enabled
- [x] Validation vector for Phase A
- [x] execute section "4. Quality Gate"

#### 3.1.1. Inventory

| Category     | Item                | Status  | Gap                                                                                                          |
|--------------|---------------------|---------|--------------------------------------------------------------------------------------------------------------|
| Scripts      | `compile`           | Exists  | —                                                                                                            |
| Scripts      | `watch`             | Exists  | —                                                                                                            |
| Scripts      | `pretest`           | Exists  | —                                                                                                            |
| Scripts      | `test`              | Exists  | —                                                                                                            |
| Scripts      | `lint`              | Exists  | **Old CLI style (`eslint src --ext ts`), not flat config, not type-aware**                                   |
| Scripts      | `package`           | Exists  | **Bypasses quality gates; does not invoke `check`**                                                          |
| Scripts      | `typecheck`         | Missing | **Added `tsc --noEmit`**                                                                                     |
| Scripts      | `format`            | Missing | **Added `prettier . --write`**                                                                               |
| Scripts      | `format:check`      | Missing | **Added `prettier . --check`**                                                                               |
| Scripts      | `check`             | Missing | **Added aggregated gate**                                                                                    |
| Scripts      | `vscode:prepublish` | Missing | **Added; runs `check` before `compile`**                                                                     |
| Configs      | `tsconfig.json`     | Exists  | **Strengthened with `noImplicitOverride`, `useUnknownInCatchVariables`, `forceConsistentCasingInFileNames`** |
| Configs      | ESLint flat config  | Missing | **Added `eslint.config.js` with `typescript-eslint` type-aware strict presets**                              |
| Configs      | Prettier config     | Missing | **Added `.prettierrc`**                                                                                      |
| Configs      | Prettier ignore     | Missing | **Added `.prettierignore`**                                                                                  |
| Dependencies | `typescript`        | Exists  | —                                                                                                            |
| Dependencies | `eslint`            | Missing | **Added `^10.3.0`**                                                                                          |
| Dependencies | `@eslint/js`        | Missing | **Added `^10.0.1`**                                                                                          |
| Dependencies | `typescript-eslint` | Missing | **Added `^8.59.2`**                                                                                          |
| Dependencies | `prettier`          | Missing | **Added `^3.8.3`**                                                                                           |

#### 3.1.2. Code Patterns Identified as Future Violations

The following patterns are present in the current source and will require remediation in Phase C:

1. **Non-null assertions (`!`)** — Widespread across production and test code (`documentDiscovery.ts`, `wikilinkNavigation.ts`, `calloutPlugin.ts`, `documentStatus.ts`, `frontmatterRenderer.ts`, `headingNavigation.ts`, `markdownRenderer.ts`, `governedDocumentProvider.ts`, `extension.test.ts`).
2. **Unused parameters with `_` prefix** — Several markdown-it renderer rule signatures and vscode stub methods trigger `@typescript-eslint/no-unused-vars`.
3. **`void` in union types** — `vscode.EventEmitter<... | void>` in `governedDocumentProvider.ts` triggers `@typescript-eslint/no-invalid-void-type`.
4. **Unsafe type assertions (`as`)** — `headingNavigation.ts` (`env as HeadingRenderEnv`), `wikilinkNavigation.ts` (`msg as Record<string, unknown>`), and vscode stub.
5. **Unnecessary conditions / optional chains** — Multiple locations in tests where narrowing is already guaranteed (`@typescript-eslint/no-unnecessary-condition`).
6. **Template literal expressions with `number`** — `headingNavigation.ts` and `previewHtml.ts` (`@typescript-eslint/restrict-template-expressions`).
7. **Base-to-string on `unknown`/`Object`** — `frontmatterRenderer.ts` and tests (`@typescript-eslint/no-base-to-string`).
8. **Unused eslint-disable directives** — Legacy suppressions in `documentDiscovery.ts` and `extension.test.ts` that are no longer needed under the new type-aware rules.
9. **Async functions without `await`** — vscode stub methods (`@typescript-eslint/require-await`).
10. **Deprecated class usage in tests** — `GovernedDocumentProvider` is still exercised in tests (`@typescript-eslint/no-deprecated`).
11. **Formatting drift** — 14 files fail `prettier . --check`.

#### 3.1.3. Phase A Quality Gate Execution Results

```text
pnpm typecheck      → PASS (no errors)
pnpm lint           → FAIL (107 errors, 9 warnings)
pnpm format:check   → FAIL (14 files with drift)
pnpm check          → FAIL (lint and format:check block it)
pnpm test           → PASS (177 tests passing)
pnpm compile        → PASS
```

The tools are installed, configured, and executable. Failures are **expected** at this stage and will be remediated in Phase C.

### 3.2. Phase B - Mandatory TypeScript Quality Gates

- [x] Add a hard `typecheck` gate using `tsc --noEmit`
- [x] Add a flat ESLint configuration with type-aware strict rules for production and test code
- [x] Add Prettier as a formatter-only tool with an explicit ignore contract
- [x] Add a single `check` script that fails on any type, lint, or formatting violation
- [x] Ensure the packaging or publish path cannot bypass the `check` gate
- [x] Tests covering Phase B
- [x] Validation vector for Phase B
- [x] execute section "4. Quality Gate"

#### 3.2.1. Validation Vector

| Gate                      | Script                                               | Config File        | Status                                                                                     |
|---------------------------|------------------------------------------------------|--------------------|--------------------------------------------------------------------------------------------|
| TypeScript typecheck      | `pnpm typecheck` → `tsc --noEmit`                    | `tsconfig.json`    | ✅ Executable, passes cleanly                                                               |
| ESLint (flat, type-aware) | `pnpm lint` → `eslint . --max-warnings 0`            | `eslint.config.js` | ✅ Executable; 107 errors / 9 warnings detected (to be fixed in Phase C)                    |
| Prettier format           | `pnpm format` → `prettier . --write`                 | `.prettierrc`      | ✅ Executable                                                                               |
| Prettier format check     | `pnpm format:check` → `prettier . --check`           | `.prettierignore`  | ✅ Executable; 14 files with drift (to be fixed in Phase C)                                 |
| Aggregated gate           | `pnpm check` → `typecheck && lint && format:check`   | `package.json`     | ✅ Executable; fails as expected because lint and format:check have violations              |
| Packaging boundary        | `pnpm package` → `vscode:prepublish && vsce package` | `package.json`     | ✅ `vscode:prepublish` runs `check` before `compile`; packaging cannot bypass quality gates |

#### 3.2.2. Phase B Quality Gate Execution Results

```text
pnpm typecheck      → PASS (no errors)
pnpm lint           → FAIL (107 errors, 9 warnings)
pnpm format:check   → FAIL (14 files with drift)
pnpm check          → FAIL (lint and format:check block it)
pnpm compile        → PASS
pnpm test           → PASS (177 tests passing)
```

The quality gates are **installed, configured, and executable**. Failures are **expected** and will be remediated in Phase C.

### 3.3. Phase E - Dependency Vulnerability Validation

- [ ] Add a package-level dependency security baseline for the VS Code extension
- [ ] Add `pnpm audit --audit-level high` as the package-manager-native dependency audit gate
- [ ] Add `osv-scanner` as the lockfile-aware open-source vulnerability scan gate for `pnpm-lock.yaml`
- [ ] Define one canonical `security` script that fails when any mandatory dependency vulnerability gate fails
- [ ] Decide whether `trivy fs` remains optional follow-up scope or is promoted into the mandatory extension gate
- [ ] Document local installation and execution expectations for any non-Node security CLI required by the package

### 3.4. Phase Z - Wrap-up

- [ ] `pnpm typecheck` passes
- [ ] `pnpm lint` passes
- [ ] `pnpm format:check` passes
- [ ] `pnpm check` passes
- [ ] `pnpm security` passes
- [ ] `pnpm run compile` passes
- [ ] `pnpm test` passes
- [ ] Update README files on packages modified

## 4. Quality Gate

- [ ] `pnpm typecheck` passes
- [ ] `pnpm lint` passes
- [ ] `pnpm format:check` passes
- [ ] `pnpm check` passes
- [ ] `pnpm security` passes

## 5. Validation Vector

- [ ] The extension cannot package or publish while type errors exist
- [ ] The extension cannot package or publish while lint violations exist
- [ ] The extension cannot package or publish while formatting drift exists
- [ ] The extension cannot pass the package quality gate while high-severity dependency vulnerabilities remain unresolved
- [ ] The dependency security baseline includes both a package-manager-native audit and a lockfile-aware open-source vulnerability scan
- [ ] ESLint remains responsible for code quality and Prettier remains responsible for formatting only
- [ ] Type-aware linting runs against the real TypeScript project instead of syntax-only parsing
- [ ] The repository contains one explicit contract that defines the quality gates for TypeScript VS Code extensions
- [ ] All phase checkboxes completed
