//! Focused tests for `GetInstructionOp`.
//!
//! All tests use isolated temporary directories and never modify the developer's
//! real instruction files.
//!
//! Test coverage:
//! - UUID validation: canonical, non-canonical, traversal-shaped, separator-containing,
//!   uppercase, extended, wrong-length inputs.
//! - Configuration loading: missing agents.yaml, invalid YAML, missing `instructions-dir`,
//!   invalid `instructions-dir` value.
//! - Containment: direct-child target, path that escapes the directory.
//! - Successful read: returns exact content.
//! - Repeated reads: file is not deleted; second read succeeds.
//! - Error cases: missing instruction, oversized content, invalid UTF-8.
//! - Platform-appropriate link tests: symbolic link rejected (Unix); reparse point
//!   rejected (Windows, when creation is supported by the test environment).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{
    GetInstructionInput, MAX_INSTRUCTION_SIZE, SYSTEM_TEMP_VAR, resolve_instructions_dir_path,
    validate_uuid, verify_containment,
};
use runtime_io::path::IoPath;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A minimal mock sender that collects `GetInstructionOutput` values.
struct MockSender {
    outputs: Vec<super::GetInstructionOutput>,
}

impl MockSender {
    fn new() -> Self {
        Self { outputs: Vec::new() }
    }
}

impl runtime_core::channel::Sender<super::GetInstructionOutput> for MockSender {
    async fn send(
        &mut self,
        value: super::GetInstructionOutput,
    ) -> runtime_core::RuntimeResult<()> {
        self.outputs.push(value);
        Ok(())
    }
}

