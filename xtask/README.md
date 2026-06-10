# `xtask`

Build and quality automation for the Forge workspace. Provides CLI commands that run cargo checks (tests, clippy, fmt) with triple-consumable output (human, script, agent) per ADR 0014 (Code Quality Metrics and Reports), plus vault-document resolution and reservation per ADR 0082, and vault frontmatter query per ADR 0085.

---

## Usage

Run from the workspace root:

```bash
cargo <alias> [options]
# e.g. cargo quality-test
# e.g. cargo quality-lint
# e.g. cargo quality-lint --report
```

### Commands (via cargo aliases)

| Alias | Description |
|-------|-------------|
| `cargo quality` | Runs the full pipeline: `cargo fmt --all --check` → `cargo quality-lint` → `cargo quality-test`. Prints each step's report inline, then a summary table with per-step status and total duration. Exit 0 only if all three steps pass. Use `--format` to apply formatting instead of only checking. |
| `cargo quality-report` | Same as `quality`; additionally writes `quality-report.txt` to the workspace root. |
| `cargo quality-stats` | Runs the full `quality` pipeline, then appends a **Statistics** section (default: `tokei` + `cargo tree`). Use `--module-structure <crate>` to run `cargo modules structure` for a specific crate instead. Missing tools degrade gracefully. Exit 0 only if the quality pipeline passes. |
| `cargo quality-stats-report` | Same as `quality-stats`; additionally writes `quality-stats-report.txt` to the workspace root. |
| `cargo quality-test` | Runs `cargo test --workspace`, then (if tests pass) `cargo llvm-cov --json --summary-only`. **Test summary:** ASCII table by crate and type (lib, bin, integration) with Passed, Failed, Ignored, Measured, Filtered. **Coverage:** per-file table (Func%, Line%, Region%, Branch%); by default only files with any metric below threshold + TOTAL row. Use `--verbose` for full test output. |
| `cargo quality-test-report` | Same as `quality-test`; additionally writes `quality-test-report.txt` to the workspace root. |
| `cargo quality-lint` | Runs `cargo clippy --workspace --all-targets --all-features -- -D warnings` followed by project-local lint rules (RULE-*). With `--markdown`, runs Markdown UTF-8 / mojibake validation instead of Rust clippy lint. **Output:** timestamp, pass/fail status, summary (including rule violation count), diagnostic blocks on failure. Exit 0 only if both clippy and all standard rules pass. |
| `cargo quality-lint-report` | Same as `quality-lint`; additionally writes `quality-lint-report.txt` to the workspace root. |

### Direct `xtask` commands

Run from the workspace root:

```bash
cargo run -p xtask -- vault <subcommand>
```

Supported vault commands:

- `cargo run -p xtask -- vault task [number]`
- `cargo run -p xtask -- vault adr [number]`
- `cargo run -p xtask -- vault guide [number]`
- `cargo run -p xtask -- vault rule [number]`
- `cargo run -p xtask -- vault research [number]`
- `cargo run -p xtask -- vault roadmap [number]`
- `cargo run -p xtask -- vault reserve <type> "<name>" [--subfolder <name>]`
- `cargo run -p xtask -- vault reserve-book "<name>"` — ADR 0104: reserves the next three-digit book code and creates `forge/50 Books/NNN <name>/00 Index.md`
- `cargo run -p xtask -- vault reserve-book-artifact <book-code> "<name>" --category <name>` — ADR 0104: reserves the next four-digit artifact id scoped to the given book and creates the file at `forge/50 Books/NNN Name/<category>/bNNNN <book> <title>.md`
- `cargo run -p xtask -- vault check [--fix]`
- `cargo run -p xtask -- vault query [EXPRESSION] [--query-id NAME]` — ADR 0085: walks the same governed `.md` trees as `vault check`, evaluates the expression against YAML frontmatter, prints one **`category,relative/path/to/file.md`** line per match (forward slashes). Omit both `EXPRESSION` and `--query-id` to exit 0 with no output. Use **`--query-id`** to load the expression string from `.cargo/.xtask/vault-query.yaml` (YAML map: id → expression); it cannot be combined with a positional expression. **List predicates:** `field=['a','b']` matches if **any** listed value appears in the field (OR); `field={'a','b'}` requires **every** listed value (AND). Parse errors and I/O/YAML failures exit 1 with `ERROR=...` on **stderr**; matches go to **stdout** only.

Notes:

