//! Vault command: resolve and reserve numbered documents, validate frontmatter (`check`, including
//! normalized `id` from the filename when applicable), and query by frontmatter expression (`query`, ADR 0085).

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

use chrono::Utc;
use clap::{Subcommand, ValueEnum};
use serde::Deserialize;
use serde_yaml::{Mapping, Value};

use crate::vault_query::{QueryExpr, eval_query_expr, parse_query_expression};

const CONFIG_PATH: &str = ".cargo/.xtask/vault.yaml";
const NAMED_QUERY_PATH: &str = ".cargo/.xtask/vault-query.yaml";
const REQUIRED_FIELDS_PREFIX: &str = "MISSING_FIELDS";
const INVALID_FIELDS_PREFIX: &str = "INVALID_FIELDS";
const WRONG_LOCATION_PREFIX: &str = "WRONG_LOCATION";
const BOOK_CODE_WIDTH: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum VaultDocumentType {
    Task,
    Research,
    Roadmap,
    Adr,
    Guide,
    Rule,
}

impl VaultDocumentType {
    #[must_use]
    const fn as_key(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Research => "research",
            Self::Roadmap => "roadmap",
            Self::Adr => "adr",
            Self::Guide => "guide",
            Self::Rule => "rule",
        }
    }

    #[must_use]
    const fn number_width(self) -> usize {
        match self {
            Self::Task => 5,
            Self::Research | Self::Roadmap | Self::Adr | Self::Guide | Self::Rule => 4,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum VaultCommand {
    /// Resolve a task by numeric prefix
    Task {
        /// Numeric task id, padded or unpadded. When omitted, returns task root metadata.
        number: Option<String>,
    },
    /// Resolve a research note by numeric prefix
    Research {
        /// Numeric research id, padded or unpadded. When omitted, returns research root metadata.
        number: Option<String>,
    },
    /// Resolve a roadmap by numeric prefix
    Roadmap {
        /// Numeric roadmap id, padded or unpadded. When omitted, returns roadmap root metadata.
        number: Option<String>,
    },
    /// Resolve an ADR by numeric prefix
    Adr {
        /// Numeric ADR id, padded or unpadded. When omitted, returns ADR root metadata.
        number: Option<String>,
    },
    /// Resolve a guide by numeric prefix
    Guide {
        /// Numeric guide id, padded or unpadded. When omitted, returns guide root metadata.
        number: Option<String>,
    },
    /// Resolve a rule by numeric prefix
    Rule {
        /// Numeric rule id, padded or unpadded. When omitted, returns rule root metadata.
        number: Option<String>,
    },
    /// Reserve the next id for a vault category
    Reserve {
        /// Category to reserve
        doc_type: VaultDocumentType,
        /// Final page name to use in `<id> <name>.md`
        name: String,
        /// Required when the category root contains subfolders and the file must live under one
        #[arg(long)]
        subfolder: Option<String>,
    },
    /// Reserve the next three-digit book code and scaffold the book folder + 00 Index.md (ADR 0104)
    ReserveBook {
        /// Human-readable book name used in the folder and index frontmatter (e.g. "Rust")
        name: String,
    },
    /// Reserve the next four-digit artifact id for a book and scaffold the artifact file (ADR 0104)
    ReserveBookArtifact {
        /// Three-digit book code (e.g. "001")
        book_code: String,
        /// Human-readable artifact name used in the filename (e.g. "Ownership and Move Semantics")
        name: String,
        /// Category subfolder inside the book folder (e.g. "memory"). Required.
        #[arg(long)]
        category: String,
    },
    /// Validate vault frontmatter across governed categories
    Check {
        /// Apply deterministic structural fixes
        #[arg(long)]
        fix: bool,
    },
    /// Filter governed markdown by frontmatter expression (ADR 0085)
    Query {
        /// Brace groups, `field='value'`, `field=['a','b']`, `field={'a','b'}`, `field@='substring'`. Quote as one shell argument.
        #[arg(value_name = "EXPRESSION", conflicts_with = "query_id")]
        expression: Option<String>,
        /// Use the named expression from `.cargo/.xtask/vault-query.yaml` (mutually exclusive with positional `EXPRESSION`).
        #[arg(long = "query-id", value_name = "NAME", conflicts_with = "expression")]
        query_id: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
pub struct VaultConfig {
    documents: BTreeMap<String, VaultDocumentRoot>,
}

#[derive(Debug, Deserialize)]
struct VaultDocumentRoot {
    root: String,
}

#[derive(Clone, Copy)]
enum StatusRule {
    None,
    Task,
    Adr,
}

#[derive(Clone, Copy)]
struct CheckCategory<'a> {
    root: &'a str,
    expected_type: &'a str,
    status_rule: StatusRule,
}

#[derive(Clone, Copy)]
struct FilenameContract {
    id_width: usize,
}

type QueryFinding = (String, String);
type CheckReport = (String, bool);

struct LinkNormalizationContext {
    workspace: PathBuf,
    entries: Vec<GovernedLinkTarget>,
    canonical_stems: BTreeMap<String, usize>,
    legacy_stems: BTreeMap<String, Vec<usize>>,
    non_book_by_root_and_id: BTreeMap<(String, u32), usize>,
    book_by_root_and_id: BTreeMap<(String, u32), usize>,
    category_roots: Vec<CategoryRootInfo>,
}

struct CategoryRootInfo {
    root_without_doc: String,
    status_rule: StatusRule,
}

struct GovernedLinkTarget {
    canonical_stem: String,
    expected_type: Option<String>,
}

struct LinkNormalizationOutput {
    updated: String,
    invalid_fields: Vec<String>,
}

#[derive(Default)]
struct CheckResult {
    missing_fields: Vec<String>,
    invalid_fields: Vec<String>,
    wrong_location: bool,
    modified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DocumentMatch {
    id: u32,
    path: PathBuf,
}

#[derive(Debug)]
struct VaultOutput {
    doc_type: VaultDocumentType,
    id: Option<String>,
    last_id: Option<String>,
    path: String,
    subfolders: Vec<String>,
    reserved_name: Option<String>,
}

/// Output produced by `reserve-book`: distinct from `VaultOutput` because books have a
/// different shape (book code, index path, no `VaultDocumentType` enum variant).
#[derive(Debug)]
struct BookReserveOutput {
    book_code: String,
    folder_path: String,
    index_path: String,
    reserved_name: String,
}

/// Output produced by `reserve-book-artifact` (ADR 0104 §2).
#[derive(Debug)]
struct BookArtifactReserveOutput {
    id: String,
    book_code: String,
    file_path: String,
    reserved_name: String,
}

pub fn run(command: VaultCommand) -> i32 {
    if let VaultCommand::Check { fix } = &command {
        let workspace = match env::current_dir() {
            Ok(workspace) => workspace,
            Err(e) => {
                println!("ERROR=could not determine workspace root: {e}");
                return 1;
            }
        };
        let config = match load_config(&workspace) {
            Ok(config) => config,
            Err(message) => {
                println!("ERROR={message}");
                return 1;
            }
        };
        let (report, passed) = execute_check(&workspace, &config, *fix);
        print!("{report}");
        return i32::from(!passed);
    }

    if let VaultCommand::Query { expression, query_id } = &command {
        return run_query(expression.as_deref(), query_id.as_deref());
    }

    if let VaultCommand::ReserveBook { name } = &command {
        let workspace = match env::current_dir() {
            Ok(workspace) => workspace,
            Err(e) => {
                println!("ERROR=could not determine workspace root: {e}");
                return 1;
            }
        };
        let config = match load_config(&workspace) {
            Ok(config) => config,
            Err(message) => {
                println!("ERROR={message}");
                return 1;
            }
        };
        return match reserve_book_command(&workspace, &config, name) {
            Ok(output) => {
                print!("{}", output.render());
                0
            }
            Err(message) => {
                println!("ERROR={message}");
                1
            }
        };
    }

    if let VaultCommand::ReserveBookArtifact { book_code, name, category } = &command {
        let workspace = match env::current_dir() {
            Ok(workspace) => workspace,
            Err(e) => {
                println!("ERROR=could not determine workspace root: {e}");
                return 1;
            }
        };
        let config = match load_config(&workspace) {
            Ok(config) => config,
            Err(message) => {
                println!("ERROR={message}");
                return 1;
            }
        };
        return match reserve_book_artifact_command(&workspace, &config, book_code, name, category) {
            Ok(output) => {
                print!("{}", output.render());
                0
            }
            Err(message) => {
                println!("ERROR={message}");
                1
            }
        };
    }

    match execute(command) {
        Ok(output) => {
            print!("{}", output.render());
            0
        }
        Err(message) => {
            println!("ERROR={message}");
            1
        }
    }
}

fn execute(command: VaultCommand) -> Result<VaultOutput, String> {
    let workspace =
        env::current_dir().map_err(|e| format!("could not determine workspace root: {e}"))?;
    let config = load_config(&workspace)?;

    match command {
        VaultCommand::Task { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Task,
            number.as_deref(),
        ),
        VaultCommand::Research { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Research,
            number.as_deref(),
        ),
        VaultCommand::Roadmap { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Roadmap,
            number.as_deref(),
        ),
        VaultCommand::Adr { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Adr,
            number.as_deref(),
        ),
        VaultCommand::Guide { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Guide,
            number.as_deref(),
        ),
        VaultCommand::Rule { number } => resolve_or_describe_category(
            &workspace,
            &config,
            VaultDocumentType::Rule,
            number.as_deref(),
        ),
        VaultCommand::Reserve { doc_type, name, subfolder } => {
            reserve_command(&workspace, &config, doc_type, &name, subfolder.as_deref())
        }
        VaultCommand::Check { .. } => unreachable!("check is handled directly in run()"),
        VaultCommand::Query { .. } => unreachable!("query is handled directly in run()"),
        VaultCommand::ReserveBook { .. } => {
            unreachable!("reserve-book is handled directly in run()")
        }
        VaultCommand::ReserveBookArtifact { .. } => {
            unreachable!("reserve-book-artifact is handled directly in run()")
        }
    }
}

/// ADR 0085 `cargo vault query`: parse expression, walk governed trees like `check`, print `<type>,<path>` per match on stdout.
///
/// Inline `expression` and `query_id` must not both be set (`clap` enforces this for the CLI; this function still checks for direct callers).
/// When `query_id` is set, the expression text is loaded only from [`NAMED_QUERY_PATH`].
#[must_use]
pub fn run_query(expression: Option<&str>, query_id: Option<&str>) -> i32 {
    let inline = expression.map(str::trim).filter(|s| !s.is_empty());
    let qid = query_id.map(str::trim).filter(|s| !s.is_empty());

    let raw: String = match (inline, qid) {
        (Some(_), Some(_)) => {
            eprintln!("ERROR=cannot use both inline EXPRESSION and --query-id");
            return 1;
        }
        (_, Some(id)) => match load_named_query_from_workspace(id) {
            Ok(s) => s,
            Err(message) => {
                eprintln!("ERROR={message}");
                return 1;
            }
        },
        (Some(expr), None) => expr.to_string(),
        (None, None) => return 0,
    };

    let expr = match parse_query_expression(&raw) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ERROR={} (byte {})", e.message, e.offset);
            return 1;
        }
    };
    let workspace = match env::current_dir() {
        Ok(workspace) => workspace,
        Err(e) => {
            eprintln!("ERROR=could not determine workspace root: {e}");
            return 1;
        }
    };
    let config = match load_config(&workspace) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("ERROR={message}");
            return 1;
        }
    };
    match collect_query_findings(&workspace, &config, &expr) {
        Ok(rows) => {
            for (type_key, rel_path) in rows {
                println!("{type_key},{rel_path}");
            }
            0
        }
        Err(msg) => {
            eprintln!("ERROR={msg}");
            1
        }
    }
}