impl runtime_core::cancel::CancelableSender<super::GetInstructionOutput> for MockSender {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// A canonical UUID for use across tests.
const VALID_UUID: &str = "93b0a984-5422-4491-9b0c-db44fdb69ea8";

/// Write a minimal `.vector/agents.yaml` that points to `instructions_dir_suffix`.
///
/// The suffix must be a relative path component under `${system-temp}`.
fn write_agents_yaml(project_dir: &TempDir, instructions_dir_suffix: &str) {
    let vector_dir = project_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(
        vector_dir.join("agents.yaml"),
        format!("instructions-dir: \"${{system-temp}}/{instructions_dir_suffix}\"\nagents: {{}}\n"),
    )
    .unwrap();
}

/// Create an isolated project root and a matching instructions directory.
///
/// Returns the project `TempDir`, the resolved instructions directory path, and the
/// `instructions_dir_suffix` used in `agents.yaml`.
///
/// The instructions directory is created inside the system temporary directory so that
/// `validate_instructions_dir` accepts the configured value. Each call uses a unique
/// suffix derived from the instructions `TempDir` base name to avoid cross-test pollution.
fn create_isolated_test_env() -> (TempDir, PathBuf, String) {
    let project_dir = tempfile::tempdir().unwrap();
    let system_temp = env::temp_dir();

    // Create a named subdirectory inside system temp for instruction files.
    // tempfile::TempDir created inside `system_temp` gives a unique name.
    let instr_temp = tempfile::TempDir::new_in(&system_temp).unwrap();
    let dir_name = instr_temp.path().file_name().unwrap().to_str().unwrap().to_string();

    // Persist the instructions directory for the test duration.
    // `TempDir::keep` converts the temp dir to a plain PathBuf, preventing deletion.
    let instructions_path = instr_temp.keep();

    write_agents_yaml(&project_dir, &dir_name);

    (project_dir, instructions_path, dir_name)
}

/// Write a valid instruction file to the instructions directory.
fn write_instruction_file(instructions_dir: &Path, uuid: &str, content: &str) {
    let filename = format!("vector-instruction-{uuid}.txt");
    fs::write(instructions_dir.join(&filename), content).unwrap();
}

// ── UUID validation ───────────────────────────────────────────────────────────

#[test]
fn uuid_validation_accepts_canonical_lowercase_uuid() {
    assert!(validate_uuid(VALID_UUID).is_ok(), "canonical UUID must be accepted");
}

#[test]
fn uuid_validation_accepts_all_zeros_uuid() {
    assert!(
        validate_uuid("00000000-0000-0000-0000-000000000000").is_ok(),
        "all-zeros UUID must be accepted"
    );
}

#[test]
fn uuid_validation_rejects_uppercase_uuid() {
    let upper = VALID_UUID.to_uppercase();
    let result = validate_uuid(&upper);
    assert!(result.is_err(), "uppercase UUID must be rejected; got: {result:?}");
}

#[test]
fn uuid_validation_rejects_mixed_case_uuid() {
    // Replace one lowercase letter with uppercase.
    let mixed = VALID_UUID.replace('a', "A");
    let result = validate_uuid(&mixed);
    assert!(result.is_err(), "mixed-case UUID must be rejected; got: {result:?}");
}

#[test]
fn uuid_validation_rejects_wrong_length_short() {
    let result = validate_uuid("93b0a984-5422-4491-9b0c-db44fdb69ea");
    assert!(result.is_err(), "short UUID (35 chars) must be rejected");
}

#[test]
fn uuid_validation_rejects_wrong_length_long() {
    let result = validate_uuid("93b0a984-5422-4491-9b0c-db44fdb69ea8x");
    assert!(result.is_err(), "long UUID (37 chars) must be rejected");
}

#[test]
fn uuid_validation_rejects_missing_hyphens() {
    // 36 chars where hyphens are replaced by hex digits — fails at first non-hyphen position.
    let result = validate_uuid("93b0a98454224491-9b0c-db44fdb69ea8x");
    assert!(result.is_err(), "UUID without hyphens at required positions must be rejected");
}

#[test]
fn uuid_validation_rejects_traversal_in_id() {
    // Traversal syntax at the start — length is wrong, but also contains non-hex chars.
    let result = validate_uuid("../b0a984-5422-4491-9b0c-db44fdb69ea8");
    assert!(result.is_err(), "traversal syntax in UUID must be rejected");
}

#[test]
fn uuid_validation_rejects_separator_in_id() {
    // Path separator inside the UUID.
    let result = validate_uuid("93b0a984/5422-4491-9b0c-db44fdb69ea8");
    assert!(result.is_err(), "path separator in UUID must be rejected");
}

#[test]
fn uuid_validation_rejects_extra_extension_chars() {
    // Last two chars are `.t` — not valid lowercase hex.
    let result = validate_uuid("93b0a984-5422-4491-9b0c-db44fdb69e.t");
    assert!(result.is_err(), "UUID with non-hex chars at the end must be rejected");
}

#[test]
fn uuid_validation_rejects_g_and_above_hex_digits() {
    // 'g' is not a valid hex digit.
    let result = validate_uuid("93b0a984-5422-4491-9b0c-db44fdb69gg8");
    assert!(result.is_err(), "non-hex character 'g' in UUID must be rejected");
}

// ── Containment ───────────────────────────────────────────────────────────────

#[test]
fn containment_accepts_direct_child() {
    let dir = PathBuf::from("/tmp/vector/instructions");
    let target = dir.join("vector-instruction-abc.txt");
    assert!(verify_containment(&dir, &target).is_ok(), "direct child must pass containment check");
}

#[test]
fn containment_rejects_grandchild() {
    let dir = PathBuf::from("/tmp/vector/instructions");
    let target = dir.join("sub").join("vector-instruction-abc.txt");
    let result = verify_containment(&dir, &target);
    assert!(result.is_err(), "grandchild path must fail containment check");
}

#[test]
fn containment_rejects_sibling_directory() {
    let dir = PathBuf::from("/tmp/vector/instructions");
    let target = PathBuf::from("/tmp/vector/other/vector-instruction-abc.txt");
    let result = verify_containment(&dir, &target);
    assert!(result.is_err(), "target in sibling directory must fail containment check");
}

// ── resolve_instructions_dir_path ────────────────────────────────────────────

#[test]
fn resolve_instructions_dir_path_appends_suffix() {
    let result = resolve_instructions_dir_path(&format!("{SYSTEM_TEMP_VAR}/vector/instructions"));
    let expected = env::temp_dir().join("vector").join("instructions");
    assert_eq!(
        result, expected,
        "resolved path must equal system temp joined with the relative suffix"
    );
}

#[test]
fn resolve_instructions_dir_path_returns_system_temp_when_no_suffix() {
    let result = resolve_instructions_dir_path(SYSTEM_TEMP_VAR);
    assert_eq!(result, env::temp_dir(), "value with no suffix must resolve to system temp");
}

// ── Successful read ───────────────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_returns_exact_content_for_valid_uuid() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();
    let content = "Execute the following task:\n\n1. Do something useful.\n";
    write_instruction_file(&instructions_dir, VALID_UUID, content);

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_ok(), "operation must succeed for a valid instruction; err: {result:?}");
    assert_eq!(sender.outputs.len(), 1, "operation must emit exactly one output");
    assert_eq!(sender.outputs[0].content, content, "content must be returned verbatim");
}

