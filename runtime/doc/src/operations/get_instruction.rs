//! Plugin operation for reading a UUID-scoped instruction from the controlled directory.
//!
//! `GetInstructionOp` is the reusable implementation layer consumed by the MCP adapter
//! in `mcp-vector`. All filesystem access, configuration loading, UUID validation,
//! containment enforcement, size enforcement, encoding checks, and platform-specific
//! link protection live here; `mcp-vector` must not duplicate them.
//!
//! # Platform guarantees
//!
//! ## Windows
//! Opens the target with `FILE_FLAG_OPEN_REPARSE_POINT` via `OpenOptionsExt::custom_flags`
//! so the kernel does not follow reparse-point redirection at the open call. After opening,
//! inspects the handle attributes with `GetFileInformationByHandle` to reject any reparse
//! target whose attribute appeared between the open and the attribute check. Then calls
//! `GetFinalPathNameByHandleW` to validate the opened handle's canonical final location
//! against the configured instructions directory.
//!
//! Residual limitation: hard links on Windows do not carry the reparse attribute and are
//! not detectable with these APIs alone. Creating a hard link to a file outside the
//! instructions directory and placing it inside requires elevated privileges under default
//! Windows system policies; this risk is documented but not fully eliminated without a
//! separate privilege check.
//!
//! ## Unix
//! Uses `fs::symlink_metadata` to inspect the file type without following symbolic links
//! before opening. A TOCTOU (time-of-check-to-time-of-use) window exists between the
//! metadata call and the subsequent `open`: a symlink could be swapped in during that
//! interval. Platforms that support `O_NOFOLLOW` would eliminate this window; this
//! implementation uses the strongest reliably portable check available in `std`.

use crate::agents::config::{InstructionsDirError, validate_instructions_dir_from_yaml};
use runtime_core::{RuntimeResult, declare_plugin_operations, plugin::PluginSender};
use runtime_io::path::IoPath;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Maximum instruction file size accepted by `GetInstructionOp`.
///
/// Fixed at 1 MiB; not configurable.
const MAX_INSTRUCTION_SIZE: u64 = 1_048_576;

/// The only recognized variable expression in `instructions-dir`.
const SYSTEM_TEMP_VAR: &str = "${system-temp}";

/// Fixed filename prefix for instruction files.
const INSTRUCTION_FILE_PREFIX: &str = "vector-instruction-";

/// Fixed filename suffix for instruction files.
const INSTRUCTION_FILE_SUFFIX: &str = ".txt";

/// Input for the `get_instruction` operation.
///
/// # DTO(Plugin operation input contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct GetInstructionInput {
    /// The root directory of the project (used to locate `.vector/agents.yaml`).
    pub root_dir: IoPath,
    /// The canonical lowercase hyphenated UUID identifying the instruction to read.
    pub id: String,
}

/// Output for the `get_instruction` operation.
///
/// # DTO(Plugin operation output contracts use public fields for ergonomic data transfer)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct GetInstructionOutput {
    /// The exact UTF-8 content of the instruction file.
    ///
    /// Returned verbatim without modification. The file is not deleted; repeated
    /// reads remain valid for the terminal lifetime.
    pub content: String,
}

async fn get_instruction(
    input: GetInstructionInput,
    output: &mut impl PluginSender<GetInstructionOutput>,
) -> RuntimeResult<()> {
    // Validate UUID before any filesystem access.
    validate_uuid(&input.id)?;

    // Resolve the configured instructions directory from `.vector/agents.yaml`.
    let instructions_dir = load_instructions_dir(&input.root_dir)?;

    // Construct the fixed filename internally; the caller never provides a path.
    let filename = format!("{}{}{}", INSTRUCTION_FILE_PREFIX, input.id, INSTRUCTION_FILE_SUFFIX);
    let target = instructions_dir.join(&filename);

    // Lexical containment: target must be a direct child of the configured directory.
    verify_containment(&instructions_dir, &target)?;

    // Read with platform-appropriate link and reparse-point protection.
    let content = read_instruction(&target)?;

    output.send(GetInstructionOutput { content }).await?;
    Ok(())
}

// ── UUID validation ───────────────────────────────────────────────────────────

/// Hyphen positions in the canonical UUID form: 8-4-4-4-12.
const HYPHEN_POSITIONS: [usize; 4] = [8, 13, 18, 23];

