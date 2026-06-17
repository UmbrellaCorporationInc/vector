# `vector-rag`

## 1. Objective

`vector-rag` is the companion CLI for local RAG runtime execution in the Vector
workspace. It owns command parsing and direct runtime delegation for all RAG
operations: store initialization, incremental indexing, and hybrid retrieval.

`vector-database` delegates its `rag ...` commands to `vector-rag` as a
subprocess. Callers that consume RAG operations through the `vector-database`
passthrough should treat `vector-rag` as an internal implementation detail.

## 2. Boundaries

### In scope

- Parsing and dispatching `rag init`, `rag search`, and `rag update-database`
  subcommands.
- Discovering the workspace root by walking up the directory tree until a
  `.vector/` directory is found.
- Delegating directly to `runtime-rag` operations through the standard
  `PluginDispatcher`.
- Emitting human-readable output for terminal users.
- Emitting machine-readable JSON when `--json` is passed.

### Out of scope

- Package synchronization — owned by `vector-database`.
- LanceDB store creation or schema rules — owned by `runtime-rag`.
- MCP protocol handling — owned by `mcp-vector`.
- Retrieval ranking, embedding, or indexing logic — owned by `runtime-rag`.

### Dependencies

| Dependency        | Role                                                                   |
|-------------------|------------------------------------------------------------------------|
| `runtime-rag`     | `IndexWorkspaceOp`, `HybridSearchOp`, `AssembleRetrievalContextOp`, defaults |
| `runtime-channel` | `PluginDispatcher` execution bridge between commands and operations    |
| `runtime-core`    | Core plugin and channel contracts                                      |
| `serde`           | JSON serialization for `--json` output contracts                       |
| `serde_json`      | JSON encoding for final machine-readable payloads                      |
| `tokio`           | Async runtime execution                                                |

## 3. Commands

### `rag init`

Creates or validates the local Phase 6 LanceDB store under
`.vector-database/rag/lancedb/`.

**Exit behavior:**

| Condition                                           | Exit code |
|-----------------------------------------------------|-----------|
| Store created, updated, or validated successfully   | `0`       |
| Store contract is incompatible or init fails        | `1`       |

### `rag search`

Executes hybrid retrieval against the local RAG store and returns governed
document context. Delegates to `HybridSearchOp` and `AssembleRetrievalContextOp`
directly in the same process.

**Arguments:**

- `<query>`: Required free-text query string.
- `--limit <n>`: Optional final result count override.
- `--package <name>`: Optional package filter applied before ranking.
- `--document <stem>`: Optional governed document stem filter applied before ranking.
- `--json`: Emit the canonical `RetrievalContext` as machine-readable JSON.

**Exit behavior:**

| Condition                                                        | Exit code |
|------------------------------------------------------------------|-----------|
| Retrieval succeeds, including empty result sets                  | `0`       |
| Store is missing, incompatible, or query execution fails         | `1`       |
| Argument parsing fails                                           | `1`       |

### `rag update-database`

Runs the Phase 7 incremental indexing pipeline. Initializes the LanceDB store
if not already present, then indexes all governed Markdown documents under the
workspace `doc/` corpus and every synchronized package corpus under
`.vector-database/packages/{package}/doc/`. Files whose content hash is
unchanged are skipped without re-embedding.

Progress events are emitted while the operation runs using stable labels
(`initializing-store`, `indexed`, `unchanged`, `failed`). A final summary with
re-indexed, skipped, and deleted document counts is emitted after all documents
are processed.

**Arguments:**

- `--json`: Emit the final machine-readable contract instead of streaming human
  text. The JSON payload includes a `progress` array of all captured
  `IndexWorkspaceProgress` events and a `summary` object with the final
  `IndexResult` counts.

**Exit behavior:**

| Condition                                              | Exit code |
|--------------------------------------------------------|-----------|
| All documents indexed or skipped successfully          | `0`       |
| One or more documents failed during indexing           | `1`       |
| Dispatcher or operation error                          | `1`       |

## 4. Usage

```sh
# Create or validate the local RAG store
vector-rag rag init

# Search the local RAG store with hybrid retrieval
vector-rag rag search "retrieval context contract"

# Filter retrieval to one package
vector-rag rag search "retrieval context contract" --package shared-docs

# Filter retrieval to one document and cap results
vector-rag rag search "retrieval context contract" --document rfc-00041-phase-9 --limit 3

# Emit machine-readable JSON for retrieval
vector-rag rag search "retrieval context contract" --json

# Run the incremental indexing pipeline
vector-rag rag update-database

# Capture a final machine-readable indexing result
vector-rag rag update-database --json
```

## 5. Workspace Root Discovery

`vector-rag` resolves the workspace root by walking up from the current
working directory until it finds a directory containing a `.vector/` folder.
If no `.vector/` directory is found before reaching the filesystem root, the
command exits with an actionable error.

Callers that invoke `vector-rag` as a subprocess (such as `vector-database`)
must run it with the workspace root as the working directory so that discovery
resolves immediately without an upward walk.

## 6. JSON Output Contracts

### `rag search --json`

Returns one JSON object matching the canonical `RetrievalContext` shape defined
by RFC 00041. The `status` field is `"found"` when evidence chunks were
returned and `"empty"` when the query succeeded but no chunks matched.

### `rag update-database --json`

Returns one JSON object with two top-level fields:

- `progress`: array of `IndexWorkspaceProgress` objects, each with `label`,
  optional `package`, optional `document_stem`, and optional `message`.
- `summary`: `IndexResult` object with `skipped_count`, `reindexed_count`,
  `deleted_count`, and `failures`.

The project intentionally does not use NDJSON for indexing. The final JSON
payload keeps MCP and CLI automation consumption stable without requiring
consumers to parse streaming human text.
