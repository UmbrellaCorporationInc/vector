---
id: spec-00006-typescript-quality-gate-contract-for-vs-code-extensions
type: spec
code: "00006"
slug: typescript-quality-gate-contract-for-vs-code-extensions
title: TypeScript Quality Gate Contract for VS Code Extensions
description: Defines mandatory quality gates, tool boundaries, and enforcement rules for TypeScript-based VS Code extension packages in the repository.
category: contract
created: 2026-05-08
updated: 2026-05-08
authors: []
tags:
  - typescript
  - vscode
  - quality
  - contract
  - frontend
related:
  - rfc-00014-vs-code-governed-documents-sidebar-extension
  - spec-00001-repository-directory-structure
supersedes: []
superseded_by: null
aliases:
  - "SPEC 00006: TypeScript Quality Gate Contract for VS Code Extensions"
---

# SPEC 00006: TypeScript Quality Gate Contract for VS Code Extensions

## 1. Purpose

This spec defines the mandatory quality gate contract for any repository package that ships a VS Code extension implemented in TypeScript.

The contract exists to prevent extension packages from compiling, bundling, packaging, or publishing while type safety, lint quality, or formatting integrity are broken.

## 2. Definition

A TypeScript-based VS Code extension package is any package that:

- declares a VS Code extension manifest through `package.json`
- produces an extension entrypoint consumed by VS Code
- contains TypeScript source that is compiled or bundled into the extension runtime artifact

Every such package must define four quality layers:

- one TypeScript type-correctness layer
- one ESLint code-quality layer
- one Prettier formatting layer
- one dependency vulnerability validation layer
- one non-optional enforcement layer for local and automated execution

### 2.1. Type-correctness layer

The package must expose a dedicated script named `typecheck`.

The `typecheck` script must run:

```text
tsc --noEmit
```

or an equivalent command that performs full project type-checking without producing build artifacts.

When the package uses a bundler such as `esbuild`, the bundler must not be treated as a substitute for `typecheck`.

### 2.2. ESLint layer

The package must expose a dedicated script named `lint`.

The `lint` script must execute ESLint against the TypeScript project with type-aware analysis enabled from the real project configuration.

The ESLint configuration must:

- use flat config
- enable `typescript-eslint` type-aware strict rules for production code
- define explicit handling for test files when test ergonomics differ from production code
- fail the command on errors

The ESLint layer owns code quality concerns such as:

- unsafe `any` usage
- unsafe assignment, member access, calls, and returns
- floating promises
- misuse of async APIs
- import discipline
- explicitness of public boundaries when required by the package contract
- maintainability rules such as bounded complexity where adopted

### 2.3. Prettier layer

The package must expose:

- one script named `format`
- one script named `format:check`

The package must define a Prettier configuration and a Prettier ignore file or equivalent ignore contract.

Prettier owns formatting only. ESLint must not be used as the primary formatting engine for the package.

### 2.4. Aggregated gate

The package must expose one canonical script named `check`.

`check` must fail if any of the following fail:

- `typecheck`
- `lint`
- `format:check`

The order may vary, but the three component gates are mandatory.

### 2.5. Dependency vulnerability validation layer

The package must expose one canonical script named `security`.

The `security` script must execute at least two complementary dependency vulnerability checks:

- one package-manager-native audit for the package dependency graph
- one lockfile-aware open-source vulnerability scanner

For `pnpm`-managed TypeScript VS Code extension packages, the minimum accepted baseline is:

- `pnpm audit --audit-level high`
- `osv-scanner` against the package root or `pnpm-lock.yaml`

The dependency vulnerability validation layer exists to catch vulnerable transitive and direct dependencies that are not surfaced by type, lint, or formatting gates.

`Trivy` may be added as a broader repository or filesystem scanner, but it is not required as part of the minimum contract for a package-scoped VS Code extension baseline unless a later spec revision promotes it.

### 2.6. Packaging and publishing boundary

Any package script that prepares the extension for packaging, bundling, or publishing must depend on the canonical `check` gate before generating the release artifact.

Valid examples include:

- `vscode:prepublish` running `check` before `compile`
- `vscode:prepublish` running `check` before `bundle`
- package workflows that call `check` before `vsce package`

Invalid examples include:

- packaging the extension directly from `compile` alone
- treating a bundler pass as equivalent to type checking
- allowing publish or package scripts to bypass lint or formatting validation

The package should also ensure that mandatory release or CI paths cannot silently ignore the `security` script when dependency vulnerability validation is part of the accepted package policy.

### 2.7. TypeScript configuration baseline