/// Validate that `id` is a canonical lowercase hyphenated UUID.
///
/// Accepts only the form `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` where every
/// non-hyphen character is a lowercase hexadecimal digit (`0`–`9`, `a`–`f`).
/// Rejects traversal syntax, separators, extra characters, alternate extensions,
/// uppercase, and otherwise noncanonical encodings.
pub(crate) fn validate_uuid(id: &str) -> RuntimeResult<()> {
    // Canonical form: 32 lowercase hex digits + 4 hyphens = 36 characters total.
    if id.len() != 36 {
        return Err(runtime_core::RuntimeError::operation(
            "invalid instruction id: must be a canonical lowercase hyphenated UUID (36 characters)",
        ));
    }

    let bytes = id.as_bytes();

    for (i, &b) in bytes.iter().enumerate() {
        if HYPHEN_POSITIONS.contains(&i) {
            if b != b'-' {
                return Err(runtime_core::RuntimeError::operation(
                    "invalid instruction id: hyphens must appear at positions 8, 13, 18, and 23",
                ));
            }
        } else if !matches!(b, b'0'..=b'9' | b'a'..=b'f') {
            return Err(runtime_core::RuntimeError::operation(
                "invalid instruction id: UUID must use only lowercase hexadecimal digits (0-9, a-f)",
            ));
        }
    }

    Ok(())
}

// ── Configuration loading ─────────────────────────────────────────────────────

/// Load and validate `instructions-dir` from `.vector/agents.yaml`.
///
/// Resolves `${system-temp}` to the platform system temporary directory and
/// returns the resulting absolute path.
///
/// # Errors
///
/// Returns an operation error for a missing file, a malformed YAML document,
/// a missing or invalid `instructions-dir` field, or an unusable path value.
fn load_instructions_dir(root_dir: &IoPath) -> RuntimeResult<PathBuf> {
    let agents_yaml_path = root_dir.as_path().join(".vector").join("agents.yaml");

    let content = fs::read_to_string(&agents_yaml_path).map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "invalid project configuration: failed to read .vector/agents.yaml: {e}"
        ))
    })?;

    let doc: noyalib::compat::serde_yaml::Value = noyalib::compat::serde_yaml::from_str(&content)
        .map_err(|e| {
        runtime_core::RuntimeError::operation(format!(
            "invalid project configuration: failed to parse .vector/agents.yaml: {e}"
        ))
    })?;

    let raw_value = doc.get("instructions-dir");

    validate_instructions_dir_from_yaml(raw_value)
        .map_err(|e: InstructionsDirError| runtime_core::RuntimeError::operation(e.message()))?;

    // validate_instructions_dir_from_yaml passed → the value is a non-empty string that
    // starts with ${system-temp} and has no traversal or escape violations.
    let value =
        raw_value.and_then(noyalib::compat::serde_yaml::Value::as_str).ok_or_else(|| {
            runtime_core::RuntimeError::operation(
                "invalid project configuration: instructions-dir is not a valid string",
            )
        })?;

    Ok(resolve_instructions_dir_path(value))
}

/// Resolve an already-validated `instructions-dir` string into an absolute `PathBuf`.
///
/// Substitutes `${system-temp}` with the platform system temporary directory
/// and appends any remaining relative path segments.
pub(crate) fn resolve_instructions_dir_path(value: &str) -> PathBuf {
    let suffix = &value[SYSTEM_TEMP_VAR.len()..];
    let tmp_dir = env::temp_dir();
    let suffix_rel = suffix.trim_start_matches(['/', '\\']);
    if suffix_rel.is_empty() { tmp_dir } else { tmp_dir.join(suffix_rel) }
}

// ── Containment enforcement ───────────────────────────────────────────────────

/// Verify that `target` is lexically a direct child of `dir`.
///
/// Checks that `target.parent()` equals `dir` using standard path comparison.
/// This is a lexical guard; platform-specific handle checks provide the
/// runtime guarantee against TOCTOU races.
pub(crate) fn verify_containment(dir: &Path, target: &Path) -> RuntimeResult<()> {
    let parent = target.parent().ok_or_else(|| {
        runtime_core::RuntimeError::operation(
            "instruction path has no parent directory; containment cannot be verified",
        )
    })?;

    if parent != dir {
        return Err(runtime_core::RuntimeError::operation(
            "instruction target is not a direct child of the configured instructions directory",
        ));
    }

    Ok(())
}

// ── Platform-dispatched file read ─────────────────────────────────────────────

/// Read the instruction file with platform-appropriate link and reparse-point protection.
///
/// Enforces the 1 MiB size limit before and during reading. Requires valid UTF-8.
fn read_instruction(target: &Path) -> RuntimeResult<String> {
    #[cfg(windows)]
    {
        read_instruction_windows(target)
    }
    #[cfg(not(windows))]
    {
        read_instruction_unix(target)
    }
}

// ── Unix implementation ───────────────────────────────────────────────────────