#[tokio::test]
async fn get_instruction_does_not_delete_file_after_read() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();
    let content = "Instruction content for repeated-read test.";
    write_instruction_file(&instructions_dir, VALID_UUID, content);

    let filename = format!("vector-instruction-{VALID_UUID}.txt");
    let file_path = instructions_dir.join(&filename);

    // First read.
    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    super::get_instruction(input, &mut sender).await.unwrap();

    assert!(file_path.exists(), "instruction file must not be deleted after a successful read");

    // Second read — must succeed with identical content.
    let input2 = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender2 = MockSender::new();
    let result2 = super::get_instruction(input2, &mut sender2).await;
    assert!(result2.is_ok(), "second read must succeed without deletion; err: {result2:?}");
    assert_eq!(sender2.outputs[0].content, content, "second read must return the same content");
}

// ── Configuration error cases ─────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_fails_when_agents_yaml_is_missing() {
    let project_dir = tempfile::tempdir().unwrap();
    // Do NOT write agents.yaml.
    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail when agents.yaml is absent");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("agents.yaml"), "error must reference agents.yaml; got: {msg}");
}

#[tokio::test]
async fn get_instruction_fails_when_agents_yaml_is_malformed() {
    let project_dir = tempfile::tempdir().unwrap();
    let vector_dir = project_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(vector_dir.join("agents.yaml"), "not: valid: yaml: :::").unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail when agents.yaml is malformed YAML");
}

#[tokio::test]
async fn get_instruction_fails_when_instructions_dir_is_missing_from_yaml() {
    let project_dir = tempfile::tempdir().unwrap();
    let vector_dir = project_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(vector_dir.join("agents.yaml"), "agents: {}\n").unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(
        result.is_err(),
        "operation must fail when instructions-dir is absent from agents.yaml"
    );
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("instructions-dir"), "error must reference instructions-dir; got: {msg}");
}

#[tokio::test]
async fn get_instruction_fails_when_instructions_dir_has_no_system_temp_var() {
    let project_dir = tempfile::tempdir().unwrap();
    let vector_dir = project_dir.path().join(".vector");
    fs::create_dir_all(&vector_dir).unwrap();
    fs::write(vector_dir.join("agents.yaml"), "instructions-dir: \"/absolute/path\"\nagents: {}\n")
        .unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(
        result.is_err(),
        "operation must fail when instructions-dir does not start with ${{system-temp}}"
    );
}

// ── UUID error cases ──────────────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_fails_for_uppercase_uuid() {
    let (project_dir, _, _) = create_isolated_test_env();
    let upper_uuid = VALID_UUID.to_uppercase();
    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), upper_uuid);
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail before file access for an uppercase UUID");
}

#[tokio::test]
async fn get_instruction_fails_for_invalid_uuid_format() {
    let (project_dir, _, _) = create_isolated_test_env();
    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), "not-a-uuid".to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail for a non-UUID id value");
}