The package TypeScript configuration must use explicit modern module semantics compatible with Node-hosted VS Code extensions.

Minimum required compiler settings:

- `module` set to `Node16` or a stricter accepted successor
- `moduleResolution` set to `Node16` or a stricter accepted successor
- `strict` enabled
- `isolatedModules` enabled
- `verbatimModuleSyntax` enabled

Strongly expected compiler settings unless a documented package exception exists:

- `noImplicitOverride`
- `noImplicitReturns`
- `noFallthroughCasesInSwitch`
- `noUncheckedIndexedAccess`
- `exactOptionalPropertyTypes`
- `useUnknownInCatchVariables`
- `forceConsistentCasingInFileNames`

### 2.8. Source code hygiene expectations

TypeScript VS Code extension code should follow these additional expectations:

- use `import type` when importing type-only bindings
- prefer `unknown` over `any` at external boundaries
- isolate direct VS Code API calls behind small adapters when that improves testability or reduces repeated side effects
- avoid top-level side effects outside accepted extension bootstrap paths
- use exhaustive branching for closed unions where the package owns the domain contract

These expectations may be enforced partly through lint rules, partly through code review, and partly through targeted follow-up tasks.

### 2.9. Non-optional enforcement layer

Every TypeScript VS Code extension package must make the quality gate executable in both local and automated contexts.

The package must define at least one of the following:

- CI execution of the canonical `check` script
- a repository-level orchestrator that invokes the package `check` script as part of mandatory validation

If both exist, the package-level `check` script remains the canonical contract.

## 3. Invariants

- Every TypeScript VS Code extension package must expose a `typecheck` script.
- `typecheck` must perform type checking without relying on bundler-only transpilation.
- Every TypeScript VS Code extension package must expose a `lint` script.
- `lint` must run with type-aware project analysis, not syntax-only parsing.
- Every TypeScript VS Code extension package must expose `format` and `format:check` scripts.
- Every TypeScript VS Code extension package must expose a canonical `security` script.
- `security` must include one package-manager-native dependency audit and one lockfile-aware open-source vulnerability scan.
- For `pnpm`-managed packages, the minimum accepted dependency security baseline is `pnpm audit --audit-level high` plus `osv-scanner`.
- Every TypeScript VS Code extension package must expose a canonical `check` script.
- `check` must fail when `typecheck`, `lint`, or `format:check` fail.
- Packaging, prepublish, bundle, or publish flows must not bypass `check`.
- ESLint owns code quality; Prettier owns formatting.
- Prettier must not be enforced primarily by running it as an ESLint rule.
- Bundling success must not be treated as evidence of type correctness.
- Dependency vulnerability validation must not be treated as satisfied by type-checking, linting, formatting, or bundling success.
- The TypeScript project must use explicit modern module semantics suitable for the VS Code Node runtime.
- Type-aware linting exceptions must be explicit and localized rather than disabled broadly across the package.
- Test-specific lint relaxations must be scoped to test files only.

## 4. Examples

Valid script contract example:

```json
{
  "scripts": {
    "typecheck": "tsc --noEmit",
    "lint": "eslint . --max-warnings 0",
    "format": "prettier . --write",
    "format:check": "prettier . --check",
    "security": "pnpm audit --audit-level high && osv-scanner scan .",
    "check": "pnpm typecheck && pnpm lint && pnpm format:check",
    "compile": "tsc -p ./",
    "vscode:prepublish": "pnpm check && pnpm compile"
  }
}
```

Valid TypeScript configuration baseline example:

```json
{
  "compilerOptions": {
    "module": "Node16",
    "moduleResolution": "Node16",
    "strict": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "noImplicitReturns": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true
  }
}
```

Invalid examples:

- a package that defines `compile` but not `typecheck`
- a package that bundles with `esbuild` and assumes bundling replaces `tsc --noEmit`
- a package that runs ESLint without project-aware type analysis
- a package that formats only through an ESLint Prettier rule and has no `format:check` contract
- a package that has no package-level dependency vulnerability scan contract
- a package that relies only on `pnpm audit` and omits a lockfile-aware open-source vulnerability scanner
- a package that allows `vsce package` to run without the canonical `check` script
- a package that disables unsafe-type rules globally instead of scoping exceptions to the few files that need them

## 5. Open Questions

- Should the repository later define one shared reusable ESLint preset for all TypeScript extension packages instead of repeating package-local flat configs?
- Should CI ownership live inside each extension package or in one repository-root workflow contract when multiple frontend packages are introduced?
- Should `Trivy` be promoted from optional broader scanner to mandatory package baseline once the repository adds more non-Node assets such as containers, workflows, or IaC?