/// Read the instruction file on Unix using `symlink_metadata` to detect and reject
/// symbolic links before opening.
///
/// # Platform limitation
/// A symlink could be atomically swapped in between the `symlink_metadata` call and
/// the `File::open` call (TOCTOU window). The strongest portable check available in
/// `std` is used here. `O_NOFOLLOW`-based protection would require `libc` or `nix`.
#[cfg(not(windows))]
fn read_instruction_unix(target: &Path) -> RuntimeResult<String> {
    use std::io::Read;

    // Inspect without following to detect symbolic links before opening.
    let meta = fs::symlink_metadata(target).map_err(|_| {
        runtime_core::RuntimeError::operation(
            "instruction not found or expired: the requested instruction file does not exist",
        )
    })?;

    if meta.file_type().is_symlink() {
        return Err(runtime_core::RuntimeError::operation(
            "instruction target is a symbolic link; only regular files are accepted",
        ));
    }

    if !meta.file_type().is_file() {
        return Err(runtime_core::RuntimeError::operation(
            "instruction target is not a regular file; only regular files are accepted",
        ));
    }

    if meta.len() > MAX_INSTRUCTION_SIZE {
        return Err(runtime_core::RuntimeError::operation(format!(
            "instruction exceeds the maximum allowed size of {MAX_INSTRUCTION_SIZE} bytes"
        )));
    }

    let file = fs::File::open(target).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to read instruction: {e}"))
    })?;

    // Limit the read to MAX_INSTRUCTION_SIZE + 1 to detect growth during read.
    let mut reader = file.take(MAX_INSTRUCTION_SIZE + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to read instruction: {e}"))
    })?;

    if bytes.len() as u64 > MAX_INSTRUCTION_SIZE {
        return Err(runtime_core::RuntimeError::operation(format!(
            "instruction exceeds the maximum allowed size of {MAX_INSTRUCTION_SIZE} bytes"
        )));
    }

    String::from_utf8(bytes).map_err(|_| {
        runtime_core::RuntimeError::operation("instruction content is not valid UTF-8")
    })
}

// ── Windows implementation ────────────────────────────────────────────────────

/// `FILE_FLAG_OPEN_REPARSE_POINT` — open the file itself rather than following
/// reparse-point redirection.
#[cfg(windows)]
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

/// `FILE_ATTRIBUTE_REPARSE_POINT` — file or directory is a reparse point.
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// `FILE_ATTRIBUTE_DIRECTORY` — target is a directory.
#[cfg(windows)]
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

/// Read the instruction file on Windows using target-specific `windows-sys` handle APIs.
///
/// Opens the target with `FILE_FLAG_OPEN_REPARSE_POINT` via `OpenOptionsExt::custom_flags`
/// to avoid following reparse-point redirection at the `open` call, then:
/// 1. Inspects the handle's file attributes with `GetFileInformationByHandle` and rejects
///    any reparse point or directory.
/// 2. Validates the handle's canonical final path with `GetFinalPathNameByHandleW` to
///    confirm the opened object resides inside the configured instructions directory.
/// 3. Enforces the 1 MiB size limit before and during reading.
/// 4. Requires valid UTF-8.
///
/// # Residual limitation
/// Hard links do not carry the reparse attribute and cannot be detected with these APIs
/// alone. Creating a hard link to a file outside the instructions directory and placing
/// it inside requires elevated privileges under default Windows system policies.
#[cfg(windows)]
fn read_instruction_windows(target: &Path) -> RuntimeResult<String> {
    use std::io::Read;
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    // Open with FILE_FLAG_OPEN_REPARSE_POINT so the handle refers to the reparse
    // point object itself rather than following the redirect.  Regular files are
    // unaffected by this flag.
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(target)
        .map_err(|_| {
            runtime_core::RuntimeError::operation(
                "instruction not found or expired: the requested instruction file does not exist",
            )
        })?;

    // Obtain the raw HANDLE from the File so it can be passed to Windows APIs.
    // SAFETY: `file` owns the handle for the full scope of this function; no
    // other thread or call site closes it during this period.
    let raw_handle = file.as_raw_handle();
    let handle = raw_handle as windows_sys::Win32::Foundation::HANDLE;

    // Inspect handle attributes to detect reparse points and directories.
    //
    // SAFETY: `handle` is valid and open; `info` is zero-initialized to the correct size.
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `handle` is valid and open; `&raw mut info` is a valid writable pointer to
    // a properly initialized `BY_HANDLE_FILE_INFORMATION` struct.
    let ok = unsafe { GetFileInformationByHandle(handle, &raw mut info) };
    if ok == 0 {
        return Err(runtime_core::RuntimeError::operation(
            "failed to inspect instruction file attributes",
        ));
    }

    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(runtime_core::RuntimeError::operation(
            "instruction target is a reparse point; only regular files are accepted",
        ));
    }

    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Err(runtime_core::RuntimeError::operation(
            "instruction target is a directory; only regular files are accepted",
        ));
    }

    // Validate the opened handle's final canonical path remains inside the configured
    // instructions directory.
    let instructions_dir = target
        .parent()
        .ok_or_else(|| runtime_core::RuntimeError::operation("instruction path has no parent"))?;
    verify_final_path_containment(handle, instructions_dir)?;

    // Check the pre-read size using the handle attribute data.
    let size = u64::from(info.nFileSizeHigh) << 32 | u64::from(info.nFileSizeLow);
    if size > MAX_INSTRUCTION_SIZE {
        return Err(runtime_core::RuntimeError::operation(format!(
            "instruction exceeds the maximum allowed size of {MAX_INSTRUCTION_SIZE} bytes"
        )));
    }

    // Read through the handle-backed File with a bounded reader.
    let mut reader = (&file).take(MAX_INSTRUCTION_SIZE + 1);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).map_err(|e| {
        runtime_core::RuntimeError::operation(format!("failed to read instruction: {e}"))
    })?;

    if bytes.len() as u64 > MAX_INSTRUCTION_SIZE {
        return Err(runtime_core::RuntimeError::operation(format!(
            "instruction exceeds the maximum allowed size of {MAX_INSTRUCTION_SIZE} bytes"
        )));
    }

    String::from_utf8(bytes).map_err(|_| {
        runtime_core::RuntimeError::operation("instruction content is not valid UTF-8")
    })
}