- Vault roots are configured in `.cargo/.xtask/vault.yaml`.
- Resolution walks the configured category tree and matches by numeric filename prefix.
- Calling a category command without an id returns category metadata: root `PATH`, `LAST_ID`, and `SUBFOLDERS`.
- `research` numbering is global across the full `doc/43 Research` tree.
- `roadmap` numbering is global across the full `doc/44 Roadmaps` tree.
- `SUBFOLDERS` are discovered dynamically by listing direct child directories of the configured category root — the filesystem is the source of truth, not a hardcoded list.
- `reserve` requires `--subfolder` when the target category root contains subfolders.
- `reserve`, `reserve-book`, and `reserve-book-artifact` validate the supplied name for OS filesystem compatibility before constructing any path. Names containing `< > : " / \ | ? *`, ending with a space or dot, or matching a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`) are rejected with `ERROR=…` and exit code 1.
- `task` reserves into `todo` by default and `adr` reserves into `draft` by default when `--subfolder` is omitted.
- `check` validates required frontmatter fields across governed vault categories.
- For non-book governed files, the canonical filename surface is `<sequence> <type> <title>.md`.
- For book artifacts, the canonical filename surface is `b<sequence> <book> <title>.md`.
- For files named with a numeric prefix, `check` requires frontmatter `id` as a YAML number equal to the filename prefix with **no leading zeros** (e.g. `00104 task …` → `id: 104`). Files without a parseable numeric prefix skip this rule.
- `check --fix` applies deterministic defaults for missing `type`, missing `created`, missing task `status`, missing ADR `status`, and missing `id` when the filename supplies one. It does **not** overwrite a wrong `id` (reports `INVALID_FIELDS … id` instead).
- For `task` and `adr`, `check` also validates that the file lives in the status-derived direct subfolder.
- For `task` and `adr`, `check --fix` creates missing status subfolders and moves files into the correct status-derived location.
- `check` reports governed internal markdown links, aliased wiki links, and `.md`-suffixed governed wiki links as invalid.
- `check --fix` rewrites deterministic governed internal links to canonical wiki links and preserves `#anchor` fragments when possible.

**Category and subfolder openness:**

| Category | Subfolders | Openness |
|----------|------------|----------|
| `task` | `todo`, `in-progress`, `done` | **Closed** — subfolders are workflow states. `check --fix` moves files based on `status` frontmatter; adding a new folder changes the lifecycle model and requires updating `check` logic. |
| `adr` | `draft`, `execution`, `implemented` | **Closed** — subfolders are ADR lifecycle stages. `check --fix` enforces placement by `status` frontmatter. Same constraint as `task`. |
| `research` | `frameworks`, `language`, `methodology`, … | **Open** — subfolders are a knowledge taxonomy. Add a new topic area by creating the folder on disk; it becomes a valid `--subfolder` target immediately on the next run. |
| `roadmap` | `language`, `platform`, … | **Open** — subfolders are a planning taxonomy. Add a new roadmap domain by creating the folder on disk; it becomes a valid `--subfolder` target immediately on the next run. |
| `guide` | *(none)* | Flat — no subfolders; `reserve` places files directly in the root. |
| `rule` | *(none)* | Flat — no subfolders; `reserve` places files directly in the root. |

**Note:** the set of recognized *categories* (`task`, `adr`, `research`, `roadmap`, `guide`, `rule`) is hardcoded in the `VaultDocumentType` enum in `xtask/src/vault.rs`. Adding a new category requires modifying that source file. Subfolders, by contrast, are open for `research` and `roadmap` — no code changes needed.

### Options (`quality-test`)

| Option | Default | Description |
|--------|---------|-------------|
| `--report` | false | Write report to `quality-test-report.txt`. |
| `--include-ignore` | false | Also run tests marked `#[ignore]` (uses `cargo test -- --include-ignored`). |
| `--no-coverage` | false | Skip coverage run; only run tests. |
| `--verbose` | false | Show full test output (default: header + failures only). |
| `--coverage-threshold N` | 90 | Threshold (%). Files with any metric below this appear in the default coverage table. |
| `--complete-coverage-summary` | false | Show full per-file coverage table; default is only files below threshold + TOTAL. |
| `--with-cov-details` | false | Print per-file uncovered line/column ranges before the coverage summary table. Forces full JSON output from `cargo llvm-cov` (slightly slower). |
| `--crate-coverage <CRATE>` | — | Scope tests and coverage to a single crate. Always shows uncovered segments + full coverage table for that crate. Ignores `--no-coverage`. |
| `-p <PACKAGE>` / `--package <PACKAGE>` | — | Scope tests and coverage to a single package (mirrors `cargo test -p` / `cargo llvm-cov --package`). All other flags remain independent. When combined with `--crate-coverage`, `--crate-coverage` takes precedence. |