/// Runs ADR 0085 query evaluation over the vault; used by tests and [`run_query`].
pub fn collect_query_findings(
    workspace: &Path,
    config: &VaultConfig,
    expr: &QueryExpr,
) -> Result<Vec<QueryFinding>, String> {
    let mut matches = Vec::new();
    for category in check_categories(config) {
        let root = workspace.join(category.root);
        walk_query_md_files(workspace, &root, category.expected_type, expr, &mut matches)?;
    }
    Ok(matches)
}

fn walk_query_md_files(
    workspace: &Path,
    dir: &Path,
    category_key: &str,
    expr: &QueryExpr,
    matches: &mut Vec<QueryFinding>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not enumerate {}: {e}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("could not read file type in {}: {e}", dir.display()))?;
        if file_type.is_dir() {
            walk_query_md_files(workspace, &path, category_key, expr, matches)?;
        } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
            let mapping = load_mapping_for_query(&path)?;
            if eval_query_expr(expr, &mapping) {
                matches.push((category_key.to_string(), display_relative(workspace, &path)));
            }
        }
    }
    Ok(())
}

/// YAML frontmatter as [`Mapping`] for query evaluation. No `---` block → empty mapping.
fn load_mapping_for_query(path: &Path) -> Result<Mapping, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let Some(fm) = extract_frontmatter(&raw) else {
        return Ok(Mapping::new());
    };
    serde_yaml::from_str(fm.raw)
        .map_err(|e| format!("could not parse frontmatter in {}: {e}", path.display()))
}

fn resolve_or_describe_category(
    workspace: &Path,
    config: &VaultConfig,
    doc_type: VaultDocumentType,
    number: Option<&str>,
) -> Result<VaultOutput, String> {
    if let Some(number) = number {
        resolve_command(workspace, config, doc_type, number)
    } else {
        category_metadata_command(workspace, config, doc_type)
    }
}

fn load_config(workspace: &Path) -> Result<VaultConfig, String> {
    let config_path = workspace.join(CONFIG_PATH);
    let raw = fs::read_to_string(&config_path).map_err(|e| {
        format!("could not read {}: {e}", display_relative(workspace, &config_path))
    })?;
    serde_yaml::from_str::<VaultConfig>(&raw)
        .map_err(|e| format!("could not parse {}: {e}", display_relative(workspace, &config_path)))
}

fn load_named_query_from_workspace(id: &str) -> Result<String, String> {
    let workspace =
        env::current_dir().map_err(|e| format!("could not determine workspace root: {e}"))?;
    let path = workspace.join(NAMED_QUERY_PATH);
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("could not read {}: {e}", display_relative(&workspace, &path)))?;
    let map: BTreeMap<String, String> = serde_yaml::from_str(&raw)
        .map_err(|e| format!("could not parse {}: {e}", display_relative(&workspace, &path)))?;
    map.get(id).cloned().ok_or_else(|| {
        format!("named query '{id}' not found in {}", display_relative(&workspace, &path))
    })
}

fn execute_check(workspace: &Path, config: &VaultConfig, fix: bool) -> CheckReport {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut lines = Vec::new();
    let link_context = build_link_normalization_context(workspace, config);

    for category in check_categories(config) {
        let root = workspace.join(category.root);
        let _ = collect_check_findings(
            workspace,
            &root,
            &root,
            category,
            &link_context,
            fix,
            &today,
            &mut lines,
        );
    }

    // Book sync enforcement (ADR 0104 §11.3): walk forge/50 Books/ independently.
    if let Some(books_root_str) = config.documents.get("book").map(|d| d.root.as_str()) {
        let books_root = workspace.join(books_root_str);
        collect_book_check_findings(workspace, &books_root, &link_context, fix, &mut lines);
    }

    if lines.is_empty() {
        ("PASS\n".to_string(), true)
    } else {
        (format!("{}\n", lines.join("\n")), false)
    }
}

fn check_categories(config: &VaultConfig) -> Vec<CheckCategory<'_>> {
    let mut categories = Vec::new();
    for (key, doc) in &config.documents {
        let status_rule = match key.as_str() {
            "task" => StatusRule::Task,
            "adr" => StatusRule::Adr,
            _ => StatusRule::None,
        };
        let expected_type = match key.as_str() {
            "task" | "adr" | "guide" | "rule" | "research" | "roadmap" | "sample" => key.as_str(),
            _ => continue,
        };
        categories.push(CheckCategory { root: &doc.root, expected_type, status_rule });
    }
    categories
}

fn build_link_normalization_context(
    workspace: &Path,
    config: &VaultConfig,
) -> LinkNormalizationContext {
    let mut context = LinkNormalizationContext {
        workspace: workspace.to_path_buf(),
        entries: Vec::new(),
        canonical_stems: BTreeMap::new(),
        legacy_stems: BTreeMap::new(),
        non_book_by_root_and_id: BTreeMap::new(),
        book_by_root_and_id: BTreeMap::new(),
        category_roots: Vec::new(),
    };

    for category in check_categories(config) {
        if filename_contract(category).is_none() {
            continue;
        }
        context.category_roots.push(CategoryRootInfo {
            root_without_doc: strip_doc_prefix(category.root).to_string(),
            status_rule: category.status_rule,
        });
        collect_non_book_link_targets(
            workspace,
            &workspace.join(category.root),
            category,
            &mut context,
        );
    }

    if let Some(books_root_str) = config.documents.get("book").map(|d| d.root.as_str()) {
        collect_book_link_targets(workspace, &workspace.join(books_root_str), &mut context);
    }

    context
}

fn collect_non_book_link_targets(
    workspace: &Path,
    dir: &Path,
    category: CheckCategory<'_>,
    context: &mut LinkNormalizationContext,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_dir() {
            collect_non_book_link_targets(workspace, &path, category, context);
            continue;
        }
        if !file_type.is_file() || path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        let Some(canonical_filename) = canonical_non_book_filename_from_disk(&path, category)
        else {
            continue;
        };
        let Some(id) = parse_numeric_prefix(&path) else {
            continue;
        };
        let canonical_stem = canonical_filename.trim_end_matches(".md").to_string();
        let legacy_stem = descriptive_name_from_filename(&path, category)
            .map(|title| format!("{id:0width$} {title}", width = category_number_width(category)));
        let index = context.entries.len();
        let _ = path;
        context.entries.push(GovernedLinkTarget {
            canonical_stem: canonical_stem.clone(),
            expected_type: Some(category.expected_type.to_string()),
        });
        context.canonical_stems.insert(canonical_stem, index);
        if let Some(legacy_stem) = legacy_stem {
            insert_unique_alias(&mut context.legacy_stems, legacy_stem, index);
        }
        context
            .non_book_by_root_and_id
            .insert((strip_doc_prefix(category.root).to_string(), id), index);
    }
    let _ = workspace;
}

fn collect_book_link_targets(
    workspace: &Path,
    books_root: &Path,
    context: &mut LinkNormalizationContext,
) {
    let Ok(entries) = fs::read_dir(books_root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(book_code) = parse_book_code_prefix(&entry.file_name().to_string_lossy()) else {
            continue;
        };
        let padded = format!("{book_code:0>BOOK_CODE_WIDTH$}");
        let book_key = read_book_key_from_index(&path)
            .filter(|key| is_valid_book_key(key))
            .unwrap_or_else(|| derive_book_key_from_folder_name(&path));
        collect_book_link_targets_in_dir(workspace, &path, &padded, &book_key, context);
    }
}

fn collect_book_link_targets_in_dir(
    workspace: &Path,
    dir: &Path,
    expected_book_code: &str,
    expected_book_key: &str,
    context: &mut LinkNormalizationContext,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_dir() {
            if entry.file_name() == OsStr::new("assets") {
                continue;
            }
            collect_book_link_targets_in_dir(
                workspace,
                &path,
                expected_book_code,
                expected_book_key,
                context,
            );
            continue;
        }
        if !file_type.is_file() || path.extension() != Some(OsStr::new("md")) {
            continue;
        }
        if matches!(path.file_name().and_then(OsStr::to_str), Some("00 Index.md" | "00 Notes.md")) {
            continue;
        }
        let Some(raw) = fs::read_to_string(&path).ok() else { continue };
        let mapping = extract_frontmatter(&raw)
            .and_then(|fm| serde_yaml::from_str::<Mapping>(fm.raw).ok())
            .unwrap_or_default();
        let Some(id) = mapping
            .get(Value::String("id".to_string()))
            .and_then(parse_frontmatter_id)
            .or_else(|| parse_artifact_id_prefix(&path))
        else {
            continue;
        };
        let Some(title) = get_nonempty_string_field(&mapping, "title").or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.splitn(3, ' ').nth(2))
                .map(str::to_string)
        }) else {
            continue;
        };
        let canonical_stem = canonical_book_artifact_filename(id, expected_book_key, &title)
            .trim_end_matches(".md")
            .to_string();
        let Some(book_root) = find_book_root_relative(workspace, &path) else {
            continue;
        };
        let index = context.entries.len();
        context.entries.push(GovernedLinkTarget {
            canonical_stem: canonical_stem.clone(),
            expected_type: None,
        });
        context.canonical_stems.insert(canonical_stem, index);
        context.book_by_root_and_id.insert((book_root, id), index);
    }
    let _ = expected_book_code;
}