// ── Missing instruction ───────────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_fails_when_instruction_file_is_absent() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();
    // Directory exists but the instruction file does not.
    fs::create_dir_all(&instructions_dir).unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail when the instruction file does not exist");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not found") || msg.contains("expired") || msg.contains("does not exist"),
        "error must describe a missing or expired instruction; got: {msg}"
    );
}

// ── Oversized content ─────────────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_fails_for_oversized_content() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();

    // Write a file that is exactly MAX_INSTRUCTION_SIZE + 1 bytes.
    let oversized: Vec<u8> =
        vec![b'x'; usize::try_from(MAX_INSTRUCTION_SIZE + 1).unwrap_or(usize::MAX)];
    let filename = format!("vector-instruction-{VALID_UUID}.txt");
    fs::write(instructions_dir.join(&filename), &oversized).unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail for content exceeding MAX_INSTRUCTION_SIZE");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("maximum") || msg.contains("size"),
        "error must describe the size limit violation; got: {msg}"
    );
}

// ── Invalid UTF-8 ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_instruction_fails_for_invalid_utf8_content() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();

    // Write raw bytes that are not valid UTF-8.
    let invalid_utf8: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01];
    let filename = format!("vector-instruction-{VALID_UUID}.txt");
    fs::write(instructions_dir.join(&filename), &invalid_utf8).unwrap();

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must fail for non-UTF-8 instruction content");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("UTF-8"), "error must describe the encoding failure; got: {msg}");
}

// ── Platform link tests ───────────────────────────────────────────────────────

/// On Unix, verify that a symbolic link pointing to a valid instruction file is rejected.
///
/// The test skips gracefully when symlink creation is not available in the test environment.
#[cfg(unix)]
#[tokio::test]
async fn get_instruction_rejects_symlink_on_unix() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();

    // Create the real instruction file in another temp directory.
    let real_dir = tempfile::tempdir().unwrap();
    let real_file = real_dir.path().join("real-instruction.txt");
    fs::write(&real_file, "real content").unwrap();

    // Create a symlink inside the instructions directory pointing to the real file.
    let filename = format!("vector-instruction-{VALID_UUID}.txt");
    let symlink_path = instructions_dir.join(&filename);
    if std::os::unix::fs::symlink(&real_file, &symlink_path).is_err() {
        // Symlink creation not available; skip.
        return;
    }

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must reject a symbolic link on Unix");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("symbolic link") || msg.contains("symlink"),
        "error must identify the symbolic link as the reason for rejection; got: {msg}"
    );
}

/// On Windows, verify that a symbolic link or junction inside the instructions
/// directory is rejected via the reparse-point handle check.
///
/// The test skips gracefully when symlink or junction creation requires elevated
/// privileges not available in the test environment.
#[cfg(windows)]
#[tokio::test]
async fn get_instruction_rejects_symlink_dir_on_windows() {
    let (project_dir, instructions_dir, _) = create_isolated_test_env();

    let real_dir = tempfile::tempdir().unwrap();

    // Attempt to create a directory symbolic link inside the instructions directory.
    // On Windows, this requires the `SeCreateSymbolicLinkPrivilege` or Developer Mode.
    let link_name = format!("vector-instruction-{VALID_UUID}.txt");
    let link_path = instructions_dir.join(&link_name);
    if std::os::windows::fs::symlink_dir(real_dir.path(), &link_path).is_err() {
        // Privilege not available; skip.
        return;
    }

    let input = GetInstructionInput::new(IoPath::new(project_dir.path()), VALID_UUID.to_string());
    let mut sender = MockSender::new();
    let result = super::get_instruction(input, &mut sender).await;

    assert!(result.is_err(), "operation must reject a reparse point on Windows");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("reparse") || msg.contains("directory") || msg.contains("not found"),
        "error must describe the non-regular target; got: {msg}"
    );
}