**Agents:** Run `cargo quality-test` without `--verbose`. Use `-p <package>` to filter a single crate; use `--crate-coverage <crate>` when you also want forced full-detail coverage.

### Example output

```
=== Quality Test Report ===
Timestamp: 2026-03-12 13:54:10 UTC
Workspace: C:\path\to\forge
Test Duration: 1.59s
Status: PASS
Summary:
| Crate      | Type        | Passed | Failed | Ignored | Measured | Filtered |
|------------|-------------|--------|--------|---------|----------|----------|
| docgen     | lib         |     63 |      0 |       0 |        0 |        0 |
| cli        | integration |     18 |      0 |       1 |        0 |        0 |
...
| TOTAL      |             |    259 |      0 |       3 |        0 |        0 |

=== Coverage Report (per-file: function, line, region, branch) ===
Duration: 8.43s
Threshold: 90% (N/A = - = 100%)

| File                         | Func% | Line% | Region% | Branch% |
|------------------------------|-------|-------|---------|---------|
| xtask\src\main.rs            |  0.0% |  0.0% |    0.0% |       - |
| cli\docgen\src\main.rs       | 54.5% | 75.8% |   65.6% |       - |
...
|------------------------------|-------|-------|---------|---------|
| TOTAL                        | 85.7% | 86.0% |   87.6% |       - |

Legend: - = N/A (100%).
```

When `--with-cov-details` is passed the coverage section includes an uncovered-segments block before the summary table:

```
=== Coverage Report (per-file: function, line, region, branch) ===
Duration: 9.12s
Threshold: 90% (N/A = - = 100%)

--- Uncovered segments per file ---
  runtime\fs\src\disk.rs:
    L42:5 - L43:1
    L57:3-15
  runtime\language\src\lexer.rs:
    L105:3-8
    L210:1 - L212:20

--- Coverage Summary ---
| File                         | Func%   | Line%   | Region%  | Branch%  |
...
```

Exit code: 0 if all tests pass, 1 otherwise.

### Options (`quality`)

| Option | Default | Description |
|--------|---------|-------------|
| `--report` | false | Write combined report to `quality-report.txt`. |
| `--format` | false | Apply `cargo fmt --all` as the first step (default: check only with `--check`). |

### Example output (`quality`)

```
=== Quality Report ===
Timestamp: 2026-03-13 01:26:51 UTC
Workspace: C:\path\to\forge

=== Format Check ===
Duration: 0.22s
Status: PASS

=== Quality Lint Report ===
...

=== Quality Test Report ===
...

=== Quality Summary ===
Format: PASS  (0.22s)
Lint:   PASS  (0.78s)
Tests:  PASS  (12.36s)
───────────────────────
Result: PASS  (total: 13.36s)
```

Exit code: 0 if all three steps pass, 1 otherwise.

### Options (`quality-stats`)

| Option | Default | Description |
|--------|---------|-------------|
| `--report` | false | Write combined report to `quality-stats-report.txt`. |
| `--module-structure <CRATE>` | — | Run ONLY `cargo modules structure -p <CRATE>` instead of the default stats. |

**Modes:**
- No flag → default stats: `tokei` (install: `cargo install tokei`) + `cargo tree --workspace`
- `--module-structure <crate>` → only `cargo modules structure -p <crate>` (install: `cargo install cargo-modules`)

Missing tools are noted with an install hint and do not abort the command.

### Options (`quality-lint`)

| Option | Default | Description |
|--------|---------|-------------|
| `--report` | false | Write report to `quality-lint-report.txt`. |
| `--markdown` | false | Run Markdown UTF-8 / mojibake validation instead of Rust clippy lint. Conflicts with `-p/--package` and `--future`. |
| `--future` | false | Additionally evaluate informational "future" rules (RULE-F*). These appear in a separate section and do not affect the pass/fail status. Conflicts with `--markdown`. |
| `-p <PACKAGE>` / `--package <PACKAGE>` | — | Scope clippy to a single package (mirrors `cargo clippy -p`). Project-local rules still run across the workspace. Omits `--workspace`. |