fn find_book_root_relative(workspace: &Path, path: &Path) -> Option<String> {
    let relative = display_relative(workspace, path).replace('\\', "/");
    let mut parts = relative.split('/');
    let first = parts.next()?;
    let second = parts.next()?;
    let third = parts.next()?;
    match (first, second) {
        ("forge" | "doc", "50 Books") => Some(format!("50 Books/{third}")),
        _ => None,
    }
}

fn insert_unique_alias(map: &mut BTreeMap<String, Vec<usize>>, key: String, index: usize) {
    let entry = map.entry(key).or_default();
    if !entry.contains(&index) {
        entry.push(index);
    }
}

fn category_number_width(category: CheckCategory<'_>) -> usize {
    filename_contract(category).map_or(4, |contract| contract.id_width)
}

fn canonical_non_book_filename_from_disk(
    path: &Path,
    category: CheckCategory<'_>,
) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let mapping = extract_frontmatter(&raw)
        .and_then(|fm| serde_yaml::from_str::<Mapping>(fm.raw).ok())
        .unwrap_or_default();
    canonical_non_book_filename(path, category, &mapping)
}

fn strip_doc_prefix(value: &str) -> &str {
    value.strip_prefix("doc/").unwrap_or(value)
}

#[allow(clippy::too_many_arguments)]
fn collect_check_findings(
    workspace: &Path,
    dir: &Path,
    category_root: &Path,
    category: CheckCategory<'_>,
    link_context: &LinkNormalizationContext,
    fix: bool,
    today: &str,
    lines: &mut Vec<String>,
) -> Result<(), String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) => {
            lines.push(format!(
                "{INVALID_FIELDS_PREFIX} {} frontmatter",
                display_relative(workspace, dir)
            ));
            return Err(format!("could not read {}: {e}", dir.display()));
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                lines.push(format!(
                    "{INVALID_FIELDS_PREFIX} {} frontmatter",
                    display_relative(workspace, dir)
                ));
                return Err(format!("could not enumerate {}: {e}", dir.display()));
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(e) => {
                lines.push(format!(
                    "{INVALID_FIELDS_PREFIX} {} frontmatter",
                    display_relative(workspace, &path)
                ));
                return Err(format!("could not read file type in {}: {e}", dir.display()));
            }
        };
        if file_type.is_dir() {
            let _ = collect_check_findings(
                workspace,
                &path,
                category_root,
                category,
                link_context,
                fix,
                today,
                lines,
            );
        } else if file_type.is_file() && path.extension() == Some(OsStr::new("md")) {
            match validate_document(&path, category_root, category, link_context, fix, today) {
                Ok(result) => {
                    let relative = display_relative(workspace, &path);
                    if !result.missing_fields.is_empty() {
                        lines.push(format!(
                            "{REQUIRED_FIELDS_PREFIX} {relative} {}",
                            result.missing_fields.join(" ")
                        ));
                    }
                    if !result.invalid_fields.is_empty() {
                        lines.push(format!(
                            "{INVALID_FIELDS_PREFIX} {relative} {}",
                            result.invalid_fields.join(" ")
                        ));
                    }
                    if result.wrong_location {
                        lines.push(format!("{WRONG_LOCATION_PREFIX} {relative}"));
                    }
                }
                Err(_) => {
                    lines.push(format!(
                        "{INVALID_FIELDS_PREFIX} {} frontmatter",
                        display_relative(workspace, &path)
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_document(
    path: &Path,
    category_root: &Path,
    category: CheckCategory<'_>,
    link_context: &LinkNormalizationContext,
    fix: bool,
    today: &str,
) -> Result<CheckResult, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let Some(frontmatter) = extract_frontmatter(&raw) else {
        let target = ValidationTarget { path, category_root, original: &raw, body: raw.as_str() };
        return validate_and_maybe_write(
            target,
            Mapping::new(),
            category,
            link_context,
            fix,
            today,
        );
    };

    let mapping: Mapping = serde_yaml::from_str(frontmatter.raw)
        .map_err(|e| format!("could not parse frontmatter in {}: {e}", path.display()))?;
    let target = ValidationTarget { path, category_root, original: &raw, body: frontmatter.body };
    validate_and_maybe_write(target, mapping, category, link_context, fix, today)
}

fn validate_and_maybe_write(
    target: ValidationTarget<'_>,
    mut mapping: Mapping,
    category: CheckCategory<'_>,
    link_context: &LinkNormalizationContext,
    fix: bool,
    today: &str,
) -> Result<CheckResult, String> {
    let mut result = CheckResult::default();

    validate_type(&mut mapping, category.expected_type, fix, &mut result);
    validate_required_scalar(&mut mapping, "created", fix, today, &mut result);
    validate_presence(&mapping, "description", &mut result);
    validate_presence(&mapping, "tags", &mut result);
    validate_status(&mut mapping, category.status_rule, fix, &mut result);
    validate_id(&mut mapping, target.path, fix, &mut result);
    ensure_non_book_title_metadata(&mut mapping, target.path, category, fix, &mut result);

    let relocation =
        expected_status_location(target.path, target.category_root, category.status_rule, &mapping);

    let mut updated_raw = target.original.to_string();
    if fix && result.modified {
        updated_raw = render_document_with_frontmatter(&mapping, target.body).map_err(|e| {
            format!("could not serialize frontmatter for {}: {e}", target.path.display())
        })?;
    }

    let link_output = normalize_governed_links(&updated_raw, target.path, link_context, fix);
    updated_raw = link_output.updated;
    result.invalid_fields.extend(link_output.invalid_fields);
    result.invalid_fields.sort();
    result.invalid_fields.dedup();

    if fix && updated_raw != target.original {
        fs::write(target.path, updated_raw)
            .map_err(|e| format!("could not write {}: {e}", target.path.display()))?;
    }

    let mut current_path = target.path.to_path_buf();
    if let Some(target_path) = relocation {
        if fix {
            move_document_to_status_path(target.path, &target_path, &mut result)?;
            current_path = target_path;
        } else {
            result.wrong_location = true;
        }
    }

    if fix
        && let Some(target_path) =
            expected_non_book_filename_path(&current_path, target.category_root, category, &mapping)
        && target_path != current_path
    {
        move_document_to_status_path(&current_path, &target_path, &mut result)?;
    }

    Ok(result)
}

fn expected_status_location(
    path: &Path,
    category_root: &Path,
    rule: StatusRule,
    mapping: &Mapping,
) -> Option<PathBuf> {
    let status_folder = status_folder_name(rule, mapping)?;
    let filename = path.file_name()?;
    let expected_parent = category_root.join(status_folder);
    let current_parent = path.parent()?;
    if current_parent == expected_parent { None } else { Some(expected_parent.join(filename)) }
}

fn status_folder_name(rule: StatusRule, mapping: &Mapping) -> Option<&str> {
    let allowed = match rule {
        StatusRule::None => return None,
        StatusRule::Task => &["todo", "in-progress", "done"][..],
        StatusRule::Adr => &["draft", "execution", "implemented", "retired"][..],
    };
    let status = mapping.get(Value::String("status".to_string()))?;
    let Value::String(status) = status else {
        return None;
    };
    allowed.iter().copied().find(|candidate| candidate == status)
}

fn move_document_to_status_path(
    current_path: &Path,
    target_path: &Path,
    result: &mut CheckResult,
) -> Result<(), String> {
    if target_path == current_path {
        return Ok(());
    }
    if target_path.exists() {
        result.invalid_fields.push("location".to_string());
        return Ok(());
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("could not create directory {}: {e}", parent.display()))?;
    }
    fs::rename(current_path, target_path).map_err(|e| {
        format!("could not move {} to {}: {e}", current_path.display(), target_path.display())
    })?;
    result.modified = true;
    Ok(())
}

fn validate_type(mapping: &mut Mapping, expected_type: &str, fix: bool, result: &mut CheckResult) {
    match mapping.get(Value::String("type".to_string())) {
        None => {
            if fix {
                mapping.insert(
                    Value::String("type".to_string()),
                    Value::String(expected_type.to_string()),
                );
                result.modified = true;
            } else {
                result.missing_fields.push("type".to_string());
            }
        }
        Some(Value::String(actual)) if actual == expected_type => {}
        Some(_) => result.invalid_fields.push("type".to_string()),
    }
}

fn validate_required_scalar(
    mapping: &mut Mapping,
    field: &str,
    fix: bool,
    default_value: &str,
    result: &mut CheckResult,
) {
    let key = Value::String(field.to_string());
    if mapping.contains_key(&key) {
        return;
    }
    if fix {
        mapping.insert(key, Value::String(default_value.to_string()));
        result.modified = true;
    } else {
        result.missing_fields.push(field.to_string());
    }
}

fn validate_presence(mapping: &Mapping, field: &str, result: &mut CheckResult) {
    if !mapping.contains_key(Value::String(field.to_string())) {
        result.missing_fields.push(field.to_string());
    }
}

fn validate_status(mapping: &mut Mapping, rule: StatusRule, fix: bool, result: &mut CheckResult) {
    let (allowed, default_value) = match rule {
        StatusRule::None => return,
        StatusRule::Task => (&["todo", "in-progress", "done"][..], "todo"),
        StatusRule::Adr => (&["draft", "execution", "implemented", "retired"][..], "draft"),
    };

    match mapping.get(Value::String("status".to_string())) {
        None => {
            if fix {
                mapping.insert(
                    Value::String("status".to_string()),
                    Value::String(default_value.to_string()),
                );
                result.modified = true;
            } else {
                result.missing_fields.push("status".to_string());
            }
        }
        Some(Value::String(actual)) if allowed.iter().any(|value| value == actual) => {}
        Some(_) => result.invalid_fields.push("status".to_string()),
    }
}

/// When the filename has a `<digits> <rest>.md` prefix, frontmatter must carry the same id as a
/// YAML number or decimal string **without leading zeros** (e.g. `00023` in the name → `id: 23`).
/// Files whose stem has no parseable numeric prefix skip this rule.
fn validate_id(mapping: &mut Mapping, path: &Path, fix: bool, result: &mut CheckResult) {
    let Some(file_id) = parse_numeric_prefix(path) else {
        return;
    };

    let key = Value::String("id".to_string());
    match mapping.get(&key) {
        None => {
            if fix {
                mapping.insert(key, Value::Number(serde_yaml::Number::from(file_id)));
                result.modified = true;
            } else {
                result.missing_fields.push("id".to_string());
            }
        }
        Some(value) => match parse_frontmatter_id(value) {
            Some(parsed) if parsed == file_id => {}
            _ => result.invalid_fields.push("id".to_string()),
        },
    }
}

fn ensure_non_book_title_metadata(
    mapping: &mut Mapping,
    path: &Path,
    category: CheckCategory<'_>,
    fix: bool,
    result: &mut CheckResult,
) {
    if !fix || filename_contract(category).is_none() {
        return;
    }
    let key = Value::String("title".to_string());
    let missing_or_empty = match mapping.get(&key) {
        None => true,
        Some(Value::String(value)) => value.trim().is_empty(),
        Some(_) => false,
    };
    if !missing_or_empty {
        return;
    }
    if let Some(title) = descriptive_name_from_filename(path, category) {
        mapping.insert(key, Value::String(title.to_string()));
        result.modified = true;
    }
}

fn parse_frontmatter_id(value: &Value) -> Option<u32> {
    match value {
        Value::Number(n) => n
            .as_u64()
            .and_then(|u| u32::try_from(u).ok())
            .or_else(|| n.as_i64().and_then(|i| u32::try_from(i).ok())),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

struct Frontmatter<'a> {
    raw: &'a str,
    body: &'a str,
}

struct ValidationTarget<'a> {
    path: &'a Path,
    category_root: &'a Path,
    original: &'a str,
    body: &'a str,
}

fn filename_contract(category: CheckCategory<'_>) -> Option<FilenameContract> {
    let width = match category.expected_type {
        "task" => VaultDocumentType::Task.number_width(),
        "adr" => VaultDocumentType::Adr.number_width(),
        "guide" => VaultDocumentType::Guide.number_width(),
        "rule" => VaultDocumentType::Rule.number_width(),
        "research" => VaultDocumentType::Research.number_width(),
        "roadmap" => VaultDocumentType::Roadmap.number_width(),
        _ => return None,
    };
    Some(FilenameContract { id_width: width })
}

fn expected_non_book_filename_path(
    current_path: &Path,
    category_root: &Path,
    category: CheckCategory<'_>,
    mapping: &Mapping,
) -> Option<PathBuf> {
    let filename = canonical_non_book_filename(current_path, category, mapping)?;
    let status_parent =
        expected_status_location(current_path, category_root, category.status_rule, mapping);
    let parent = status_parent
        .as_deref()
        .and_then(Path::parent)
        .unwrap_or_else(|| current_path.parent().unwrap_or(category_root));
    Some(parent.join(filename))
}

fn canonical_non_book_filename(
    path: &Path,
    category: CheckCategory<'_>,
    mapping: &Mapping,
) -> Option<String> {
    let contract = filename_contract(category)?;
    let id = parse_frontmatter_id(mapping.get(Value::String("id".to_string()))?)
        .or_else(|| parse_numeric_prefix(path))?;
    let title = get_nonempty_string_field(mapping, "title")
        .or_else(|| descriptive_name_from_filename(path, category).map(str::to_string))?;
    if !is_safe_filename_component(&title) {
        return None;
    }
    let padded_id = format!("{id:0width$}", width = contract.id_width);
    Some(format!("{padded_id} {} {title}.md", category.expected_type))
}

fn get_nonempty_string_field(mapping: &Mapping, field: &str) -> Option<String> {
    match mapping.get(Value::String(field.to_string()))? {
        Value::String(value) if !value.trim().is_empty() => Some(value.trim().to_string()),
        _ => None,
    }
}

fn render_document_with_frontmatter(
    mapping: &Mapping,
    body: &str,
) -> Result<String, serde_yaml::Error> {
    let yaml = serde_yaml::to_string(mapping)?;
    let serialized = if yaml.ends_with('\n') { yaml } else { format!("{yaml}\n") };
    Ok(format!("---\n{serialized}---\n{body}"))
}

fn descriptive_name_from_filename<'a>(
    path: &'a Path,
    category: CheckCategory<'_>,
) -> Option<&'a str> {
    let stem = path.file_stem()?.to_str()?;
    let (_, rest) = stem.split_once(' ')?;
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let title = rest
        .strip_prefix(category.expected_type)
        .and_then(|suffix| suffix.strip_prefix(' '))
        .unwrap_or(rest)
        .trim();
    if title.is_empty() { None } else { Some(title) }
}

fn is_safe_filename_component(value: &str) -> bool {
    !value.is_empty()
        && !value.contains(['<', '>', ':', '"', '/', '\\', '|', '?', '*'])
        && !value.ends_with([' ', '.'])
}

/// Validates a name supplied to any `reserve` subcommand for OS filesystem compatibility.
///
/// Returns `Ok(())` when the name is safe to embed in a filename on all supported operating
/// systems (Windows NTFS, macOS HFS+, Linux ext4). Returns `Err` with a human-readable
/// diagnostic otherwise.
///
/// Checked constraints (union of Windows + Unix rules so vault files remain portable):
/// - Must not be empty.
/// - Must not contain characters illegal on Windows: `< > : " / \ | ? *`
/// - Must not end with a space or a dot (Windows silently strips these).
/// - Must not be a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`,
///   `COM1`–`COM9`, `LPT1`–`LPT9`), case-insensitively.
fn validate_reserve_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("name must not be empty".to_string());
    }

    const ILLEGAL: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if let Some(ch) = name.chars().find(|c| ILLEGAL.contains(c)) {
        return Err(format!(
            "name contains the character '{ch}' which is not allowed in filenames on Windows; \
             remove or replace it"
        ));
    }

    if name.ends_with([' ', '.']) {
        return Err(
            "name must not end with a space or a dot — Windows silently strips these characters \
             from filenames"
                .to_string(),
        );
    }

    // Windows reserved device names are illegal as filename stems on NTFS.
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let upper = name.to_uppercase();
    if RESERVED.contains(&upper.as_str()) {
        return Err(format!(
            "'{name}' is a reserved device name on Windows and cannot be used as a filename"
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LinkKind {
    Markdown,
    Wiki,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExtractedLink {
    kind: LinkKind,
    span: Range<usize>,
    target_span: Range<usize>,
    target: String,
    anchor: Option<String>,
    alias: Option<String>,
    has_md_suffix: bool,
}

/// Extract governed-doc candidate links from Markdown without paying the cost of a full parser.
///
/// The extractor intentionally supports only the subset needed by ADR 0115 Phase 0:
/// wiki links, markdown links, anchors, aliases, `.md` suffix detection, and skipping obvious
/// no-rewrite regions such as fenced code blocks and inline code spans.
#[must_use]
fn extract_links(text: &str) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    let mut offset = 0usize;
    let mut in_fenced_code = false;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if is_fence_line(trimmed) {
            in_fenced_code = !in_fenced_code;
            offset += line.len();
            continue;
        }
        if in_fenced_code {
            offset += line.len();
            continue;
        }
        links.extend(extract_links_from_line(line, offset));
        offset += line.len();
    }

    links
}

#[must_use]
fn is_fence_line(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

#[must_use]
fn extract_links_from_line(line: &str, line_offset: usize) -> Vec<ExtractedLink> {
    let mut links = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut in_inline_code = false;

    while index < bytes.len() {
        match bytes[index] {
            b'`' => {
                in_inline_code = !in_inline_code;
                index += 1;
            }
            b'[' if !in_inline_code => {
                if index + 1 < bytes.len()
                    && bytes[index + 1] == b'['
                    && let Some(link) = parse_wiki_link(line, line_offset, index)
                {
                    index = link.span.end - line_offset;
                    links.push(link);
                    continue;
                }
                if (index == 0 || bytes[index.saturating_sub(1)] != b'!')
                    && let Some(link) = parse_markdown_link(line, line_offset, index)
                {
                    index = link.span.end - line_offset;
                    links.push(link);
                    continue;
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    links
}

fn parse_wiki_link(line: &str, line_offset: usize, start: usize) -> Option<ExtractedLink> {
    let rest = &line[start + 2..];
    let relative_end = rest.find("]]")?;
    let end = start + 2 + relative_end + 2;
    let content_start = start + 2;
    let content_end = end - 2;
    let content = &line[content_start..content_end];

    let (target_with_anchor, alias) = content
        .split_once('|')
        .map_or((content, None), |(target, alias)| (target, Some(alias.to_string())));
    let (target, anchor) = target_with_anchor
        .split_once('#')
        .map_or((target_with_anchor.to_string(), None), |(target, anchor)| {
            (target.to_string(), Some(anchor.to_string()))
        });

    Some(ExtractedLink {
        kind: LinkKind::Wiki,
        span: line_offset + start..line_offset + end,
        target_span: line_offset + content_start..line_offset + content_start + target.len(),
        has_md_suffix: target.ends_with(".md"),
        target,
        anchor,
        alias,
    })
}

fn parse_markdown_link(line: &str, line_offset: usize, start: usize) -> Option<ExtractedLink> {
    let label_end = find_matching_bracket(line, start + 1, b'[', b']')?;
    if line.as_bytes().get(label_end + 1) != Some(&b'(') {
        return None;
    }
    let target_end = find_matching_paren(line, label_end + 2)?;
    let raw_target_segment = &line[label_end + 2..target_end];
    let (target_text, target_range) = extract_markdown_target(raw_target_segment)?;
    let (target, anchor) =
        target_text.split_once('#').map_or((target_text.to_string(), None), |(target, anchor)| {
            (target.to_string(), Some(anchor.to_string()))
        });

    Some(ExtractedLink {
        kind: LinkKind::Markdown,
        span: line_offset + start..line_offset + target_end + 1,
        target_span: line_offset + label_end + 2 + target_range.start
            ..line_offset + label_end + 2 + target_range.end,
        has_md_suffix: target.ends_with(".md"),
        target,
        anchor,
        alias: None,
    })
}

fn find_matching_bracket(text: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = start;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            value if value == open => {
                depth += 1;
                index += 1;
            }
            value if value == close => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

fn find_matching_paren(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = start;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

fn normalize_governed_links(
    raw: &str,
    source_path: &Path,
    context: &LinkNormalizationContext,
    fix: bool,
) -> LinkNormalizationOutput {
    let mut replacements = Vec::new();
    let mut invalid_fields = BTreeSet::new();

    for link in extract_links(raw) {
        let Some(resolution) = resolve_governed_link(&link, source_path, context) else {
            continue;
        };

        if resolution.ambiguous {
            invalid_fields.insert("ambiguous_governed_links".to_string());
            continue;
        }

        if resolution.is_canonical {
            continue;
        }

        if fix && !resolution.replacement.is_empty() {
            replacements.push((link.span.clone(), resolution.replacement));
            continue;
        }

        for field in resolution.invalid_fields {
            invalid_fields.insert(field);
        }
    }

    let mut updated = raw.to_string();
    for (span, replacement) in replacements.into_iter().rev() {
        updated.replace_range(span, &replacement);
    }

    LinkNormalizationOutput { updated, invalid_fields: invalid_fields.into_iter().collect() }
}

struct ResolvedGovernedLink {
    replacement: String,
    invalid_fields: Vec<String>,
    is_canonical: bool,
    ambiguous: bool,
}

fn resolve_governed_link(
    link: &ExtractedLink,
    source_path: &Path,
    context: &LinkNormalizationContext,
) -> Option<ResolvedGovernedLink> {
    match link.kind {
        LinkKind::Markdown => resolve_markdown_governed_link(link, source_path, context),
        LinkKind::Wiki => resolve_wiki_governed_link(link, source_path, context),
    }
}

fn resolve_markdown_governed_link(
    link: &ExtractedLink,
    source_path: &Path,
    context: &LinkNormalizationContext,
) -> Option<ResolvedGovernedLink> {
    if is_external_markdown_target(&link.target) || link.target.starts_with('#') {
        return None;
    }
    let entry = resolve_governed_path_target(&link.target, source_path, context)?;
    let replacement = render_canonical_wiki_link(&context.entries[entry], link.anchor.as_deref());
    Some(ResolvedGovernedLink {
        replacement,
        invalid_fields: vec!["internal_markdown_links".to_string()],
        is_canonical: false,
        ambiguous: false,
    })
}

fn resolve_wiki_governed_link(
    link: &ExtractedLink,
    source_path: &Path,
    context: &LinkNormalizationContext,
) -> Option<ResolvedGovernedLink> {
    let target = link.target.trim();
    let resolution = resolve_governed_wiki_target(target, source_path, context)?;
    if resolution.ambiguous {
        return Some(ResolvedGovernedLink {
            replacement: String::new(),
            invalid_fields: Vec::new(),
            is_canonical: false,
            ambiguous: true,
        });
    }
    let canonical_entry = &context.entries[resolution.index?];
    let replacement = render_canonical_wiki_link(canonical_entry, link.anchor.as_deref());
    let mut invalid_fields = Vec::new();
    if link.alias.is_some() {
        invalid_fields.push("aliased_wiki_links".to_string());
    }
    if link.has_md_suffix {
        invalid_fields.push("md_wiki_links".to_string());
    }
    if target != canonical_entry.canonical_stem {
        invalid_fields.push("noncanonical_wiki_links".to_string());
    }
    Some(ResolvedGovernedLink {
        replacement,
        is_canonical: invalid_fields.is_empty(),
        invalid_fields,
        ambiguous: false,
    })
}

struct WikiTargetResolution {
    index: Option<usize>,
    ambiguous: bool,
}

fn resolve_governed_wiki_target(
    target: &str,
    _source_path: &Path,
    context: &LinkNormalizationContext,
) -> Option<WikiTargetResolution> {
    if let Some(index) = context.canonical_stems.get(target).copied() {
        return Some(WikiTargetResolution { index: Some(index), ambiguous: false });
    }
    let stripped = target.trim_end_matches(".md");
    if let Some(index) = context.canonical_stems.get(stripped).copied() {
        return Some(WikiTargetResolution { index: Some(index), ambiguous: false });
    }
    if let Some(found) = context.legacy_stems.get(stripped) {
        let preferred = disambiguate_legacy_candidates(found, context);
        return Some(WikiTargetResolution { index: preferred, ambiguous: preferred.is_none() });
    }
    resolve_vault_relative_governed_path_target(stripped, context)
        .map(|index| WikiTargetResolution { index: Some(index), ambiguous: false })
}

fn disambiguate_legacy_candidates(
    candidates: &[usize],
    context: &LinkNormalizationContext,
) -> Option<usize> {
    if candidates.len() == 1 {
        return candidates.first().copied();
    }
    let mut preferred = candidates
        .iter()
        .copied()
        .filter(|index| context.entries[*index].expected_type.as_deref() == Some("adr"));
    let first = preferred.next()?;
    if preferred.next().is_none() { Some(first) } else { None }
}

fn resolve_governed_path_target(
    target: &str,
    source_path: &Path,
    context: &LinkNormalizationContext,
) -> Option<usize> {
    let normalized = normalize_target_path(source_path, target, &context.workspace)?;
    if let Some(index) = resolve_non_book_target_by_path(&normalized, context) {
        return Some(index);
    }
    resolve_book_target_by_path(&normalized, context)
}

fn resolve_vault_relative_governed_path_target(
    target: &str,
    context: &LinkNormalizationContext,
) -> Option<usize> {
    let normalized = normalize_vault_relative_target(target);
    if let Some(index) = resolve_non_book_target_by_path(&normalized, context) {
        return Some(index);
    }
    resolve_book_target_by_path(&normalized, context)
}

fn normalize_target_path(source_path: &Path, target: &str, workspace: &Path) -> Option<String> {
    let target = normalize_local_markdown_target(target);
    let target_path = if target.starts_with("doc/") || target.starts_with("forge/") {
        workspace.join(&target)
    } else if is_windows_absolute_target(&target) {
        PathBuf::from(&target)
    } else if target.starts_with('/') {
        let trimmed = target.trim_start_matches('/');
        if is_windows_absolute_target(trimmed) {
            PathBuf::from(trimmed)
        } else {
            workspace.join(trimmed)
        }
    } else {
        source_path.parent().unwrap_or(workspace).join(target)
    };
    let normalized = normalize_path(&target_path);
    let relative = relative_to_workspace_path(&normalized, workspace)?;
    Some(relative.trim_end_matches(".md").to_string())
}

fn normalize_vault_relative_target(target: &str) -> String {
    target
        .replace('\\', "/")
        .trim_start_matches("doc/")
        .trim_start_matches("forge/")
        .trim_end_matches(".md")
        .to_string()
}

fn relative_to_workspace_path(path: &Path, workspace: &Path) -> Option<String> {
    let path_text = path.to_string_lossy().replace('\\', "/");
    let workspace_text = workspace.to_string_lossy().replace('\\', "/");
    if let Some(relative) = path_text.strip_prefix(&workspace_text) {
        return Some(relative.trim_start_matches('/').to_string());
    }
    if path_text.len() >= workspace_text.len()
        && path_text[..workspace_text.len()].eq_ignore_ascii_case(&workspace_text)
    {
        return Some(path_text[workspace_text.len()..].trim_start_matches('/').to_string());
    }
    None
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn resolve_non_book_target_by_path(
    target: &str,
    context: &LinkNormalizationContext,
) -> Option<usize> {
    let target = target.trim_start_matches("doc/").trim_end_matches(".md");
    for root in &context.category_roots {
        let prefix = format!("{}/", root.root_without_doc);
        if !target.starts_with(&prefix) {
            continue;
        }
        let remainder = target.strip_prefix(&prefix)?;
        let remainder = strip_optional_status_prefix(remainder, root.status_rule);
        let filename = remainder.rsplit('/').next()?;
        let id = parse_numeric_prefix(Path::new(filename))?;
        if let Some(index) =
            context.non_book_by_root_and_id.get(&(root.root_without_doc.clone(), id))
        {
            return Some(*index);
        }
    }
    None
}

fn strip_optional_status_prefix(value: &str, rule: StatusRule) -> &str {
    match rule {
        StatusRule::Task => value
            .strip_prefix("todo/")
            .or_else(|| value.strip_prefix("in-progress/"))
            .or_else(|| value.strip_prefix("done/"))
            .unwrap_or(value),
        StatusRule::Adr => value
            .strip_prefix("draft/")
            .or_else(|| value.strip_prefix("execution/"))
            .or_else(|| value.strip_prefix("implemented/"))
            .or_else(|| value.strip_prefix("retired/"))
            .unwrap_or(value),
        StatusRule::None => value,
    }
}

fn resolve_book_target_by_path(target: &str, context: &LinkNormalizationContext) -> Option<usize> {
    let target =
        target.trim_start_matches("doc/").trim_start_matches("forge/").trim_end_matches(".md");
    let parts: Vec<&str> = target.split('/').collect();
    if parts.len() < 4 || parts.first().copied() != Some("50 Books") {
        return None;
    }
    let book_root = format!("{}/{}", parts[0], parts[1]);
    let file_name = parts.last().copied()?;
    let artifact_id = parse_artifact_id_prefix(Path::new(file_name))?;
    context.book_by_root_and_id.get(&(book_root, artifact_id)).copied()
}

fn render_canonical_wiki_link(target: &GovernedLinkTarget, anchor: Option<&str>) -> String {
    match anchor.filter(|value| !value.trim().is_empty()) {
        Some(anchor) => format!("[[{}#{anchor}]]", target.canonical_stem),
        None => format!("[[{}]]", target.canonical_stem),
    }
}

fn is_external_markdown_target(target: &str) -> bool {
    let target = target.trim();
    target.starts_with("http://") || target.starts_with("https://")
}

fn normalize_local_markdown_target(target: &str) -> String {
    let normalized = target.trim().replace('\\', "/");
    let stripped = strip_file_uri_prefix(&normalized).unwrap_or(&normalized);
    let decoded = decode_percent_encoding(stripped);
    strip_obsidian_line_suffix(&decoded).to_string()
}

fn strip_file_uri_prefix(target: &str) -> Option<&str> {
    target.strip_prefix("file:///").or_else(|| target.strip_prefix("file://"))
}

fn strip_obsidian_line_suffix(target: &str) -> &str {
    let Some(md_index) = target.rfind(".md") else {
        return target;
    };
    let suffix = &target[md_index + 3..];
    if let Some(stripped) = suffix.strip_prefix(':')
        && !stripped.is_empty()
        && stripped
            .split(':')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
    {
        return &target[..md_index + 3];
    }
    target
}

fn is_windows_absolute_target(target: &str) -> bool {
    let bytes = target.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

fn decode_percent_encoding(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = String::with_capacity(value.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push(char::from(high * 16 + low));
            index += 3;
            continue;
        }
        decoded.push(char::from(bytes[index]));
        index += 1;
    }
    decoded
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

type MarkdownTarget<'a> = (&'a str, Range<usize>);

#[must_use]
fn extract_markdown_target(target_raw: &str) -> Option<MarkdownTarget<'_>> {
    let trimmed = target_raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('<')
        && let Some(end) = trimmed.find('>')
    {
        let start_in_raw = target_raw.find('<').unwrap_or_default() + 1;
        let end_in_raw = start_in_raw + end - 1;
        return Some((&trimmed[1..end], start_in_raw..end_in_raw));
    }
    let target = trimmed;
    let start_in_raw = target_raw.find(target).unwrap_or_default();
    Some((target, start_in_raw..start_in_raw + target.len()))
}

fn extract_frontmatter(text: &str) -> Option<Frontmatter<'_>> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if trim_line_ending(first) != "---" {
        return None;
    }

    let mut consumed = first.len();
    let mut frontmatter_len = 0usize;
    for line in lines {
        if trim_line_ending(line) == "---" {
            let raw = &text[first.len()..first.len() + frontmatter_len];
            let body = &text[consumed + line.len()..];
            return Some(Frontmatter { raw, body });
        }
        frontmatter_len += line.len();
        consumed += line.len();
    }

    None
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn resolve_command(
    workspace: &Path,
    config: &VaultConfig,
    doc_type: VaultDocumentType,
    number: &str,
) -> Result<VaultOutput, String> {
    let target_id = parse_requested_id(number)?;
    let root = configured_root(workspace, config, doc_type)?;
    let subfolders = list_direct_subfolders(&root)?;
    let candidates = find_matching_documents(&root, target_id)?;
    match candidates.as_slice() {
        [] => Err(format!("no {} document found for id {}", doc_type.as_key(), number)),
        [matched] => Ok(VaultOutput {
            doc_type,
            id: Some(format!("{:0width$}", matched.id, width = doc_type.number_width())),
            last_id: None,
            path: display_relative(workspace, &matched.path),
            subfolders,
            reserved_name: None,
        }),
        _ => Err(format!("ambiguous {} id {} matched multiple files", doc_type.as_key(), number)),
    }
}

fn reserve_command(
    workspace: &Path,
    config: &VaultConfig,
    doc_type: VaultDocumentType,
    name: &str,
    subfolder: Option<&str>,
) -> Result<VaultOutput, String> {
    validate_reserve_name(name)?;
    let root = configured_root(workspace, config, doc_type)?;
    let subfolders = list_direct_subfolders(&root)?;
    let target_parent = reserve_parent_dir(doc_type, &root, &subfolders, subfolder)?;
    let next_id = find_max_numeric_prefix(&root)?.saturating_add(1);
    let padded_id = format!("{:0width$}", next_id, width = doc_type.number_width());
    let filename = format!("{padded_id} {name}.md");
    let path = target_parent.join(filename);
    Ok(VaultOutput {
        doc_type,
        id: Some(padded_id),
        last_id: None,
        path: display_relative(workspace, &path),
        subfolders,
        reserved_name: Some(name.to_string()),
    })
}

fn category_metadata_command(
    workspace: &Path,
    config: &VaultConfig,
    doc_type: VaultDocumentType,
) -> Result<VaultOutput, String> {
    let root = configured_root(workspace, config, doc_type)?;
    let subfolders = list_direct_subfolders(&root)?;
    let last_id = find_max_numeric_prefix(&root)?;
    Ok(VaultOutput {
        doc_type,
        id: None,
        last_id: Some(format!("{:0width$}", last_id, width = doc_type.number_width())),
        path: display_relative(workspace, &root),
        subfolders,
        reserved_name: None,
    })
}

fn reserve_parent_dir(
    doc_type: VaultDocumentType,
    root: &Path,
    subfolders: &[String],
    subfolder: Option<&str>,
) -> Result<PathBuf, String> {
    let default_subfolder = default_reserve_subfolder(doc_type);
    match (subfolders.is_empty(), subfolder) {
        (true, None) => Ok(root.to_path_buf()),
        (true, Some(requested)) => Err(format!(
            "subfolder '{requested}' was provided, but this category root has no subfolders"
        )),
        (false, None) => {
            let requested = default_subfolder.ok_or_else(|| {
                "this category has subfolders; pass --subfolder <name> to choose the target directory"
                    .to_string()
            })?;
            if subfolders.iter().any(|entry| entry == requested) {
                Ok(root.join(requested))
            } else {
                Err(format!(
                    "default subfolder '{}' is not available for {}",
                    requested,
                    doc_type.as_key()
                ))
            }
        }
        (false, Some(requested)) => {
            if subfolders.iter().any(|entry| entry == requested) {
                Ok(root.join(requested))
            } else {
                Err(format!("unknown subfolder '{requested}'"))
            }
        }
    }
}

const fn default_reserve_subfolder(doc_type: VaultDocumentType) -> Option<&'static str> {
    match doc_type {
        VaultDocumentType::Task => Some("todo"),
        VaultDocumentType::Adr => Some("draft"),
        VaultDocumentType::Research
        | VaultDocumentType::Roadmap
        | VaultDocumentType::Guide
        | VaultDocumentType::Rule => None,
    }
}

fn configured_root(
    workspace: &Path,
    config: &VaultConfig,
    doc_type: VaultDocumentType,
) -> Result<PathBuf, String> {
    let key = doc_type.as_key();
    let root = config
        .documents
        .get(key)
        .ok_or_else(|| format!("vault config is missing document root for '{key}'"))?;
    Ok(workspace.join(&root.root))
}

fn parse_requested_id(number: &str) -> Result<u32, String> {
    number.parse::<u32>().map_err(|_| format!("invalid numeric id '{number}'"))
}

fn list_direct_subfolders(root: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(root)
        .map_err(|e| format!("could not read directory {}: {e}", root.display()))?;
    let mut subfolders = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not enumerate {}: {e}", root.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("could not read file type in {}: {e}", root.display()))?;
        if file_type.is_dir() {
            subfolders.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    subfolders.sort();
    Ok(subfolders)
}

fn find_matching_documents(root: &Path, target_id: u32) -> Result<Vec<DocumentMatch>, String> {
    let mut matches = Vec::new();
    collect_documents(root, &mut matches)?;
    Ok(matches.into_iter().filter(|entry| entry.id == target_id).collect())
}

fn find_max_numeric_prefix(root: &Path) -> Result<u32, String> {
    let mut matches = Vec::new();
    collect_documents(root, &mut matches)?;
    Ok(matches.into_iter().map(|entry| entry.id).max().unwrap_or(0))
}

fn collect_documents(dir: &Path, matches: &mut Vec<DocumentMatch>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("could not read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not enumerate {}: {e}", dir.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("could not read file type in {}: {e}", dir.display()))?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_documents(&path, matches)?;
        } else if file_type.is_file()
            && path.extension() == Some(OsStr::new("md"))
            && let Some(id) = parse_numeric_prefix(&path)
        {
            matches.push(DocumentMatch { id, path });
        }
    }
    Ok(())
}

fn parse_numeric_prefix(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    let (prefix, _) = stem.split_once(' ')?;
    prefix.parse::<u32>().ok()
}

fn display_relative(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace).unwrap_or(path).display().to_string().replace('\\', "/")
}

/// Reserves the next three-digit book code, creates the book folder and scaffolds `00 Index.md`.
///
/// # Side Effects
/// Creates a directory at `<books_root>/<NNN> <name>/` and writes
/// `<books_root>/<NNN> <name>/00 Index.md` with skeleton frontmatter.
fn reserve_book_command(
    workspace: &Path,
    config: &VaultConfig,
    name: &str,
) -> Result<BookReserveOutput, String> {
    validate_reserve_name(name)?;
    let books_root = book_root(workspace, config)?;
    let next_code = find_max_book_code(&books_root)?.saturating_add(1);
    let padded_code = format!("{next_code:0>BOOK_CODE_WIDTH$}");
    let folder_name = format!("{padded_code} {name}");
    let folder_path = books_root.join(&folder_name);

    if folder_path.exists() {
        return Err(format!(
            "book folder already exists: {}",
            display_relative(workspace, &folder_path)
        ));
    }

    fs::create_dir_all(&folder_path)
        .map_err(|e| format!("could not create book folder {}: {e}", folder_path.display()))?;

    let book_key = normalize_book_key(name);
    if book_key.is_empty() {
        return Err("book name does not produce a valid book key".to_string());
    }
    let index_path = folder_path.join("00 Index.md");
    let index_content = build_index_scaffold(&padded_code, &book_key, name);
    fs::write(&index_path, index_content)
        .map_err(|e| format!("could not write {}: {e}", index_path.display()))?;

    Ok(BookReserveOutput {
        book_code: padded_code,
        folder_path: display_relative(workspace, &folder_path),
        index_path: display_relative(workspace, &index_path),
        reserved_name: name.to_string(),
    })
}

fn book_root(workspace: &Path, config: &VaultConfig) -> Result<PathBuf, String> {
    let root = config
        .documents
        .get("book")
        .ok_or_else(|| "vault config is missing document root for 'book'".to_string())?;
    Ok(workspace.join(&root.root))
}

/// Scans the books root for folders whose names start with a three-digit numeric prefix
/// (`NNN <name>`) and returns the highest code found, or 0 if none exist.
fn find_max_book_code(books_root: &Path) -> Result<u32, String> {
    if !books_root.exists() {
        return Ok(0);
    }
    let entries = fs::read_dir(books_root)
        .map_err(|e| format!("could not read books root {}: {e}", books_root.display()))?;
    let max = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                parse_book_code_prefix(&entry.file_name().to_string_lossy())
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);
    Ok(max)
}

/// Parses a three-digit numeric prefix from a folder name of the form `NNN <rest>`.
#[must_use]
fn parse_book_code_prefix(folder_name: &str) -> Option<u32> {
    let (prefix, _) = folder_name.split_once(' ')?;
    if prefix.len() == BOOK_CODE_WIDTH && prefix.chars().all(|c| c.is_ascii_digit()) {
        prefix.parse().ok()
    } else {
        None
    }
}

#[must_use]
fn build_index_scaffold(book_code: &str, book_key: &str, name: &str) -> String {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    format!(
        "---\ntype: overview\nbook_code: {book_code}\nbook: {book_key}\ntitle: \"{name} — Book \
         Index\"\ndescription: \"Entry point for the {name} technical book in this \
         vault.\"\nstatus: draft\ncreated: {today}\ntags:\n  - book\n  - {book_key}\n  - \
         index\n---\n\n# {name}\n\n> [!ABSTRACT]\n> Describe the book scope and intended \
         audience here.\n\n## Contents\n\n<!-- Add chapter links here as you create them -->\n"
    )
}

const ARTIFACT_ID_WIDTH: usize = 4;

#[must_use]
fn normalize_book_key(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).flat_map(char::to_lowercase).collect()
}

#[must_use]
fn is_valid_book_key(value: &str) -> bool {
    !value.is_empty()
        && !value.contains([' ', '-'])
        && value.chars().all(|ch| ch.is_ascii_alphanumeric())
}

#[must_use]
fn canonical_book_artifact_filename(id: u32, book_key: &str, title: &str) -> String {
    format!("b{id:0>ARTIFACT_ID_WIDTH$} {book_key} {title}.md")
}

#[must_use]
fn derive_book_key_from_folder_name(book_folder: &Path) -> String {
    let folder_name = book_folder.file_name().and_then(|n| n.to_str()).unwrap_or("");
    folder_name.split_once(' ').map_or_else(String::new, |(_, rest)| normalize_book_key(rest))
}

/// Resolves the book folder for the given three-digit `book_code` under `books_root`.
///
/// Returns the first matching directory whose name starts with `<code> `, or an error if none.
fn find_book_folder(books_root: &Path, book_code: &str) -> Result<PathBuf, String> {
    if books_root.exists() {
        let entries = fs::read_dir(books_root)
            .map_err(|e| format!("could not read books root {}: {e}", books_root.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                format!("could not enumerate books root {}: {e}", books_root.display())
            })?;
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&format!("{book_code} ")) {
                    return Ok(entry.path());
                }
            }
        }
    }
    Err(format!("no book folder found for book code '{book_code}'"))
}

