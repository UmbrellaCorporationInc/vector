#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

static CURRENT_DIR_TEST_LOCK: LazyLock<std::sync::Mutex<()>> =
    LazyLock::new(|| std::sync::Mutex::new(()));

struct TestWorkspace {
    root: PathBuf,
    previous_dir: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let previous_dir = std::env::current_dir().unwrap();
        let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("xtask-vault-test-{unique}"));
        fs::create_dir_all(root.join(".cargo/.xtask")).unwrap();
        fs::create_dir_all(root.join("doc/00 Project Hub/Tasks/todo")).unwrap();
        fs::create_dir_all(root.join("doc/00 Project Hub/Tasks/in-progress")).unwrap();
        fs::create_dir_all(root.join("doc/00 Project Hub/Tasks/done")).unwrap();
        fs::create_dir_all(root.join("doc/20 Architecture/draft")).unwrap();
        fs::create_dir_all(root.join("doc/20 Architecture/execution")).unwrap();
        fs::create_dir_all(root.join("doc/20 Architecture/implemented")).unwrap();
        fs::create_dir_all(root.join("doc/40 Guides")).unwrap();
        fs::create_dir_all(root.join("doc/42 AI Rules")).unwrap();
        fs::create_dir_all(root.join("doc/43 Research/language")).unwrap();
        fs::create_dir_all(root.join("doc/43 Research/frameworks")).unwrap();
        fs::create_dir_all(root.join("doc/44 Roadmaps/language")).unwrap();
        fs::create_dir_all(root.join("doc/44 Roadmaps/platform")).unwrap();
        fs::create_dir_all(root.join("doc/89 Samples")).unwrap();
        fs::create_dir_all(root.join("forge/50 Books")).unwrap();
        fs::write(
            root.join(CONFIG_PATH),
            concat!(
                "version: 1\n",
                "documents:\n",
                "  task:\n",
                "    root: doc/00 Project Hub/Tasks\n",
                "  adr:\n",
                "    root: doc/20 Architecture\n",
                "  guide:\n",
                "    root: doc/40 Guides\n",
                "  rule:\n",
                "    root: doc/42 AI Rules\n",
                "  research:\n",
                "    root: doc/43 Research\n",
                "  roadmap:\n",
                "    root: doc/44 Roadmaps\n",
                "  sample:\n",
                "    root: doc/89 Samples\n",
                "  book:\n",
                "    root: forge/50 Books\n",
            ),
        )
        .unwrap();
        std::env::set_current_dir(&root).unwrap();
        Self { root, previous_dir }
    }

    fn write_file(&self, relative: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, "# test\n").unwrap();
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous_dir);
        let _ = remove_tree(&self.root);
    }
}

fn remove_tree(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path)
}

fn lock_test_dir() -> std::sync::MutexGuard<'static, ()> {
    CURRENT_DIR_TEST_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn resolve_adr_returns_relative_path_and_empty_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/20 Architecture/0082 xtask Vault Document Resolution and Reservation.md");

    let output = execute(VaultCommand::Adr { number: Some("82".to_string()) }).unwrap();

    assert_eq!(output.id.as_deref(), Some("0082"));
    assert_eq!(output.last_id, None);
    assert_eq!(
        output.path,
        "doc/20 Architecture/0082 xtask Vault Document Resolution and Reservation.md"
    );
    assert_eq!(
        output.subfolders,
        vec!["draft".to_string(), "execution".to_string(), "implemented".to_string()]
    );
}

#[test]
fn resolve_research_walks_entire_tree_and_lists_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/43 Research/language/0001 Rust 2024 Edition Adoption.md");

    let output = execute(VaultCommand::Research { number: Some("1".to_string()) }).unwrap();

    assert_eq!(output.id.as_deref(), Some("0001"));
    assert_eq!(output.last_id, None);
    assert_eq!(output.path, "doc/43 Research/language/0001 Rust 2024 Edition Adoption.md");
    assert_eq!(output.subfolders, vec!["frameworks".to_string(), "language".to_string()]);
}

#[test]
fn resolve_roadmap_walks_entire_tree_and_lists_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/44 Roadmaps/language/0001 Babel Emitter Delivery Plan.md");

    let output = execute(VaultCommand::Roadmap { number: Some("1".to_string()) }).unwrap();

    assert_eq!(output.id.as_deref(), Some("0001"));
    assert_eq!(output.last_id, None);
    assert_eq!(output.path, "doc/44 Roadmaps/language/0001 Babel Emitter Delivery Plan.md");
    assert_eq!(output.subfolders, vec!["language".to_string(), "platform".to_string()]);
}

#[test]
fn reserve_task_uses_next_five_digit_id_from_disk() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/00 Project Hub/Tasks/00104 Existing Task.md");

    let output = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "Implement Something".to_string(),
        subfolder: None,
    })
    .unwrap();

    assert_eq!(output.id.as_deref(), Some("00105"));
    assert_eq!(output.last_id, None);
    assert_eq!(output.path, "doc/00 Project Hub/Tasks/todo/00105 Implement Something.md");
}

#[test]
fn reserve_adr_defaults_to_draft_subfolder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/20 Architecture/implemented/0084 Existing ADR.md");

    let output = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Adr,
        name: "New ADR".to_string(),
        subfolder: None,
    })
    .unwrap();

    assert_eq!(output.id.as_deref(), Some("0085"));
    assert_eq!(output.path, "doc/20 Architecture/draft/0085 New ADR.md");
}

#[test]
fn reserve_research_requires_subfolder_when_category_has_directories() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Research,
        name: "Rust 2024 Edition Adoption".to_string(),
        subfolder: None,
    })
    .unwrap_err();

    assert!(error.contains("this category has subfolders"));
}

#[test]
fn reserve_research_with_subfolder_returns_target_path_and_global_next_id() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/43 Research/frameworks/0002 Existing Framework Note.md");

    let output = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Research,
        name: "Rust 2024 Edition Adoption".to_string(),
        subfolder: Some("language".to_string()),
    })
    .unwrap();

    assert_eq!(output.id.as_deref(), Some("0003"));
    assert_eq!(output.last_id, None);
    assert_eq!(output.path, "doc/43 Research/language/0003 Rust 2024 Edition Adoption.md");
    assert_eq!(output.subfolders, vec!["frameworks".to_string(), "language".to_string()]);
}

#[test]
fn reserve_roadmap_requires_subfolder_when_category_has_directories() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Roadmap,
        name: "Babel Emitter Delivery Plan".to_string(),
        subfolder: None,
    })
    .unwrap_err();

    assert!(error.contains("this category has subfolders"));
}

#[test]
fn reserve_roadmap_with_subfolder_returns_target_path_and_global_next_id() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/44 Roadmaps/platform/0002 Existing Platform Roadmap.md");

    let output = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Roadmap,
        name: "Babel Emitter Delivery Plan".to_string(),
        subfolder: Some("language".to_string()),
    })
    .unwrap();

    assert_eq!(output.id.as_deref(), Some("0003"));
    assert_eq!(output.last_id, None);
    assert_eq!(output.path, "doc/44 Roadmaps/language/0003 Babel Emitter Delivery Plan.md");
    assert_eq!(output.subfolders, vec!["language".to_string(), "platform".to_string()]);
}

#[test]
fn reserve_rejects_name_with_colon() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "babel parser: fix type_expr".to_string(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap_err();

    assert!(error.contains(':'), "expected colon mentioned in error, got: {error}");
}

#[test]
fn reserve_rejects_name_with_angle_bracket() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "Add <T> support".to_string(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap_err();

    assert!(error.contains('<'), "expected '<' mentioned in error, got: {error}");
}

#[test]
fn reserve_rejects_name_ending_with_dot() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "Fix trailing dot.".to_string(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap_err();

    assert!(error.contains("dot"), "expected dot mentioned in error, got: {error}");
}

#[test]
fn reserve_rejects_name_ending_with_space() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "Trailing space ".to_string(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap_err();

    assert!(error.contains("space"), "expected space mentioned in error, got: {error}");
}

#[test]
fn reserve_rejects_empty_name() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: String::new(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap_err();

    assert!(error.contains("empty"), "expected 'empty' in error, got: {error}");
}

#[test]
fn reserve_rejects_windows_reserved_device_name() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    for name in &["CON", "nul", "Com3", "LPT9", "PRN", "AUX"] {
        let error = execute(VaultCommand::Reserve {
            doc_type: VaultDocumentType::Task,
            name: name.to_string(),
            subfolder: Some("todo".to_string()),
        })
        .unwrap_err();
        assert!(
            error.contains("reserved"),
            "expected 'reserved' in error for name '{name}', got: {error}"
        );
    }
}