/// Validate that the canonical final path of `handle` has `expected_dir` as its parent.
///
/// Uses `GetFinalPathNameByHandleW` with `VOLUME_NAME_DOS | FILE_NAME_NORMALIZED` to
/// resolve the handle's final location, then compares the normalized parent path against
/// the configured instructions directory.
///
/// `dunce::canonicalize` is used to normalize both sides, removing the `\\?\` extended
/// path prefix that `GetFinalPathNameByHandleW` may produce.
#[cfg(windows)]
fn verify_final_path_containment(
    handle: windows_sys::Win32::Foundation::HANDLE,
    expected_dir: &Path,
) -> RuntimeResult<()> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_NAME_NORMALIZED, GetFinalPathNameByHandleW, VOLUME_NAME_DOS,
    };

    // Initial buffer sized for MAX_PATH + null terminator.
    let mut buf: Vec<u16> = vec![0u16; 512];

    // SAFETY: `handle` is valid and open; `buf` is writable with length matching `cchFilePath`.
    let len = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            buf.as_mut_ptr(),
            u32::try_from(buf.len()).unwrap_or(u32::MAX),
            VOLUME_NAME_DOS | FILE_NAME_NORMALIZED,
        )
    };

    if len == 0 {
        return Err(runtime_core::RuntimeError::operation(
            "failed to resolve the final path of the instruction handle",
        ));
    }

    // If the buffer was too small, the return value is the required buffer size
    // (including the null terminator); resize and retry.
    let char_count = if len as usize >= buf.len() {
        buf.resize(len as usize + 1, 0);
        // SAFETY: `handle` is valid; `buf` is now large enough.
        let len2 = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buf.as_mut_ptr(),
                u32::try_from(buf.len()).unwrap_or(u32::MAX),
                VOLUME_NAME_DOS | FILE_NAME_NORMALIZED,
            )
        };
        if len2 == 0 || len2 as usize >= buf.len() {
            return Err(runtime_core::RuntimeError::operation(
                "failed to resolve the final path of the instruction handle",
            ));
        }
        len2 as usize
    } else {
        len as usize
    };

    let final_path = PathBuf::from(std::ffi::OsString::from_wide(&buf[..char_count]));

    let final_parent = final_path.parent().ok_or_else(|| {
        runtime_core::RuntimeError::operation(
            "resolved instruction path has no parent; containment cannot be verified",
        )
    })?;

    // Normalize both sides with dunce to strip \\?\ and resolve casing on Windows.
    let expected_norm =
        dunce::canonicalize(expected_dir).unwrap_or_else(|_| expected_dir.to_path_buf());
    let actual_norm =
        dunce::canonicalize(final_parent).unwrap_or_else(|_| final_parent.to_path_buf());

    if actual_norm != expected_norm {
        return Err(runtime_core::RuntimeError::operation(
            "resolved instruction path escapes the configured instructions directory",
        ));
    }

    Ok(())
}

// ── Operation declaration ─────────────────────────────────────────────────────

declare_plugin_operations! {
    GetInstructionOp => get_instruction(GetInstructionInput, GetInstructionOutput)
}

impl GetInstructionInput {
    /// Construct a `GetInstructionInput` with explicit fields.
    #[must_use]
    pub const fn new(root_dir: IoPath, id: String) -> Self {
        Self { root_dir, id }
    }
}

impl GetInstructionOp {
    /// Construct a new `GetInstructionOp`.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for GetInstructionOp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "get_instruction_test.rs"]
mod tests;