/// Returns the highest four-digit artifact id found under `book_folder` (across all subfolders),
/// or 0 if no artifacts exist yet.
///
/// Only files whose stems match a supported artifact pattern are counted; other files are skipped.
fn find_max_artifact_id(book_folder: &Path) -> Result<u32, String> {
    let mut max = 0u32;
    collect_artifact_ids(book_folder, &mut max)?;
    Ok(max)
}

fn collect_artifact_ids(dir: &Path, max: &mut u32) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("could not read directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not enumerate {}: {e}", dir.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("could not read file type in {}: {e}", dir.display()))?;
        let path = entry.path();
        if file_type.is_dir() {
            collect_artifact_ids(&path, max)?;
        } else if file_type.is_file()
            && let Some(id) = parse_artifact_id_prefix(&path)
            && id > *max
        {
            *max = id;
        }
    }
    Ok(())
}

/// Parses the artifact id prefix from a supported artifact filename stem.
///
/// Supported patterns:
/// - `bNNNN <book> <title>.md`
/// - legacy `NNNN-<title>.md`
///
/// Returns `None` for filenames that do not match a supported pattern (e.g. `00 Index.md`).
#[must_use]
fn parse_artifact_id_prefix(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_str()?;
    if let Some(rest) = stem.strip_prefix('b') {
        let (prefix, tail) = rest.split_once(' ')?;
        if prefix.len() == ARTIFACT_ID_WIDTH
            && prefix.chars().all(|c| c.is_ascii_digit())
            && !tail.trim().is_empty()
        {
            return prefix.parse().ok();
        }
    }
    let (prefix, rest) = stem.split_once('-')?;
    if prefix.len() == ARTIFACT_ID_WIDTH
        && prefix.chars().all(|c| c.is_ascii_digit())
        && !rest.is_empty()
    {
        return prefix.parse().ok();
    }
    None
}