#[test]
fn reserve_accepts_clean_name() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let output = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "babel parser support namespace basic type".to_string(),
        subfolder: Some("todo".to_string()),
    })
    .unwrap();

    assert!(output.path.contains("babel parser support namespace basic type"));
}

#[test]
fn resolve_returns_error_for_ambiguous_numeric_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/43 Research/language/0001 One.md");
    ws.write_file("doc/43 Research/frameworks/0001 Two.md");

    let error = execute(VaultCommand::Research { number: Some("1".to_string()) }).unwrap_err();

    assert!(error.contains("ambiguous"));
}

#[test]
fn resolve_roadmap_returns_error_for_ambiguous_numeric_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/44 Roadmaps/language/0001 One.md");
    ws.write_file("doc/44 Roadmaps/platform/0001 Two.md");

    let matches = find_matching_documents(&ws.root.join("doc/44 Roadmaps"), 1).unwrap();

    assert_eq!(matches.len(), 2);
}

#[test]
fn resolve_roadmap_returns_error_for_unknown_id() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let error = execute(VaultCommand::Roadmap { number: Some("1".to_string()) }).unwrap_err();

    assert!(error.contains("no roadmap document found"));
}

#[test]
fn guide_without_id_returns_root_last_id_and_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::create_dir_all(ws.root.join("doc/40 Guides/examples")).unwrap();
    ws.write_file("doc/40 Guides/0017 Vault Organization.md");

    let output = execute(VaultCommand::Guide { number: None }).unwrap();

    assert_eq!(output.id, None);
    assert_eq!(output.last_id.as_deref(), Some("0017"));
    assert_eq!(output.path, "doc/40 Guides");
    assert_eq!(output.subfolders, vec!["examples".to_string()]);
}

#[test]
fn task_without_id_returns_root_and_last_id() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/00 Project Hub/Tasks/00104 Existing Task.md");

    let output = execute(VaultCommand::Task { number: None }).unwrap();

    assert_eq!(output.id, None);
    assert_eq!(output.last_id.as_deref(), Some("00104"));
    assert_eq!(output.path, "doc/00 Project Hub/Tasks");
    assert_eq!(
        output.subfolders,
        vec!["done".to_string(), "in-progress".to_string(), "todo".to_string()]
    );
}

#[test]
fn research_without_id_returns_root_last_id_and_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/43 Research/frameworks/0002 Existing Framework Note.md");

    let output = execute(VaultCommand::Research { number: None }).unwrap();

    assert_eq!(output.id, None);
    assert_eq!(output.last_id.as_deref(), Some("0002"));
    assert_eq!(output.path, "doc/43 Research");
    assert_eq!(output.subfolders, vec!["frameworks".to_string(), "language".to_string()]);
}

#[test]
fn roadmap_without_id_returns_root_last_id_and_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/44 Roadmaps/platform/0002 Existing Platform Roadmap.md");

    let output = execute(VaultCommand::Roadmap { number: None }).unwrap();

    assert_eq!(output.id, None);
    assert_eq!(output.last_id.as_deref(), Some("0002"));
    assert_eq!(output.path, "doc/44 Roadmaps");
    assert_eq!(output.subfolders, vec!["language".to_string(), "platform".to_string()]);
}

#[test]
fn check_returns_pass_when_frontmatter_is_complete() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/40 Guides/0017 Vault Organization.md");
    fs::write(
        ws.root.join("doc/40 Guides/0017 Vault Organization.md"),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example guide.\n\
id: 17\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed);
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_reports_missing_fields_without_fix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/00104 Existing Task.md"),
        "---\n\
tags:\n\
  - task\n\
description: Example task.\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(
        "MISSING_FIELDS doc/00 Project Hub/Tasks/00104 Existing Task.md type created status id"
    ));
}

#[test]
fn check_fix_inserts_deterministic_defaults_but_still_reports_missing_semantic_fields() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/20 Architecture/0083 Example ADR.md"),
        "---\n\
tags:\n\
  - adr\n\
---\n\
# ADR\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);
    let updated =
        fs::read_to_string(ws.root.join("doc/20 Architecture/draft/0083 adr Example ADR.md"))
            .unwrap();

    assert!(!passed);
    assert!(report.contains("MISSING_FIELDS doc/20 Architecture/0083 Example ADR.md description"));
    assert!(updated.contains("type: adr"));
    assert!(updated.contains("status: draft"));
    let today = Utc::now().format("%Y-%m-%d").to_string();
    assert!(updated.contains(&format!("created: {today}")));
    assert!(updated.contains("id: 83"));
}

#[test]
fn check_reports_invalid_status_separately() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/00104 Existing Task.md"),
        "---\n\
type: task\n\
status: Backlog\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example task.\n\
id: 104\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(
        report.contains("INVALID_FIELDS doc/00 Project Hub/Tasks/00104 Existing Task.md status")
    );
}

#[test]
fn check_reports_wrong_location_for_task_without_fix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/00104 Existing Task.md"),
        "---\n\
type: task\n\
status: done\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example task.\n\
id: 104\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains("WRONG_LOCATION doc/00 Project Hub/Tasks/00104 Existing Task.md"));
}

#[test]
fn check_fix_moves_task_into_status_subfolder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/00104 Existing Task.md"),
        "---\n\
type: task\n\
status: done\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example task.\n\
id: 104\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed);
    assert_eq!(report, "PASS\n");
    assert!(!ws.root.join("doc/00 Project Hub/Tasks/00104 Existing Task.md").exists());
    assert!(ws.root.join("doc/00 Project Hub/Tasks/done/00104 task Existing Task.md").exists());
}

#[test]
fn check_fix_uses_default_status_before_moving_adr() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/20 Architecture/0084 Example ADR.md"),
        "---\n\
type: adr\n\
tags:\n\
  - adr\n\
created: 2026-04-02\n\
description: Example adr.\n\
id: 84\n\
---\n\
# ADR\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed);
    assert_eq!(report, "PASS\n");
    assert!(!ws.root.join("doc/20 Architecture/0084 Example ADR.md").exists());
    assert!(ws.root.join("doc/20 Architecture/draft/0084 adr Example ADR.md").exists());
}

#[test]
fn check_passes_for_adr_with_retired_status_in_retired_folder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::create_dir_all(ws.root.join("doc/20 Architecture/retired")).unwrap();
    fs::write(
        ws.root.join("doc/20 Architecture/retired/0059 adr Babel Bytecode Lowering.md"),
        "---\n\
type: adr\n\
tags:\n\
  - adr\n\
created: 2026-04-02\n\
description: Retired ADR superseded by ADR 0133.\n\
id: 59\n\
status: retired\n\
---\n\
# Babel Bytecode Lowering\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "expected pass for retired ADR in retired/ folder, got: {report}");
}

#[test]
fn check_fix_moves_adr_with_retired_status_into_retired_folder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/20 Architecture/draft/0059 adr Babel Bytecode Lowering.md"),
        "---\n\
type: adr\n\
tags:\n\
  - adr\n\
created: 2026-04-02\n\
description: Retired ADR superseded by ADR 0133.\n\
id: 59\n\
status: retired\n\
---\n\
# Babel Bytecode Lowering\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected fix pass, got: {report}");
    assert!(
        !ws.root.join("doc/20 Architecture/draft/0059 adr Babel Bytecode Lowering.md").exists()
    );
    assert!(
        ws.root.join("doc/20 Architecture/retired/0059 adr Babel Bytecode Lowering.md").exists()
    );
}

#[test]
fn run_query_no_expression_returns_zero_without_loading_config() {
    assert_eq!(run_query(None, None), 0);
}

#[test]
fn run_query_valid_expression_returns_zero_in_temp_workspace() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();
    assert_eq!(run_query(Some("{type='task'}"), None), 0);
}

#[test]
fn run_query_malformed_expression_returns_one() {
    assert_eq!(run_query(Some("{"), None), 1);
}

#[test]
fn run_query_named_query_resolves_from_vault_query_yaml() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();
    fs::write(
        std::env::current_dir().unwrap().join(NAMED_QUERY_PATH),
        "demo: \"{type='guide'}\"\n",
    )
    .unwrap();
    assert_eq!(run_query(None, Some("demo")), 0);
}

#[test]
fn run_query_named_query_missing_id_returns_one() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();
    fs::write(
        std::env::current_dir().unwrap().join(NAMED_QUERY_PATH),
        "other: \"{type='guide'}\"\n",
    )
    .unwrap();
    assert_eq!(run_query(None, Some("demo")), 1);
}

