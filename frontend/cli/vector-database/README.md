# `vector-database`

## 1. Objective

`vector-database` is the command-line interface (CLI) for executing package synchronization and managing repository package manifest mutations in the Vector workspace. It acts as the execution surface for the planning operations defined in `runtime-packages`.

It also exposes the Phase 6 local RAG store initialization command, the Phase 7
incremental indexing command, and the Phase 9 canonical retrieval context search
command by delegating `vector-database rag ...` invocations to the `vector-rag`
companion CLI.

## 2. Boundaries

### In scope

- Running package synchronization (`sync` command) which executes `git clone`, `git fetch`, and file copy operations.
- Interfacing with `runtime-packages` to add new dependencies into `.vector/packages.yaml` via CLI arguments.
- Triggering the RAG-owned LanceDB lifecycle operation through `vector-rag`.
- Running the Phase 7 incremental indexing pipeline via `rag update-database` through `vector-rag`.
- Running the RAG search command via `rag search` through `vector-rag`.
- Streaming subprocess execution logs and print messages before running actions.
- Rejecting invalid package structures (i.e. making sure synchronized packages contain `doc/` and `.vector/`).

### Out of scope

- Direct YAML editing (delegated to `runtime-packages`).
- Version resolution, semver parsing, or transitive dependency resolution.
- Package publishing or remote repository creation.

### Dependencies

| Dependency | Role |
|---|---|
| `runtime-packages` | Manifest contracts and `sync-packages` / `add-package` operations |
| `runtime-channel` | Standard dispatcher used to execute plugin operations |
| `runtime-io` | Terminal command execution, path helpers, and IO |
| `runtime-core` | Core runtime types and traits |
| `tokio` | Async runtime execution |
| `thiserror` | Custom error type formatting |

## 3. Commands

### `package sync`

Reads `.vector/packages.yaml`, determines the difference between the manifest and `.vector-database/packages/`, and executes the planning results.

**Execution Details:**
- Runs `git clone` if a package does not exist locally.
- Runs `git fetch` and updates if the Git package already exists.
- Runs copy operations for local file packages.
- Validates that every completed package contains a `doc/` directory and `.vector/` folder. If validation fails, the synchronized package path is deleted to prevent a corrupt cache.

**Exit behavior:**

| Condition | Exit code |
|---|---|
| All packages synchronized and validated successfully | `0` |
| Verification fails for any package | `1` |
| Execution errors (command failure, invalid manifest) | `1` |

### `package add`

Appends a new package dependency to `.vector/packages.yaml`.

**Arguments:**
- `<name>`: The unique name of the package.
- `<type>`: Either `git` or `file`.
- `<url>`: The repository URL or local file path.
- `[tag]`: The target reference. Required for Git (e.g. `v1.0.0` or `branch:main`), optional for `file`.

**Exit behavior:**

| Condition | Exit code |
|---|---|
| Package added and manifest saved successfully | `0` |
| Duplicate package name or validation failure | `1` |

### `rag update-database`

Runs the Phase 7 incremental indexing pipeline against the local workspace.
Initializes the LanceDB store if not already present, then indexes all governed
Markdown documents, skipping files whose content hash is unchanged.

**Execution Details:**
- Executes `vector-rag rag update-database` with the workspace root as the
  subprocess working directory.
- Streams `vector-rag` stdout and stderr without rewriting output.
- Supports `--json` to emit a final machine-readable payload with captured
  `progress` events plus the final `summary`.
- Returns the exact `vector-rag` exit status.
- Prints an install guidance error when `vector-rag` is not available on `PATH`.

**Arguments:**
- `--json`: Emit the final indexing contract as JSON instead of human-oriented
  progress lines and summary text.

**Output Contract:**
- Default output remains human-oriented streaming text for terminal users.
- `--json` returns one final JSON document with `progress[]` and `summary`.
- The project intentionally does not use NDJSON for indexing yet because the
  current MCP bridge returns a final tool result, not a streamed subprocess
  event channel. A final JSON payload keeps MCP consumption stable without
  forcing agents to parse human CLI text.
- Existing `rag update-database` users remain compatible because plain-text
  output is still the default path.

**Exit behavior:**

| Condition | Exit code |
|---|---|
| All documents indexed or skipped successfully | `0` |
| One or more documents failed during indexing | `1` |
| Dispatcher or operation error | `1` |

### `rag search`

Executes hybrid retrieval against the local RAG store through `vector-rag`.

**Execution Details:**
- Executes `vector-rag rag search ...` with the workspace root as the subprocess
  working directory.
- Streams `vector-rag` stdout and stderr without rewriting output.
- Returns the exact `vector-rag` exit status.
- Prints an install guidance error when `vector-rag` is not available on `PATH`.

**Arguments:**
- `<query>`: Required free-text query string.
- `--limit <n>`: Optional final result count override.
- `--package <name>`: Optional package filter.
- `--document <stem>`: Optional governed document stem filter.
- `--json`: Emit machine-readable JSON output.

**Exit behavior:**

| Condition | Exit code |
|---|---|
| Retrieval succeeds, including empty result sets | `0` |
| Store is missing, incompatible, or query execution fails | `1` |
| Argument parsing fails | `1` |

### `rag init`

Creates or validates the local Phase 6 LanceDB store under
`.vector-database/rag/lancedb/`.

**Execution Details:**
- Executes `vector-rag rag init` with the workspace root as the subprocess
  working directory.
- Streams `vector-rag` stdout and stderr without rewriting output.
- Returns the exact `vector-rag` exit status.
- Prints an install guidance error when `vector-rag` is not available on `PATH`.

**Phase 6 Store Contract:**
- The local retrieval store lives only under `.vector-database/rag/lancedb/`.
- The primary table persists one retrieval-oriented chunk row per embedded Markdown chunk.
- Persisted rows include `chunk_id`, governed package and document identity, document and chunk hashes, heading path, frontmatter, raw text, token count, embedding metadata, and the vector payload.
- `chunk_id` remains the deterministic upsert identity for replacing unchanged or updated chunks.
- Full-text indexing over `text` and vector indexing over `vector` are owned by `runtime-rag`, not by the CLI layer.

**Ownership Boundary:**
- `vector-database` is only the user-facing command surface for RAG commands.
- `vector-rag` owns command parsing and runtime execution for RAG commands.
- `runtime-rag` owns LanceDB compatibility validation, schema rules, index creation, and actionable persistence errors.

**Exit behavior:**

| Condition | Exit code |
|---|---|
| Store created, updated, or validated successfully | `0` |
| Store contract is incompatible or initialization fails | `1` |

## 4. Usage

```sh
# Synchronize all packages in the manifest
vector-database package sync

# Add a git-based package
vector-database package add my-pkg git https://github.com/org/my-pkg.git v1.0.0

# Add a branch-tracking git package
vector-database package add my-pkg git https://github.com/org/my-pkg.git branch:main

# Add a local file-based package
vector-database package add my-local file /absolute/path/to/source

# Create or validate the local RAG store
vector-database rag init

# Search the local RAG store with hybrid retrieval
vector-database rag search "hybrid retrieval"

# Filter hybrid retrieval to one package and emit JSON
vector-database rag search "hybrid retrieval" --package shared-docs --limit 3 --json

# Run the incremental indexing pipeline
vector-database rag update-database

# Capture a final machine-readable indexing result
vector-database rag update-database --json
```