/// Atomically reserves the next artifact id for the book identified by `book_code`, creates
/// the category subfolder if needed, and writes a skeleton artifact file.
fn reserve_book_artifact_command(
    workspace: &Path,
    config: &VaultConfig,
    book_code: &str,
    name: &str,
    category: &str,
) -> Result<BookArtifactReserveOutput, String> {
    validate_reserve_name(name)?;
    let books_root = book_root(workspace, config)?;
    let book_folder = find_book_folder(&books_root, book_code)?;

    let next_id = find_max_artifact_id(&book_folder)?.saturating_add(1);
    let category_dir = book_folder.join(category);
    if !category_dir.exists() {
        fs::create_dir_all(&category_dir).map_err(|e| {
            format!("could not create category folder {}: {e}", category_dir.display())
        })?;
    }

    let book_key = read_book_key_from_index(&book_folder)
        .unwrap_or_else(|| derive_book_key_from_folder_name(&book_folder));
    let filename = canonical_book_artifact_filename(next_id, &book_key, name);
    let file_path = category_dir.join(&filename);
    if file_path.exists() {
        return Err(format!(
            "artifact file already exists: {}",
            display_relative(workspace, &file_path)
        ));
    }

    let content = build_artifact_scaffold(next_id, book_code, &book_key, category, name);
    fs::write(&file_path, content)
        .map_err(|e| format!("could not write {}: {e}", file_path.display()))?;

    Ok(BookArtifactReserveOutput {
        id: format!("{next_id:0>ARTIFACT_ID_WIDTH$}"),
        book_code: book_code.to_string(),
        file_path: display_relative(workspace, &file_path),
        reserved_name: name.to_string(),
    })
}