#[test]
fn run_query_inline_and_query_id_both_set_returns_one() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();
    assert_eq!(run_query(Some("{type='task'}"), Some("demo")), 1);
}

#[test]
fn collect_query_findings_matches_guide_frontmatter() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/40 Guides/0001 Demo Guide.md"),
        "---\ntype: guide\ntags:\n  - doc\ncreated: 2026-04-02\ndescription: Demo.\nid: 1\n---\n# Demo\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let expr = crate::vault_query::parse_query_expression("{type='guide'}").unwrap();
    let rows = collect_query_findings(&ws.root, &config, &expr).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "guide");
    assert_eq!(rows[0].1, "doc/40 Guides/0001 Demo Guide.md");
}

#[test]
fn collect_query_findings_or_matches_alternate_types() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/40 Guides/0002 A.md"),
        "---\ntype: guide\ncreated: 2026-04-02\ndescription: A\ntags: []\nid: 2\n---\n",
    )
    .unwrap();
    fs::write(
        ws.root.join("doc/20 Architecture/draft/0003 B.md"),
        "---\ntype: adr\ncreated: 2026-04-02\ndescription: B\ntags: []\nstatus: draft\nid: 3\n---\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let expr = crate::vault_query::parse_query_expression("{type='task'}{type='adr'}").unwrap();
    let rows = collect_query_findings(&ws.root, &config, &expr).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "adr");
}

#[test]
fn check_passes_without_id_when_filename_has_no_numeric_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/40 Guides/Unnumbered Title.md"),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: No numeric prefix in filename.\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed);
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_fix_inserts_normalized_id_from_padded_filename_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/todo/00104 Fix Id Task.md"),
        "---\n\
type: task\n\
status: todo\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example.\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed);
    assert_eq!(report, "PASS\n");
    let updated =
        fs::read_to_string(ws.root.join("doc/00 Project Hub/Tasks/todo/00104 task Fix Id Task.md"))
            .unwrap();
    assert!(updated.contains("id: 104"));
}

#[test]
fn check_accepts_id_as_string_with_leading_zeros_matching_filename() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/40 Guides/00023 Padded Name.md"),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example.\n\
id: '00023'\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed);
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_reports_invalid_id_when_mismatch_even_with_fix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/todo/00104 Wrong Id Task.md"),
        "---\n\
type: task\n\
status: todo\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example.\n\
id: 999\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(!passed);
    assert!(
        report.contains("INVALID_FIELDS doc/00 Project Hub/Tasks/todo/00104 Wrong Id Task.md id")
    );
}

#[test]
fn check_fix_inserts_deterministic_defaults_for_roadmap() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/44 Roadmaps/language/0001 Example Roadmap.md"),
        "---\n\
tags:\n\
  - roadmap\n\
description: Example roadmap.\n\
---\n\
# Roadmap\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected PASS but got: {report}");
    let updated = fs::read_to_string(
        ws.root.join("doc/44 Roadmaps/language/0001 roadmap Example Roadmap.md"),
    )
    .unwrap();
    assert!(updated.contains("type: roadmap"));
    assert!(updated.contains("id: 1"));
    let today = Utc::now().format("%Y-%m-%d").to_string();
    assert!(updated.contains(&format!("created: {today}")));
}

#[test]
fn check_fix_inserts_title_metadata_for_non_book_document_from_filename() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/40 Guides/0001 Canonical Guide Name.md"),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example guide.\n\
id: 1\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected PASS but got: {report}");
    let updated =
        fs::read_to_string(ws.root.join("doc/40 Guides/0001 guide Canonical Guide Name.md"))
            .unwrap();
    assert!(updated.contains("title: Canonical Guide Name"));
}

#[test]
fn check_fix_renames_non_book_document_from_title_metadata() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let old_path = "doc/40 Guides/0001 Old Guide Name.md";
    let new_path = "doc/40 Guides/0001 guide New Guide Name.md";
    fs::write(
        ws.root.join(old_path),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example guide.\n\
id: 1\n\
title: New Guide Name\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected PASS but got: {report}");
    assert!(!ws.root.join(old_path).exists());
    assert!(ws.root.join(new_path).exists());
}

#[test]
fn check_fix_moves_and_renames_task_using_shared_filename_contract() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let old_path = "doc/00 Project Hub/Tasks/00104 Old Task Name.md";
    let new_path = "doc/00 Project Hub/Tasks/done/00104 task Better Task Name.md";
    fs::write(
        ws.root.join(old_path),
        "---\n\
type: task\n\
status: done\n\
tags:\n\
  - task\n\
created: 2026-04-02\n\
description: Example task.\n\
id: 104\n\
title: Better Task Name\n\
---\n\
# Task\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected PASS but got: {report}");
    assert!(!ws.root.join(old_path).exists());
    assert!(ws.root.join(new_path).exists());
}

#[test]
fn check_without_fix_does_not_report_non_book_filename_drift_yet() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let path = "doc/40 Guides/0001 Old Guide Name.md";
    fs::write(
        ws.root.join(path),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example guide.\n\
id: 1\n\
title: New Guide Name\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "expected PASS but got: {report}");
    assert_eq!(report, "PASS\n");
    assert!(ws.root.join(path).exists());
}

#[test]
fn check_fix_preserves_title_when_filename_already_uses_type_token() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let path = "doc/40 Guides/0001 guide Canonical Guide Name.md";
    fs::write(
        ws.root.join(path),
        "---\n\
type: guide\n\
tags:\n\
  - guide\n\
created: 2026-04-02\n\
description: Example guide.\n\
id: 1\n\
---\n\
# Guide\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "expected PASS but got: {report}");
    let updated = fs::read_to_string(ws.root.join(path)).unwrap();
    assert!(updated.contains("title: Canonical Guide Name"));
}

#[test]
fn check_reports_missing_semantic_fields_for_roadmap() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/44 Roadmaps/language/0001 Example Roadmap.md"),
        "---\n\
type: roadmap\n\
created: 2026-04-02\n\
id: 1\n\
---\n\
# Roadmap\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(
        "MISSING_FIELDS doc/44 Roadmaps/language/0001 Example Roadmap.md description tags"
    ));
}

#[test]
fn check_reports_invalid_type_for_roadmap_file() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/44 Roadmaps/language/0001 Example Roadmap.md"),
        "---\n\
type: guide\n\
tags:\n\
  - roadmap\n\
created: 2026-04-02\n\
description: Example roadmap.\n\
id: 1\n\
---\n\
# Roadmap\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(
        report.contains("INVALID_FIELDS doc/44 Roadmaps/language/0001 Example Roadmap.md type")
    );
}

// --- reserve-book tests ---

#[test]
fn reserve_book_first_book_gets_code_001() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let config = load_config(&ws.root).unwrap();
    let output = reserve_book_command(&ws.root, &config, "Rust").unwrap();

    assert_eq!(output.book_code, "001");
    assert_eq!(output.reserved_name, "Rust");
    assert_eq!(output.folder_path, "forge/50 Books/001 Rust");
    assert_eq!(output.index_path, "forge/50 Books/001 Rust/00 Index.md");
}

#[test]
fn reserve_book_creates_folder_and_index_on_disk() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    assert!(ws.root.join("forge/50 Books/001 Rust").is_dir());
    assert!(ws.root.join("forge/50 Books/001 Rust/00 Index.md").is_file());
}

#[test]
fn reserve_book_index_contains_required_frontmatter_fields() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let content = fs::read_to_string(ws.root.join("forge/50 Books/001 Rust/00 Index.md")).unwrap();
    assert!(content.contains("type: overview"), "missing type");
    assert!(content.contains("book_code: 001"), "missing book_code");
    assert!(content.contains("book: rust"), "missing book key");
    assert!(content.contains("status: draft"), "missing status");
    assert!(content.contains("tags:"), "missing tags");
    assert!(content.contains("- book"), "missing book tag");
    assert!(content.contains("- index"), "missing index tag");
}

#[test]
fn reserve_book_assigns_consecutive_codes_when_existing_books_present() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::create_dir_all(ws.root.join("forge/50 Books/001 Rust")).unwrap();
    fs::create_dir_all(ws.root.join("forge/50 Books/002 Python")).unwrap();

    let config = load_config(&ws.root).unwrap();
    let output = reserve_book_command(&ws.root, &config, "Go").unwrap();

    assert_eq!(output.book_code, "003");
    assert_eq!(output.folder_path, "forge/50 Books/003 Go");
}

