# runtime-packages

Package governance, manifest loading, validation, sync planning, and manifest mutation operations for the Vector runtime.

This crate manages the package definitions located in `.vector/packages.yaml` and plans package synchronization into `.vector-database/packages/`.

## Manifest Contract

The manifest is written in YAML at `.vector/packages.yaml` and contains package entries with their type, source URL, and tag specifications:

```yaml
# .vector/packages.yaml
my_git_package:
  type: git
  url: https://github.com/example/repo.git
  tag: branch:main

my_file_package:
  type: file
  url: /path/to/local/dir
```

### Git Packages
- `type` must be `"git"`.
- `url` must specify the Git repository URL.
- `tag` is **required** and can be either a Git tag name or `branch:<name>` (e.g. `branch:main`) to track a branch HEAD.

### File Packages
- `type` must be `"file"`.
- `url` must specify the local file path.
- `tag` is **optional**.

## Public Interface

### Types

- `PackageEntry`: A single package configuration containing `type`, `url`, and `tag`.
- `PackageManifest`: Represents the full `.vector/packages.yaml` structure. Supports parsing and serialization to YAML.
- `ManifestError`: Validation errors (e.g. missing required fields, unsupported source type, missing Git tag).

### Operations

This crate exposes two governed operations:

#### 1. `SyncPackagesOp` (`sync-packages`)
Evaluates the package manifest and local cache directories, producing a list of deterministic sync actions to execute.

- **Input**: `SyncPackagesInput { root_dir }`
- **Output**: `SyncPackagesOutput { actions }` where each `SyncAction` contains:
  - `name`: Package name.
  - `command_type`: `Clone`, `Fetch`, or `Copy`.
  - `description`: Agent-facing description of the execution step.

#### 2. `AddPackageOp` (`add-package`)
Adds a new package entry to `.vector/packages.yaml` after validating it against the manifest contract and checking for duplicate names.

- **Input**: `AddPackageInput { root_dir, name, type, url, tag }`
- **Output**: `AddPackageOutput`

## Scope

This crate owns:
- Manifest parsing, validation, loading, and serialization.
- Sync planning logic and execution instructions.
- Verification and mutation of the package configurations.

This crate does **not** invoke external shell commands (like `git clone` or `git fetch`) directly; execution is the responsibility of the CLI surface (`vector-database`).

## Dependencies

- `runtime-core` — core operation traits and error bounds
- `runtime-io` — path and file reading/writing helpers
- `serde` / `serde_yaml` — YAML serialization/deserialization
- `thiserror` — error implementation