fn read_book_key_from_index(book_folder: &Path) -> Option<String> {
    let raw = fs::read_to_string(book_folder.join("00 Index.md")).ok()?;
    let frontmatter = extract_frontmatter(&raw)?;
    let mapping = serde_yaml::from_str::<Mapping>(frontmatter.raw).ok()?;
    get_nonempty_string_field(&mapping, "book")
}

#[must_use]
fn build_artifact_scaffold(
    id: u32,
    book_code: &str,
    book_key: &str,
    category: &str,
    name: &str,
) -> String {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    format!(
        "---\ntype: concept\nbook_code: {book_code}\nbook: {book_key}\nid: \
         {id}\nchapter: \nsequence: \ncategory: {category}\ntitle: \"{name}\"\ndescription: \
         \"\"\nstatus: draft\ncreated: {today}\ntags:\n  - {book_key}\n  - {category}\nprerequisites: \
         []\n---\n\n# {name}\n\n<!-- Write the first paragraph here: what is this, why does it \
         matter, what will the reader know after reading it. -->\n"
    )
}

impl BookArtifactReserveOutput {
    #[must_use]
    fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "ID={}", self.id);
        let _ = writeln!(out, "BOOK_CODE={}", self.book_code);
        let _ = writeln!(out, "FILE_PATH={}", self.file_path);
        let _ = writeln!(out, "RESERVED_NAME={}", self.reserved_name);
        out
    }
}