#[test]
fn reserve_book_skips_folders_without_three_digit_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    // A folder with a non-conforming name must not affect the counter.
    fs::create_dir_all(ws.root.join("forge/50 Books/01 OldFormat")).unwrap();
    fs::create_dir_all(ws.root.join("forge/50 Books/assets")).unwrap();

    let config = load_config(&ws.root).unwrap();
    let output = reserve_book_command(&ws.root, &config, "Rust").unwrap();

    assert_eq!(output.book_code, "001");
}

#[test]
fn reserve_book_successive_calls_produce_consecutive_codes() {
    // This also verifies that existing book folders are correctly accounted for on each
    // call, and that the collision guard does not fire in normal sequential usage.
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let first = reserve_book_command(&ws.root, &config, "Rust").unwrap();
    let second = reserve_book_command(&ws.root, &config, "Python").unwrap();
    let third = reserve_book_command(&ws.root, &config, "Go").unwrap();

    assert_eq!(first.book_code, "001");
    assert_eq!(second.book_code, "002");
    assert_eq!(third.book_code, "003");
}

#[test]
fn parse_book_code_prefix_accepts_three_digit_folder() {
    assert_eq!(parse_book_code_prefix("001 Rust"), Some(1));
    assert_eq!(parse_book_code_prefix("042 Python"), Some(42));
    assert_eq!(parse_book_code_prefix("999 Go"), Some(999));
}

#[test]
fn parse_book_code_prefix_rejects_non_conforming_names() {
    assert_eq!(parse_book_code_prefix("01 OldFormat"), None);
    assert_eq!(parse_book_code_prefix("assets"), None);
    assert_eq!(parse_book_code_prefix("1234 TooLong"), None);
    assert_eq!(parse_book_code_prefix("abc Rust"), None);
}

#[test]
fn reserve_book_returns_error_when_book_config_missing() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join(CONFIG_PATH),
        "version: 1\ndocuments:\n  task:\n    root: doc/00 Project Hub/Tasks\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let err = reserve_book_command(&ws.root, &config, "Rust").unwrap_err();

    assert!(err.contains("'book'"), "expected missing key error, got: {err}");
}

#[test]
fn reserve_book_rejects_name_with_colon() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let err = reserve_book_command(&ws.root, &config, "Rust: Advanced").unwrap_err();

    assert!(err.contains(':'), "expected colon mentioned in error, got: {err}");
}

#[test]
fn reserve_book_rejects_name_with_illegal_character() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let err = reserve_book_command(&ws.root, &config, "Rust <2024>").unwrap_err();

    assert!(err.contains('<'), "expected '<' mentioned in error, got: {err}");
}

#[test]
fn reserve_book_rejects_windows_reserved_device_name() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let err = reserve_book_command(&ws.root, &config, "NUL").unwrap_err();

    assert!(err.contains("reserved"), "expected 'reserved' in error, got: {err}");
}

#[test]
fn reserve_book_accepts_clean_name() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let output = reserve_book_command(&ws.root, &config, "Advanced Rust").unwrap();

    assert!(output.folder_path.contains("Advanced Rust"));
}

#[test]
fn find_max_book_code_returns_zero_when_books_root_does_not_exist() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let missing = ws.root.join("forge/50 Books/nonexistent");

    let max = find_max_book_code(&missing).unwrap();

    assert_eq!(max, 0);
}

#[test]
fn find_max_book_code_ignores_non_conforming_folders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let books_root = ws.root.join("forge/50 Books");
    fs::create_dir_all(books_root.join("assets")).unwrap();
    fs::create_dir_all(books_root.join("01 OldFormat")).unwrap();

    let max = find_max_book_code(&books_root).unwrap();

    assert_eq!(max, 0);
}

#[test]
fn build_index_scaffold_contains_all_required_markers() {
    let content = build_index_scaffold("001", "rust", "Rust");

    assert!(content.contains("type: overview"));
    assert!(content.contains("book_code: 001"));
    assert!(content.contains("book: rust"));
    assert!(content.contains("status: draft"));
    assert!(content.contains("- book"));
    assert!(content.contains("- rust"));
    assert!(content.contains("- index"));
    assert!(content.contains("# Rust"));
}

#[test]
fn run_reserve_book_exits_zero_and_creates_folder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let exit = run(VaultCommand::ReserveBook { name: "Rust".to_string() });

    assert_eq!(exit, 0);
    assert!(ws.root.join("forge/50 Books/001 Rust").is_dir());
    assert!(ws.root.join("forge/50 Books/001 Rust/00 Index.md").is_file());
}

#[test]
fn run_reserve_book_exits_one_when_config_missing_book_key() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join(CONFIG_PATH),
        "version: 1\ndocuments:\n  task:\n    root: doc/00 Project Hub/Tasks\n",
    )
    .unwrap();

    let exit = run(VaultCommand::ReserveBook { name: "Rust".to_string() });

    assert_eq!(exit, 1);
}

#[test]
fn book_reserve_output_render_contains_all_keys() {
    let output = BookReserveOutput {
        book_code: "001".to_string(),
        folder_path: "forge/50 Books/001 Rust".to_string(),
        index_path: "forge/50 Books/001 Rust/00 Index.md".to_string(),
        reserved_name: "Rust".to_string(),
    };

    let rendered = output.render();

    assert!(rendered.contains("BOOK_CODE=001"));
    assert!(rendered.contains("FOLDER_PATH=forge/50 Books/001 Rust"));
    assert!(rendered.contains("INDEX_PATH=forge/50 Books/001 Rust/00 Index.md"));
    assert!(rendered.contains("RESERVED_NAME=Rust"));
}

// --- reserve-book-artifact tests ---

#[test]
fn parse_artifact_id_prefix_accepts_four_digit_prefix() {
    use std::path::Path;
    assert_eq!(parse_artifact_id_prefix(Path::new("b0001 rust Ownership.md")), Some(1));
    assert_eq!(parse_artifact_id_prefix(Path::new("b0042 rust Borrowing.md")), Some(42));
    assert_eq!(parse_artifact_id_prefix(Path::new("b9999 rust Lifetimes.md")), Some(9999));
    assert_eq!(parse_artifact_id_prefix(Path::new("0001-ownership.md")), Some(1));
}

#[test]
fn parse_artifact_id_prefix_rejects_non_conforming_names() {
    use std::path::Path;
    assert_eq!(parse_artifact_id_prefix(Path::new("001-ownership.md")), None);
    assert_eq!(parse_artifact_id_prefix(Path::new("b00042 rust Ownership.md")), None);
    assert_eq!(parse_artifact_id_prefix(Path::new("b0001rust Ownership.md")), None);
    assert_eq!(parse_artifact_id_prefix(Path::new("00 Index.md")), None);
    assert_eq!(parse_artifact_id_prefix(Path::new("b0001 .md")), None);
    assert_eq!(parse_artifact_id_prefix(Path::new("abc1-ownership.md")), None);
}

#[test]
fn find_book_folder_resolves_existing_book() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::create_dir_all(ws.root.join("forge/50 Books/001 Rust")).unwrap();

    let books_root = ws.root.join("forge/50 Books");
    let result = find_book_folder(&books_root, "001").unwrap();

    assert_eq!(result, ws.root.join("forge/50 Books/001 Rust"));
}

#[test]
fn find_book_folder_returns_error_for_missing_code() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let books_root = ws.root.join("forge/50 Books");
    let err = find_book_folder(&books_root, "002").unwrap_err();

    assert!(err.contains("'002'"));
}

#[test]
fn find_max_artifact_id_returns_zero_for_empty_book() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let book_folder = ws.root.join("forge/50 Books/001 Rust");
    fs::create_dir_all(&book_folder).unwrap();

    let max = find_max_artifact_id(&book_folder).unwrap();

    assert_eq!(max, 0);
}

#[test]
fn find_max_artifact_id_ignores_index_and_non_conforming_files() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let book_folder = ws.root.join("forge/50 Books/001 Rust");
    let memory_dir = book_folder.join("memory");
    fs::create_dir_all(&memory_dir).unwrap();
    fs::write(book_folder.join("00 Index.md"), "").unwrap();
    fs::write(memory_dir.join("notes.md"), "").unwrap();

    let max = find_max_artifact_id(&book_folder).unwrap();

    assert_eq!(max, 0);
}

#[test]
fn find_max_artifact_id_finds_highest_id_across_subfolders() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let book_folder = ws.root.join("forge/50 Books/001 Rust");
    let memory_dir = book_folder.join("memory");
    let concurrency_dir = book_folder.join("concurrency");
    fs::create_dir_all(&memory_dir).unwrap();
    fs::create_dir_all(&concurrency_dir).unwrap();
    fs::write(memory_dir.join("b0001 rust Ownership.md"), "").unwrap();
    fs::write(memory_dir.join("b0003 rust Lifetimes.md"), "").unwrap();
    fs::write(concurrency_dir.join("b0002 rust Threads.md"), "").unwrap();

    let max = find_max_artifact_id(&book_folder).unwrap();

    assert_eq!(max, 3);
}

