# `vector-database`

## 1. Objective

`vector-database` is the command-line interface (CLI) for executing package synchronization and managing repository package manifest mutations in the Vector workspace. It acts as the execution surface for the planning operations defined in `runtime-packages`.

## 2. Boundaries

### In scope

- Running package synchronization (`sync` command) which executes `git clone`, `git fetch`, and file copy operations.
- Interfacing with `runtime-packages` to add new dependencies into `.vector/packages.yaml` via CLI arguments.
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
| `runtime-io` | Terminal commands execution, path helpers, and IO |
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
```