/// Required frontmatter fields for every **artifact** file under a book (ADR 0104 §5).
/// `00 Index.md` (overview) uses a shorter set — see [`BOOK_INDEX_REQUIRED_FIELDS`].
const BOOK_ARTIFACT_REQUIRED_FIELDS: &[&str] = &[
    "type",
    "book_code",
    "book",
    "id",
    "chapter",
    "sequence",
    "category",
    "title",
    "description",
    "status",
    "tags",
];

/// Required frontmatter fields for `00 Index.md` overview files (ADR 0104 §5 / §6).
const BOOK_INDEX_REQUIRED_FIELDS: &[&str] =
    &["type", "book_code", "book", "title", "description", "status", "tags"];

/// Walk `books_root`, find every book folder, and validate all markdown files inside.
///
/// The four sync invariants from ADR 0104 §11.3 are checked here.  No auto-fix is
/// performed for sync invariants — the frontmatter or filename must be corrected by hand.
/// Missing required fields are reported as `MISSING_FIELDS`.
fn collect_book_check_findings(
    workspace: &Path,
    books_root: &Path,
    link_context: &LinkNormalizationContext,
    fix: bool,
    lines: &mut Vec<String>,
) {
    if !books_root.exists() {
        return;
    }
    let mut seen_book_keys = BTreeMap::<String, String>::new();
    let Ok(entries) = fs::read_dir(books_root) else {
        lines.push(format!(
            "{INVALID_FIELDS_PREFIX} {} frontmatter",
            display_relative(workspace, books_root)
        ));
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            // Only process folders whose names match the NNN <name> pattern.
            if let Some(book_code) = parse_book_code_prefix(&entry.file_name().to_string_lossy()) {
                let padded = format!("{book_code:0>BOOK_CODE_WIDTH$}");
                if let Some(book_key) =
                    walk_book_folder(workspace, &path, &padded, link_context, fix, lines)
                {
                    let relative = display_relative(workspace, &path);
                    if let Some(first_path) =
                        seen_book_keys.insert(book_key.clone(), relative.clone())
                    {
                        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} book"));
                        lines.push(format!("{INVALID_FIELDS_PREFIX} {first_path} book"));
                    }
                }
            }
        }
    }
}

/// Walk a single book folder and validate every `.md` file inside.
fn walk_book_folder(
    workspace: &Path,
    book_folder: &Path,
    book_code: &str,
    link_context: &LinkNormalizationContext,
    fix: bool,
    lines: &mut Vec<String>,
) -> Option<String> {
    let Ok(entries) = fs::read_dir(book_folder) else {
        return None;
    };
    let mut book_key = None;
    let mut subfolders = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            subfolders.push((entry.file_name().to_string_lossy().to_string(), path));
        } else if ft.is_file() && path.extension() == Some(OsStr::new("md")) {
            // Files directly in the book root (e.g. `00 Index.md`).
            book_key = validate_book_index(
                workspace,
                &path,
                book_code,
                book_folder,
                link_context,
                fix,
                lines,
            );
        }
    }
    let expected_book_key =
        book_key.unwrap_or_else(|| derive_book_key_from_folder_name(book_folder));
    for (subfolder_name, path) in subfolders {
        if subfolder_name == "assets" {
            continue;
        }
        walk_book_subfolder(
            workspace,
            &path,
            book_code,
            &expected_book_key,
            &subfolder_name,
            link_context,
            fix,
            lines,
        );
    }
    Some(expected_book_key)
}