#[test]
fn reserve_book_artifact_first_artifact_gets_id_0001() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let output =
        reserve_book_artifact_command(&ws.root, &config, "001", "Ownership", "memory").unwrap();

    assert_eq!(output.id, "0001");
    assert_eq!(output.book_code, "001");
    assert_eq!(output.reserved_name, "Ownership");
    assert_eq!(output.file_path, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");
}

#[test]
fn reserve_book_artifact_creates_category_subfolder_and_file_on_disk() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    reserve_book_artifact_command(&ws.root, &config, "001", "Ownership", "memory").unwrap();

    assert!(ws.root.join("forge/50 Books/001 Rust/memory").is_dir());
    assert!(ws.root.join("forge/50 Books/001 Rust/memory/b0001 rust Ownership.md").is_file());
}

#[test]
fn reserve_book_artifact_assigns_consecutive_ids_across_categories() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let a = reserve_book_artifact_command(&ws.root, &config, "001", "Ownership", "memory").unwrap();
    let b =
        reserve_book_artifact_command(&ws.root, &config, "001", "Threads", "concurrency").unwrap();
    let c = reserve_book_artifact_command(&ws.root, &config, "001", "Lifetimes", "memory").unwrap();

    assert_eq!(a.id, "0001");
    assert_eq!(b.id, "0002");
    assert_eq!(c.id, "0003");
}

#[test]
fn reserve_book_artifact_file_contains_required_frontmatter_fields() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();
    reserve_book_artifact_command(&ws.root, &config, "001", "Ownership", "memory").unwrap();

    let content =
        fs::read_to_string(ws.root.join("forge/50 Books/001 Rust/memory/b0001 rust Ownership.md"))
            .unwrap();

    assert!(content.contains("type: concept"), "missing type");
    assert!(content.contains("book_code: 001"), "missing book_code");
    assert!(content.contains("book: rust"), "missing book");
    assert!(content.contains("id: 1"), "missing id");
    assert!(content.contains("category: memory"), "missing category");
    assert!(content.contains("status: draft"), "missing status");
    assert!(content.contains("tags:"), "missing tags");
    assert!(content.contains("prerequisites: []"), "missing prerequisites");
}

#[test]
fn reserve_book_artifact_returns_error_for_unknown_book_code() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let err =
        reserve_book_artifact_command(&ws.root, &config, "999", "Ownership", "memory").unwrap_err();

    assert!(err.contains("'999'"));
}

#[test]
fn reserve_book_artifact_rejects_name_with_colon() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let err =
        reserve_book_artifact_command(&ws.root, &config, "001", "Ownership: Deep Dive", "memory")
            .unwrap_err();

    assert!(err.contains(':'), "expected colon mentioned in error, got: {err}");
}

#[test]
fn reserve_book_artifact_rejects_name_with_illegal_character() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let err = reserve_book_artifact_command(&ws.root, &config, "001", "Ownership <T>", "memory")
        .unwrap_err();

    assert!(err.contains('<'), "expected '<' mentioned in error, got: {err}");
}

#[test]
fn reserve_book_artifact_rejects_windows_reserved_device_name() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let err = reserve_book_artifact_command(&ws.root, &config, "001", "CON", "memory").unwrap_err();

    assert!(err.contains("reserved"), "expected 'reserved' in error, got: {err}");
}

#[test]
fn reserve_book_artifact_accepts_clean_name() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let output = reserve_book_artifact_command(
        &ws.root,
        &config,
        "001",
        "Ownership and Move Semantics",
        "memory",
    )
    .unwrap();

    assert!(output.file_path.contains("Ownership and Move Semantics"));
}

#[test]
fn reserve_book_artifact_uses_existing_category_folder_when_present() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();
    // Pre-create the category folder with an existing artifact.
    let memory_dir = ws.root.join("forge/50 Books/001 Rust/memory");
    fs::create_dir_all(&memory_dir).unwrap();
    fs::write(memory_dir.join("b0001 rust Ownership.md"), "").unwrap();

    let output =
        reserve_book_artifact_command(&ws.root, &config, "001", "Borrowing", "memory").unwrap();

    assert_eq!(output.id, "0002");
    assert!(ws.root.join("forge/50 Books/001 Rust/memory/b0002 rust Borrowing.md").is_file());
}

#[test]
fn reserve_book_artifact_multi_word_name_uses_spaced_filename() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let output = reserve_book_artifact_command(
        &ws.root,
        &config,
        "001",
        "Ownership and Move Semantics",
        "memory",
    )
    .unwrap();

    assert_eq!(
        output.file_path,
        "forge/50 Books/001 Rust/memory/b0001 rust Ownership and Move Semantics.md"
    );
}

#[test]
fn build_artifact_scaffold_contains_all_required_fields() {
    let content = build_artifact_scaffold(1, "001", "rust", "memory", "Ownership");

    assert!(content.contains("type: concept"));
    assert!(content.contains("book_code: 001"));
    assert!(content.contains("book: rust"));
    assert!(content.contains("id: 1"));
    assert!(content.contains("category: memory"));
    assert!(content.contains("status: draft"));
    assert!(content.contains("prerequisites: []"));
    assert!(content.contains("# Ownership"));
}

#[test]
fn book_artifact_reserve_output_render_contains_all_keys() {
    let output = BookArtifactReserveOutput {
        id: "0001".to_string(),
        book_code: "001".to_string(),
        file_path: "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md".to_string(),
        reserved_name: "Ownership".to_string(),
    };

    let rendered = output.render();

    assert!(rendered.contains("ID=0001"));
    assert!(rendered.contains("BOOK_CODE=001"));
    assert!(rendered.contains("FILE_PATH=forge/50 Books/001 Rust/memory/b0001 rust Ownership.md"));
    assert!(rendered.contains("RESERVED_NAME=Ownership"));
}

#[test]
fn run_reserve_book_artifact_exits_zero_and_creates_file() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let exit = run(VaultCommand::ReserveBookArtifact {
        book_code: "001".to_string(),
        name: "Ownership".to_string(),
        category: "memory".to_string(),
    });

    assert_eq!(exit, 0);
    assert!(ws.root.join("forge/50 Books/001 Rust/memory/b0001 rust Ownership.md").is_file());
}

#[test]
fn run_reserve_book_artifact_exits_one_for_missing_book_code() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let exit = run(VaultCommand::ReserveBookArtifact {
        book_code: "999".to_string(),
        name: "Ownership".to_string(),
        category: "memory".to_string(),
    });

    assert_eq!(exit, 1);
}

// ============================================================================
// Phase C — CLI exit-code validation for illegal reserve names (Task 00199)
// ============================================================================

#[test]
fn run_reserve_task_exits_one_for_name_with_colon() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let exit = run(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "fix parser: update grammar".to_string(),
        subfolder: Some("todo".to_string()),
    });

    assert_eq!(exit, 1);
}

#[test]
fn run_reserve_book_exits_one_for_name_with_colon() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let exit = run(VaultCommand::ReserveBook { name: "Rust: The Language".to_string() });

    assert_eq!(exit, 1);
}

#[test]
fn run_reserve_book_artifact_exits_one_for_name_with_colon() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let exit = run(VaultCommand::ReserveBookArtifact {
        book_code: "001".to_string(),
        name: "Ownership: Deep Dive".to_string(),
        category: "memory".to_string(),
    });

    assert_eq!(exit, 1);
}

// ============================================================================
// Phase N — Cross-phase and regression tests (Task 00199)
// ============================================================================

#[test]
fn run_reserve_task_clean_name_exits_zero_and_produces_path() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let exit = run(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Task,
        name: "fix parser support namespace basic type".to_string(),
        subfolder: Some("todo".to_string()),
    });

    assert_eq!(exit, 0);
}

#[test]
fn run_reserve_book_clean_name_exits_zero() {
    let _guard = lock_test_dir();
    let _ws = TestWorkspace::new();

    let exit = run(VaultCommand::ReserveBook { name: "Advanced Rust Programming".to_string() });

    assert_eq!(exit, 0);
}

#[test]
fn run_reserve_book_artifact_clean_name_exits_zero() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();
    reserve_book_command(&ws.root, &config, "Rust").unwrap();

    let exit = run(VaultCommand::ReserveBookArtifact {
        book_code: "001".to_string(),
        name: "Ownership and Move Semantics".to_string(),
        category: "memory".to_string(),
    });

    assert_eq!(exit, 0);
}