### Example output (`quality-lint`)

```
=== Quality Lint Report ===
Timestamp: 2026-03-13 01:00:00 UTC
Workspace: C:\path\to\forge
Lint Duration: 4.21s
Status: PASS
Summary: 0 warnings, 0 errors
```

On failure:

```
=== Quality Lint Report ===
Timestamp: 2026-03-13 01:00:00 UTC
Workspace: C:\path\to\forge
Lint Duration: 4.21s
Status: FAIL
Summary: 1 warning(s), 0 error(s)

--- Diagnostics ---
warning: unused variable `x`
  --> runtime\io\src\disk.rs:42:9
   |
42 |         let x = 5;
   |             ^ help: if this is intentional, prefix it with an underscore: `_x`
```

Exit code: 0 if zero warnings/errors, 1 otherwise.

Markdown mode:

```
=== Quality Lint Report ===
Timestamp: 2026-03-13 01:00:00 UTC
Workspace: C:\path\to\forge
Lint Duration: 0.04s
Mode: markdown
Status: FAIL
Summary: 2 finding(s)

--- Diagnostics ---
doc/00 Project Hub/Tasks/00086 Babel Exhaustiveness and Unification Hardening.md:7: suspicious mojibake marker `Ã`
doc/20 Architecture/0058 Babel Impl Coherence and Generic Impl Headers.md:12: suspicious mojibake marker `Â`
```

---

## Dependencies

| Crate | Role |
|-------|------|
| `clap` | CLI argument parsing and subcommand routing |
| `chrono` | Timestamp formatting for report headers |
| `serde_json` | Parsing cargo-llvm-cov JSON for coverage table |
| `serde` | Loading structured xtask configuration |
| `serde_yaml` | Parsing `.cargo/.xtask/vault.yaml` |

---

## Project-Local Lint Rules (`lint_rules`)

`cargo quality-lint` runs clippy and then evaluates project-local rules against the workspace. Rules fall into two modes:

- **Standard** — always active; a violation fails the run.
- **Future** — active only under `--future`; violations appear in a separate `--- future violations ---` section and do not affect the exit code. Used during the migration window before a rule graduates to standard enforcement.

### Graduation path

A future rule graduates to standard once the workspace clears all its violations. Update `is_active` in the rule struct to always return `true` and remove the `is_future` field.

### Active standard rules

| Rule ID | File | Description |
|---------|------|-------------|
| RULE-5 | `no_tuple_in_signature.rs` | Function signatures must not use tuples with 2+ elements; use a named type instead. |
| RULE-7 | `no_to_string_in_map_err.rs` | `.map_err(\|e\| e.to_string())` is forbidden; propagate typed errors or use `thiserror`. |
| RULE-10 | `no_pub_struct_fields.rs` | `pub` struct fields outside DTOs are forbidden; use private fields with accessors. |
| RULE-12 | `no_std_process_command.rs` | `std::process::Command` is forbidden outside the Shell subsystem. |
| RULE-13A | `no_inline_tests.rs` | `#[cfg(test)]` mod with an inline body in a non-test file is forbidden; use a sibling `_test.rs`. |
| RULE-13B | `no_inline_tests.rs` | The test module identifier must be exactly `tests`. |
| RULE-13C | `no_inline_tests.rs` | The `#[path]` value must be `<stem>_test.rs`. |
| RULE-13D | `no_inline_tests.rs` | Every non-exempt source file must declare a `#[cfg(test)] mod tests`. Exempt: `main.rs`, `lib.rs`, `mod.rs`, `*_test.rs`, files under `tests/`. |
| RULE-14 | `no_allow_outside_test.rs` | `#[allow(...)]` attributes are forbidden outside test modules. |
| RULE-15 | `no_workspace_dependency.rs` | Workspace-level `[dependencies]` without a version specifier are forbidden. |
| RULE-27 | `aggregator_only_exports.rs` | `lib.rs` and `mod.rs` must contain only `mod`, `use`, and `extern crate` declarations; no implementation items. |

### Active future rules

| Rule ID | File | Description | Graduation condition |
|---------|------|-------------|----------------------|
| RULE-26 | `file_too_long.rs` | A source file must not exceed 300 code lines (blank and comment lines excluded). Exempt: `*_test.rs`, `main.rs`, `lib.rs`, `mod.rs`. | Workspace clears all violations. |