/// Walk a category subfolder (`memory/`, `concurrency/`, …) and validate artifact files.
#[allow(clippy::too_many_arguments)]
fn walk_book_subfolder(
    workspace: &Path,
    dir: &Path,
    book_code: &str,
    expected_book_key: &str,
    category_name: &str,
    link_context: &LinkNormalizationContext,
    fix: bool,
    lines: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            // Recurse into nested subfolders; category name stays fixed at this level.
            walk_book_subfolder(
                workspace,
                &path,
                book_code,
                expected_book_key,
                category_name,
                link_context,
                fix,
                lines,
            );
        } else if ft.is_file() && path.extension() == Some(OsStr::new("md")) {
            validate_book_artifact(
                workspace,
                &path,
                book_code,
                expected_book_key,
                category_name,
                link_context,
                fix,
                lines,
            );
        }
    }
}

/// Validate a `00 Index.md` (or any `.md` directly in the book root).
fn validate_book_index(
    workspace: &Path,
    path: &Path,
    expected_book_code: &str,
    book_folder: &Path,
    link_context: &LinkNormalizationContext,
    fix: bool,
    lines: &mut Vec<String>,
) -> Option<String> {
    let relative = display_relative(workspace, path);
    let Ok(raw) = fs::read_to_string(path) else {
        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} frontmatter"));
        return None;
    };
    let mut mapping = match extract_frontmatter(&raw) {
        Some(fm) => if let Ok(m) = serde_yaml::from_str::<Mapping>(fm.raw) { m } else {
            lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} frontmatter"));
            return None;
        },
        None => Mapping::new(),
    };

    let mut missing = Vec::new();
    for &field in BOOK_INDEX_REQUIRED_FIELDS {
        if !field_present_and_nonempty(&mapping, field) {
            missing.push(field.to_string());
        }
    }

    // Sync: book_code in frontmatter must match the folder prefix.
    if !missing.contains(&"book_code".to_string())
        && let Some(actual) = get_string_field(&mapping, "book_code")
        && actual != expected_book_code
    {
        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} book_code"));
    }
    let expected_book_key = derive_book_key_from_folder_name(book_folder);
    let actual_book_key = if fix {
        ensure_book_key_metadata(&mut mapping, &expected_book_key)
    } else {
        get_nonempty_string_field(&mapping, "book")
    };
    if let Some(book_key) = actual_book_key.as_deref() {
        if !is_valid_book_key(book_key) || book_key != expected_book_key {
            lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} book"));
        }
    } else if !missing.contains(&"book".to_string()) {
        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} book"));
    }

    if !missing.is_empty() {
        lines.push(format!("{REQUIRED_FIELDS_PREFIX} {relative} {}", missing.join(" ")));
    }
    let mut updated_raw = raw.clone();
    if fix
        && let Some(frontmatter) = extract_frontmatter(&raw)
        && let Ok(rendered) = render_document_with_frontmatter(&mapping, frontmatter.body)
    {
        updated_raw = rendered;
    }
    let link_output = normalize_governed_links(&updated_raw, path, link_context, fix);
    if !link_output.invalid_fields.is_empty() {
        lines.push(format!(
            "{INVALID_FIELDS_PREFIX} {relative} {}",
            link_output.invalid_fields.join(" ")
        ));
    }
    if fix && link_output.updated != raw {
        let _ = fs::write(path, link_output.updated);
    }
    actual_book_key
}

/// Validate an artifact file (`NNNN-kebab-name.md`) inside a category subfolder.
///
/// Enforces ADR 0104 §11.3:
/// 1. `category` matches parent subfolder name.
/// 2. `id` matches numeric prefix of filename.
/// 3. `book_code` matches numeric prefix of book folder.
/// 4. All required fields present and non-empty.
#[allow(clippy::too_many_arguments)]
fn validate_book_artifact(
    workspace: &Path,
    path: &Path,
    expected_book_code: &str,
    expected_book_key: &str,
    expected_category: &str,
    link_context: &LinkNormalizationContext,
    fix: bool,
    lines: &mut Vec<String>,
) {
    let relative = display_relative(workspace, path);
    let Ok(raw) = fs::read_to_string(path) else {
        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} frontmatter"));
        return;
    };
    let mut current_path = path.to_path_buf();
    let mut mapping = match extract_frontmatter(&raw) {
        Some(fm) => if let Ok(m) = serde_yaml::from_str::<Mapping>(fm.raw) { m } else {
            lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} frontmatter"));
            return;
        },
        None => Mapping::new(),
    };

    // 4. Required fields — collect missing/empty ones first.
    let mut missing = Vec::new();
    for &field in BOOK_ARTIFACT_REQUIRED_FIELDS {
        if !field_present_and_nonempty(&mapping, field) {
            missing.push(field.to_string());
        }
    }

    // 1. category sync
    let mut invalid = Vec::new();
    if !missing.contains(&"category".to_string())
        && let Some(actual) = get_string_field(&mapping, "category")
        && actual != expected_category
    {
        invalid.push("category".to_string());
    }

    // 2. id sync — the numeric frontmatter id must match the bNNNN filename prefix.
    if !missing.contains(&"id".to_string())
        && let Some(expected_id) = parse_artifact_id_prefix(path)
    {
        match mapping.get(Value::String("id".to_string())).and_then(parse_frontmatter_id) {
            Some(actual) if actual == expected_id => {}
            _ => invalid.push("id".to_string()),
        }
    }

    // 3. book_code sync
    if !missing.contains(&"book_code".to_string())
        && let Some(actual) = get_string_field(&mapping, "book_code")
        && actual != expected_book_code
    {
        invalid.push("book_code".to_string());
    }
    if !missing.contains(&"book".to_string())
        && let Some(actual) = get_string_field(&mapping, "book")
        && (!is_valid_book_key(&actual) || actual != expected_book_key)
    {
        invalid.push("book".to_string());
    }
    if mapping.contains_key(Value::String("artifact_id".to_string())) {
        if fix {
            mapping.remove(Value::String("artifact_id".to_string()));
        } else {
            invalid.push("artifact_id".to_string());
        }
    }

    if fix {
        let actual_book_key = ensure_book_key_metadata(&mut mapping, expected_book_key);
        if let (Some(book_key), Some(title), Some(id)) = (
            actual_book_key,
            get_nonempty_string_field(&mapping, "title"),
            mapping
                .get(Value::String("id".to_string()))
                .and_then(parse_frontmatter_id)
                .or_else(|| parse_artifact_id_prefix(&current_path)),
        ) {
            let canonical_name = canonical_book_artifact_filename(id, &book_key, &title);
            if let Some(parent) = current_path.parent() {
                let target_path = parent.join(canonical_name);
                if target_path != current_path {
                    if target_path.exists() {
                        invalid.push("location".to_string());
                    } else {
                        let _ = fs::rename(&current_path, &target_path);
                        current_path = target_path;
                    }
                }
            }
        }
        let mut updated_raw = raw.clone();
        if let Some(frontmatter) = extract_frontmatter(&raw)
            && let Ok(rendered) = render_document_with_frontmatter(&mapping, frontmatter.body)
        {
            updated_raw = rendered;
        }
        let link_output = normalize_governed_links(&updated_raw, path, link_context, true);
        invalid.extend(link_output.invalid_fields);
        if link_output.updated != raw {
            let _ = fs::write(&current_path, link_output.updated);
        }
    } else {
        invalid.extend(normalize_governed_links(&raw, path, link_context, false).invalid_fields);
    }

    if !missing.is_empty() {
        lines.push(format!("{REQUIRED_FIELDS_PREFIX} {relative} {}", missing.join(" ")));
    }
    if !invalid.is_empty() {
        invalid.sort();
        invalid.dedup();
        lines.push(format!("{INVALID_FIELDS_PREFIX} {relative} {}", invalid.join(" ")));
    }
}

fn ensure_book_key_metadata(mapping: &mut Mapping, expected_book_key: &str) -> Option<String> {
    let key = Value::String("book".to_string());
    match mapping.get(&key) {
        Some(Value::String(value))
            if !value.trim().is_empty()
                && is_valid_book_key(value.trim())
                && value.trim() == expected_book_key =>
        {
            Some(value.trim().to_string())
        }
        _ => {
            mapping.insert(key, Value::String(expected_book_key.to_string()));
            Some(expected_book_key.to_string())
        }
    }
}

/// Returns `true` when the field is present in the mapping **and** its value is a
/// non-empty string, a non-empty sequence, or any numeric/boolean value.
fn field_present_and_nonempty(mapping: &Mapping, field: &str) -> bool {
    match mapping.get(Value::String(field.to_string())) {
        None => false,
        Some(Value::String(s)) => !s.trim().is_empty(),
        Some(Value::Sequence(seq)) => !seq.is_empty(),
        Some(Value::Null) => false,
        Some(_) => true, // numbers, booleans — always considered present
    }
}

/// Extracts the value of a field from a frontmatter mapping as a `String`.
///
/// Returns `None` only when the field is absent.  When the value is a YAML number it is
/// rendered as its decimal representation so that `book_code: 001` (string) and
/// `book_code: 1` (number) can both be compared against the padded folder prefix `"001"`.
/// Callers are responsible for zero-padding comparisons where needed.
fn get_string_field(mapping: &Mapping, field: &str) -> Option<String> {
    match mapping.get(Value::String(field.to_string()))? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl BookReserveOutput {
    #[must_use]
    fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "BOOK_CODE={}", self.book_code);
        let _ = writeln!(out, "FOLDER_PATH={}", self.folder_path);
        let _ = writeln!(out, "INDEX_PATH={}", self.index_path);
        let _ = writeln!(out, "RESERVED_NAME={}", self.reserved_name);
        out
    }
}

impl VaultOutput {
    #[must_use]
    fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "TYPE={}", self.doc_type.as_key());
        if let Some(id) = &self.id {
            let _ = writeln!(out, "ID={id}");
        }
        if let Some(last_id) = &self.last_id {
            let _ = writeln!(out, "LAST_ID={last_id}");
        }
        let _ = writeln!(out, "PATH={}", self.path);
        let _ = writeln!(out, "SUBFOLDERS={}", self.subfolders.join(","));
        if let Some(name) = &self.reserved_name {
            let _ = writeln!(out, "RESERVED_NAME={name}");
        }
        out
    }
}

#[cfg(test)]
#[path = "vault_test.rs"]
mod tests;