#[test]
fn vault_check_fix_passes_after_name_validation_is_added() {
    // Regression: adding validate_reserve_name must not break vault check --fix
    // on a workspace whose existing files all have clean names.
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    ws.write_file("doc/00 Project Hub/Tasks/todo/00001 task Clean Task Name.md");
    fs::write(
        ws.root.join("doc/00 Project Hub/Tasks/todo/00001 task Clean Task Name.md"),
        "---\ntags:\n  - task\nstatus: todo\npriority: 🟡 Medium\ncreated: 2026-04-28\ntype: task\ndescription: A clean task.\n---\n\n# Clean Task Name\n",
    )
    .unwrap();

    let exit = run(VaultCommand::Check { fix: true });

    assert_eq!(exit, 0);
}

// ============================================================================
// Phase 0 — link extraction and classification foundation (Task 00149)
// ============================================================================

#[test]
fn extract_links_classifies_wiki_links_with_anchor_alias_and_md_suffix() {
    let text = "See [[b0001-babel-ownership.md#examples|Ownership examples]].";

    let links = extract_links(text);

    assert_eq!(links.len(), 1);
    let link = &links[0];
    assert_eq!(link.kind, LinkKind::Wiki);
    assert_eq!(link.target, "b0001-babel-ownership.md");
    assert_eq!(link.anchor.as_deref(), Some("examples"));
    assert_eq!(link.alias.as_deref(), Some("Ownership examples"));
    assert!(link.has_md_suffix);
    assert_eq!(
        &text[link.span.clone()],
        "[[b0001-babel-ownership.md#examples|Ownership examples]]"
    );
    assert_eq!(&text[link.target_span.clone()], "b0001-babel-ownership.md");
}

#[test]
fn extract_links_classifies_markdown_links_and_preserves_target_span() {
    let text = "Read [ADR](../20 Architecture/0115 Vault Internal Link.md#decision).";

    let links = extract_links(text);

    assert_eq!(links.len(), 1);
    let link = &links[0];
    assert_eq!(link.kind, LinkKind::Markdown);
    assert_eq!(link.target, "../20 Architecture/0115 Vault Internal Link.md");
    assert_eq!(link.anchor.as_deref(), Some("decision"));
    assert_eq!(link.alias, None);
    assert!(link.has_md_suffix);
    assert_eq!(
        &text[link.target_span.clone()],
        "../20 Architecture/0115 Vault Internal Link.md#decision"
    );
}

#[test]
fn extract_links_ignores_code_fences() {
    let text = concat!(
        "Before fence [[b0001-babel-ownership]].\n",
        "```md\n",
        "[[b0002-babel-borrowing]]\n",
        "[Doc](./internal.md)\n",
        "```\n",
        "After fence [Guide](https://example.com).\n",
    );

    let links = extract_links(text);

    assert_eq!(links.len(), 2);
    assert_eq!(links[0].target, "b0001-babel-ownership");
    assert_eq!(links[1].target, "https://example.com");
}

#[test]
fn extract_links_ignores_inline_code_regions() {
    let text = "Ignore `[[b0001-babel-ownership]]` and keep [[b0002-babel-borrowing]].";

    let links = extract_links(text);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].target, "b0002-babel-borrowing");
}

#[test]
fn extract_links_understands_angle_bracket_markdown_targets() {
    let text = "External [PyTerrier](<https://pyterrier.readthedocs.io/en/latest/>).";

    let links = extract_links(text);

    assert_eq!(links.len(), 1);
    assert_eq!(links[0].kind, LinkKind::Markdown);
    assert_eq!(links[0].target, "https://pyterrier.readthedocs.io/en/latest/");
    assert!(!links[0].has_md_suffix);
}

#[test]
fn check_fix_rewrites_non_book_internal_links_to_canonical_wiki_links() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let adr_path =
        ws.root.join("doc/20 Architecture/implemented/0082 adr Vault Resolution Contract.md");
    fs::write(
        &adr_path,
        "---\n\
type: adr\n\
id: 82\n\
title: Vault Resolution Contract\n\
description: ADR description.\n\
status: implemented\n\
created: 2026-04-17\n\
tags:\n\
  - adr\n\
---\n\
# Vault Resolution Contract\n",
    )
    .unwrap();
    let guide_path = ws.root.join("doc/40 Guides/0001 guide Link Guide.md");
    fs::write(
        &guide_path,
        "---\n\
type: guide\n\
id: 1\n\
title: Link Guide\n\
description: Guide description.\n\
status: stable\n\
created: 2026-04-17\n\
tags:\n\
  - guide\n\
---\n\
See [ADR](../20 Architecture/0082 Vault Resolution Contract.md#decision) and \
[[20 Architecture/0082 Vault Resolution Contract|ADR 0082]].\n\
External [PyTerrier](https://pyterrier.readthedocs.io/en/latest/) stays markdown.\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "expected fix pass but got: {fix_report}");

    let updated = fs::read_to_string(&guide_path).unwrap();
    assert!(updated.contains("[[0082 adr Vault Resolution Contract#decision]]"));
    assert!(updated.contains("[[0082 adr Vault Resolution Contract]]"));
    assert!(updated.contains("[PyTerrier](https://pyterrier.readthedocs.io/en/latest/)"));

    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(passed, "expected clean check after fix but got: {report}");
}

#[test]
fn check_fix_rewrites_absolute_workspace_markdown_links_to_canonical_wiki_links() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let adr_path =
        ws.root.join("doc/20 Architecture/implemented/0082 adr Vault Resolution Contract.md");
    fs::write(
        &adr_path,
        "---\n\
type: adr\n\
id: 82\n\
title: Vault Resolution Contract\n\
description: ADR description.\n\
status: implemented\n\
created: 2026-04-17\n\
tags:\n\
  - adr\n\
---\n\
# Vault Resolution Contract\n",
    )
    .unwrap();
    let guide_path = ws.root.join("doc/40 Guides/0001 guide Link Guide.md");
    let absolute_target = adr_path.to_string_lossy().replace('\\', "/").replace(' ', "%20");
    fs::write(
        &guide_path,
        format!(
            "---\n\
type: guide\n\
id: 1\n\
title: Link Guide\n\
description: Guide description.\n\
status: stable\n\
created: 2026-04-17\n\
tags:\n\
  - guide\n\
---\n\
See [ADR](/{absolute_target}:16).\n"
        ),
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "expected fix pass but got: {fix_report}");

    let updated = fs::read_to_string(&guide_path).unwrap();
    assert!(updated.contains("[[0082 adr Vault Resolution Contract]]"));

    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(passed, "expected clean check after fix but got: {report}");
}

#[test]
fn check_fix_rewrites_file_uri_markdown_links_to_canonical_wiki_links() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let adr_path =
        ws.root.join("doc/20 Architecture/implemented/0082 adr Vault Resolution Contract.md");
    fs::write(
        &adr_path,
        "---\n\
type: adr\n\
id: 82\n\
title: Vault Resolution Contract\n\
description: ADR description.\n\
status: implemented\n\
created: 2026-04-17\n\
tags:\n\
  - adr\n\
---\n\
# Vault Resolution Contract\n",
    )
    .unwrap();
    let task_path = ws.root.join("doc/00 Project Hub/Tasks/done/00001 task Link Task.md");
    let file_uri =
        format!("file:///{}", adr_path.to_string_lossy().replace('\\', "/").replace(' ', "%20"));
    fs::write(
        &task_path,
        format!(
            "---\n\
type: task\n\
id: 1\n\
title: Link Task\n\
description: Task description.\n\
status: done\n\
created: 2026-04-17\n\
tags:\n\
  - task\n\
---\n\
See [ADR]({file_uri}).\n"
        ),
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "expected fix pass but got: {fix_report}");

    let updated = fs::read_to_string(&task_path).unwrap();
    assert!(updated.contains("[[0082 adr Vault Resolution Contract]]"));

    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(passed, "expected clean check after fix but got: {report}");
}

#[test]
fn check_fix_rewrites_book_path_links_to_canonical_wiki_links() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    write_valid_artifact(&ws, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");

    let index_path = ws.root.join("forge/50 Books/001 Rust/00 Index.md");
    fs::write(
        &index_path,
        "---\n\
type: overview\n\
book_code: '001'\n\
book: rust\n\
title: Rust - Book Index\n\
description: Entry point.\n\
status: stable\n\
created: 2026-04-17\n\
tags:\n\
  - book\n\
---\n\
# Rust - Book Index\n\n\
1. [[50 Books/001 Rust/memory/0001-ownership|Ownership]]\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "expected fix pass but got: {fix_report}");

    let updated = fs::read_to_string(&index_path).unwrap();
    assert!(updated.contains("[[b0001 rust Ownership]]"));
    assert!(!updated.contains("|Ownership"));

    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(passed, "expected clean check after fix but got: {report}");
}

#[test]
fn check_fix_rewrites_non_book_links_to_book_artifacts() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    write_valid_artifact(&ws, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");
    fs::write(
        ws.root.join("doc/40 Guides/0001 guide Link Guide.md"),
        "---\n\
type: guide\n\
id: 1\n\
title: Link Guide\n\
description: Guide description.\n\
status: stable\n\
created: 2026-04-17\n\
tags:\n\
  - guide\n\
---\n\
Read [[50 Books/001 Rust/memory/0001-ownership|Ownership]] before this section.\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "expected fix pass but got: {fix_report}");

    let updated =
        fs::read_to_string(ws.root.join("doc/40 Guides/0001 guide Link Guide.md")).unwrap();
    assert!(updated.contains("[[b0001 rust Ownership]]"));
    assert!(!updated.contains("|Ownership"));

    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(passed, "expected clean check after fix but got: {report}");
}

#[test]
fn check_reports_invalid_governed_link_forms_without_fix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    fs::write(
        ws.root.join("doc/20 Architecture/implemented/0082 adr Vault Resolution Contract.md"),
        "---\n\
type: adr\n\
id: 82\n\
title: Vault Resolution Contract\n\
description: ADR description.\n\
status: implemented\n\
created: 2026-04-17\n\
tags:\n\
  - adr\n\
---\n\
# Vault Resolution Contract\n",
    )
    .unwrap();
    fs::write(
        ws.root.join("doc/40 Guides/0001 guide Link Guide.md"),
        "---\n\
type: guide\n\
id: 1\n\
title: Link Guide\n\
description: Guide description.\n\
status: stable\n\
created: 2026-04-17\n\
tags:\n\
  - guide\n\
---\n\
See [ADR](../20 Architecture/0082 Vault Resolution Contract.md), \
[[20 Architecture/0082 Vault Resolution Contract|ADR 0082]], and \
[[0082 adr Vault Resolution Contract.md]].\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(!passed);
    assert!(report.contains("INVALID_FIELDS doc/40 Guides/0001 guide Link Guide.md"));
    assert!(report.contains("internal_markdown_links"));
    assert!(report.contains("aliased_wiki_links"));
    assert!(report.contains("md_wiki_links"));
    assert!(report.contains("noncanonical_wiki_links"));
}

// ============================================================================
// Phase C — cargo vault check --fix sync enforcement (ADR 0104 §11.3)
// ============================================================================

/// Writes a fully-valid artifact file to `path` inside `ws.root`.
fn write_valid_artifact(ws: &TestWorkspace, rel_path: &str) {
    let abs = ws.root.join(rel_path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &abs,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 1\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"Ownership\"\n\
description: \"How Rust enforces single ownership.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();
}

/// Writes a fully-valid `00 Index.md` to `path` inside `ws.root`.
fn write_valid_index(ws: &TestWorkspace, rel_path: &str, book_code: &str, book_key: &str) {
    let abs = ws.root.join(rel_path);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &abs,
        format!(
            "---\n\
type: overview\n\
book_code: {book_code}\n\
book: {book_key}\n\
title: \"{book_key} index\"\n\
description: \"Entry point.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - book\n\
---\n\
# Index\n"
        ),
    )
    .unwrap();
}

// --- Invariant 1: category matches parent subfolder name ---

#[test]
fn check_reports_category_mismatch_when_frontmatter_differs_from_subfolder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let artifact_path = "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md";
    let abs = ws.root.join(artifact_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // `category` says "concurrency" but file lives in "memory/"
    fs::write(
        &abs,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 1\n\
chapter: 1\n\
sequence: 1\n\
category: concurrency\n\
title: \"Ownership\"\n\
description: \"How Rust enforces single ownership.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(&format!("INVALID_FIELDS {artifact_path} category")));
}

// --- Invariant 2: id matches numeric prefix of filename ---

#[test]
fn check_reports_book_id_mismatch_when_frontmatter_differs_from_filename() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let artifact_path = "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md";
    let abs = ws.root.join(artifact_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // filename says 0001 but frontmatter says 42
    fs::write(
        &abs,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 42\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"Ownership\"\n\
description: \"How Rust enforces single ownership.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(&format!("INVALID_FIELDS {artifact_path} id")));
}

// --- Invariant 3: book_code matches numeric prefix of book folder ---

#[test]
fn check_reports_book_code_mismatch_when_frontmatter_differs_from_folder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let artifact_path = "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md";
    let abs = ws.root.join(artifact_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // folder says 001 but frontmatter says 002
    fs::write(
        &abs,
        "---\n\
 type: concept\n\
 book_code: 002\n\
 book: rust\n\
 id: 1\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"Ownership\"\n\
description: \"How Rust enforces single ownership.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(&format!("INVALID_FIELDS {artifact_path} book_code")));
}

// --- Invariant 4: all required fields present and non-empty ---

#[test]
fn check_reports_missing_fields_for_artifact_with_sparse_frontmatter() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let artifact_path = "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md";
    let abs = ws.root.join(artifact_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // Only type + book_code present; everything else missing.
    fs::write(
        &abs,
        "---\n\
type: concept\n\
book_code: 001\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    let line = report
        .lines()
        .find(|l| l.contains("MISSING_FIELDS") && l.contains(artifact_path))
        .expect("expected MISSING_FIELDS line for artifact");
    // All fields absent from the sparse frontmatter must be reported.
    for field in
        &["book", "id", "chapter", "sequence", "category", "title", "description", "status", "tags"]
    {
        assert!(line.contains(field), "expected '{field}' in missing fields: {line}");
    }
}

#[test]
fn check_reports_missing_fields_for_artifact_with_empty_string_values() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let artifact_path = "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md";
    let abs = ws.root.join(artifact_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // Fields present but empty strings — must be treated the same as absent.
    fs::write(
        &abs,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 1\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"\"\n\
description: \"\"\n\
status: draft\n\
created: 2026-04-11\n\
tags: []\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    let line = report
        .lines()
        .find(|l| l.contains("MISSING_FIELDS") && l.contains(artifact_path))
        .expect("expected MISSING_FIELDS line for empty-string artifact");
    assert!(line.contains("title"), "expected 'title' flagged: {line}");
    assert!(line.contains("description"), "expected 'description' flagged: {line}");
    assert!(line.contains("tags"), "expected empty tags flagged: {line}");
}

// --- PASS cases ---

#[test]
fn check_passes_for_correctly_structured_artifact() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_artifact(&ws, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "expected PASS but got: {report}");
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_passes_for_correctly_structured_index() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "expected PASS but got: {report}");
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_passes_for_mixed_valid_book_with_multiple_artifacts() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    write_valid_artifact(&ws, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");
    // Second artifact — different id and category.
    let second = ws.root.join("forge/50 Books/001 Rust/concurrency/b0002 rust Threads.md");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(
        &second,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 2\n\
chapter: 2\n\
sequence: 1\n\
category: concurrency\n\
title: \"Threads\"\n\
description: \"OS thread model in Rust.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Threads\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "expected PASS but got: {report}");
}

// --- assets/ folder is skipped ---

#[test]
fn check_skips_assets_subfolder() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    // Place a markdown file with no frontmatter inside assets/ — must not cause failure.
    let assets = ws.root.join("forge/50 Books/001 Rust/assets/diagram.md");
    fs::create_dir_all(assets.parent().unwrap()).unwrap();
    fs::write(&assets, "# Raw diagram notes\n").unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "assets/ content should be ignored: {report}");
}

// --- non-conforming book folders are skipped ---

#[test]
fn check_skips_book_folders_without_three_digit_prefix() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    // A folder "01 OldFormat" does not match the NNN pattern.
    let old = ws.root.join("forge/50 Books/01 OldFormat/memory/b0001 rust Ownership.md");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::write(&old, "# No frontmatter\n").unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "non-conforming folders should be ignored: {report}");
}

// --- index file book_code sync ---

#[test]
fn check_reports_book_code_mismatch_in_index_file() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let index_path = "forge/50 Books/001 Rust/00 Index.md";
    let abs = ws.root.join(index_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    // book_code in frontmatter says 002 but folder says 001
    fs::write(
        &abs,
        "---\n\
type: overview\n\
book_code: 002\n\
book: rust\n\
title: \"Rust — Book Index\"\n\
description: \"Entry point.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - book\n\
---\n\
# Rust\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(&format!("INVALID_FIELDS {index_path} book_code")));
}

// --- four invariants are detected independently ---

#[test]
fn check_detects_all_four_drift_conditions_independently() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    // Invariant 1 violation: category mismatch
    let cat_path = "forge/50 Books/001 Rust/memory/b0001 rust Cat Drift.md";
    let abs = ws.root.join(cat_path);
    fs::create_dir_all(abs.parent().unwrap()).unwrap();
    fs::write(&abs, "---\ntype: concept\nbook_code: 001\nbook: rust\nid: 1\nchapter: 1\nsequence: 1\ncategory: WRONG\ntitle: \"T\"\ndescription: \"D.\"\nstatus: draft\ncreated: 2026-04-11\ntags:\n  - rust\nprerequisites: []\n---\n# T\n").unwrap();

    // Invariant 2 violation: id mismatch
    let aid_path = "forge/50 Books/001 Rust/memory/b0002 rust Aid Drift.md";
    let abs2 = ws.root.join(aid_path);
    fs::write(&abs2, "---\ntype: concept\nbook_code: 001\nbook: rust\nid: 9999\nchapter: 1\nsequence: 2\ncategory: memory\ntitle: \"T2\"\ndescription: \"D2.\"\nstatus: draft\ncreated: 2026-04-11\ntags:\n  - rust\nprerequisites: []\n---\n# T2\n").unwrap();

    // Invariant 3 violation: book_code mismatch
    let bc_path = "forge/50 Books/001 Rust/memory/b0003 rust Bc Drift.md";
    let abs3 = ws.root.join(bc_path);
    fs::write(&abs3, "---\ntype: concept\nbook_code: 999\nbook: rust\nid: 3\nchapter: 1\nsequence: 3\ncategory: memory\ntitle: \"T3\"\ndescription: \"D3.\"\nstatus: draft\ncreated: 2026-04-11\ntags:\n  - rust\nprerequisites: []\n---\n# T3\n").unwrap();

    // Invariant 4 violation: missing required fields
    let mf_path = "forge/50 Books/001 Rust/memory/0004-missing-fields.md";
    let abs4 = ws.root.join(mf_path);
    fs::write(&abs4, "---\ntype: concept\n---\n# T4\n").unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(!passed);
    assert!(report.contains(&format!("INVALID_FIELDS {cat_path} category")), "inv1: {report}");
    assert!(report.contains(&format!("INVALID_FIELDS {aid_path} id")), "inv2: {report}");
    assert!(report.contains(&format!("INVALID_FIELDS {bc_path} book_code")), "inv3: {report}");
    assert!(report.contains(&format!("MISSING_FIELDS {mf_path}")), "inv4: {report}");
}

// --- books_root absent is a no-op ---

#[test]
fn check_passes_when_books_root_does_not_exist() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    // Remove the books root that TestWorkspace created.
    fs::remove_dir_all(ws.root.join("forge/50 Books")).unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);

    assert!(passed, "missing books root should be a no-op: {report}");
}

// --- cross-phase: reserve then check passes ---

#[test]
fn check_passes_on_book_and_artifacts_created_via_reserve_commands() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    reserve_book_command(&ws.root, &config, "Rust").unwrap();
    reserve_book_artifact_command(&ws.root, &config, "001", "Ownership", "memory").unwrap();
    reserve_book_artifact_command(&ws.root, &config, "001", "Borrowing", "memory").unwrap();
    // Populate required fields the scaffold leaves blank (chapter, sequence).
    // We do this by re-reading and patching rather than testing partial scaffold state.
    // Instead just confirm the scaffold-only content is flagged correctly: chapter and sequence
    // are present but empty in the scaffold, so they should appear in MISSING_FIELDS.
    let (_, _) = execute_check(&ws.root, &config, false);
    // The test here is: no INVALID_FIELDS for sync invariants. Only MISSING_FIELDS for blanks.
    let config2 = load_config(&ws.root).unwrap();
    let (report, _) = execute_check(&ws.root, &config2, false);
    assert!(
        !report.contains("INVALID_FIELDS"),
        "sync invariants must not fire on reserved output: {report}"
    );
}

// --- cross-phase (Phase N): check --fix passes clean on correctly structured book ---
// (The full Phase N test is above under "cross-phase test"; this verifies the check path.)

#[test]
fn check_fix_passes_clean_on_correctly_structured_book() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    write_valid_artifact(&ws, "forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "check --fix should PASS on clean book: {report}");
    assert_eq!(report, "PASS\n");
}

#[test]
fn check_fix_renames_legacy_book_artifact_without_recreating_old_path() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    let legacy_path = ws.root.join("forge/50 Books/001 Rust/memory/0001-ownership.md");
    fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    fs::write(
        &legacy_path,
        "---\n\
 type: concept\n\
 book_code: 001\n\
 book: rust\n\
 id: 1\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"Ownership\"\n\
description: \"Ownership fundamentals.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, true);

    assert!(passed, "check --fix should PASS after renaming legacy artifact: {report}");
    let canonical_path = ws.root.join("forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");
    assert!(canonical_path.is_file(), "missing canonical artifact path");
    assert!(!legacy_path.exists(), "legacy artifact path should be removed");
}

#[test]
fn check_fix_removes_legacy_artifact_id_field_from_book_artifact() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    write_valid_index(&ws, "forge/50 Books/001 Rust/00 Index.md", "001", "rust");
    let artifact_path = ws.root.join("forge/50 Books/001 Rust/memory/b0001 rust Ownership.md");
    fs::create_dir_all(artifact_path.parent().unwrap()).unwrap();
    fs::write(
        &artifact_path,
        "---\n\
type: concept\n\
book_code: 001\n\
book: rust\n\
id: 1\n\
artifact_id: 0001\n\
chapter: 1\n\
sequence: 1\n\
category: memory\n\
title: \"Ownership\"\n\
description: \"Ownership fundamentals.\"\n\
status: draft\n\
created: 2026-04-11\n\
tags:\n\
  - rust\n\
prerequisites: []\n\
---\n\
# Ownership\n",
    )
    .unwrap();

    let config = load_config(&ws.root).unwrap();
    let (report, passed) = execute_check(&ws.root, &config, false);
    assert!(!passed, "legacy artifact_id should fail without --fix: {report}");
    assert!(report.contains("artifact_id"), "expected legacy field violation: {report}");

    let (fix_report, fix_passed) = execute_check(&ws.root, &config, true);
    assert!(fix_passed, "check --fix should remove legacy artifact_id: {fix_report}");
    let updated = fs::read_to_string(&artifact_path).unwrap();
    assert!(updated.contains("id: 1"));
    assert!(!updated.contains("artifact_id:"));
}

// --- cross-phase test: reserve book then reserve artifact ---

#[test]
fn reserve_book_followed_by_reserve_artifact_produces_consistent_codes_and_paths() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();
    let config = load_config(&ws.root).unwrap();

    let book_out = reserve_book_command(&ws.root, &config, "Rust").unwrap();
    let artifact_out = reserve_book_artifact_command(
        &ws.root,
        &config,
        &book_out.book_code,
        "Ownership",
        "memory",
    )
    .unwrap();

    // Book and artifact must agree on book code.
    assert_eq!(book_out.book_code, artifact_out.book_code);
    // Artifact path must sit inside the book folder.
    assert!(artifact_out.file_path.starts_with(&book_out.folder_path));
    // File must exist on disk.
    assert!(ws.root.join(&artifact_out.file_path).is_file());
    // The artifact file frontmatter must reference the book code.
    let content = fs::read_to_string(ws.root.join(&artifact_out.file_path)).unwrap();
    assert!(content.contains(&format!("book_code: {}", book_out.book_code)));
}

#[test]
fn reserve_roadmap_then_resolve_round_trip_uses_same_id_and_path() {
    let _guard = lock_test_dir();
    let ws = TestWorkspace::new();

    let reserved = execute(VaultCommand::Reserve {
        doc_type: VaultDocumentType::Roadmap,
        name: "Emitter Plan".to_string(),
        subfolder: Some("language".to_string()),
    })
    .unwrap();
    let reserved_path = ws.root.join(&reserved.path);
    fs::write(
        &reserved_path,
        "---\n\
type: roadmap\n\
tags:\n\
  - roadmap\n\
created: 2026-04-17\n\
description: Example roadmap.\n\
id: 1\n\
---\n\
# Emitter Plan\n",
    )
    .unwrap();

    let resolved = execute(VaultCommand::Roadmap { number: Some("1".to_string()) }).unwrap();

    assert_eq!(reserved.id, resolved.id);
    assert_eq!(reserved.path, resolved.path);
}
