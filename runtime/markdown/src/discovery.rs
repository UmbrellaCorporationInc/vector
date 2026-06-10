//! Markdown discovery APIs.

use runtime_io::{
    DirectoryEntry, DirectoryTraversalOptions, FileContentHash, IoError, IoPath, hash_file_content,
    traverse_directory_with_options,
};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

type IssueSortKey<'a> = (u8, Option<&'a str>, PathBuf, &'a str);

/// Hashing behavior requested by Markdown discovery callers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownHashingMode {
    /// Compute content hashes for discovered Markdown files.
    #[default]
    Content,
}

/// Deterministic result of Markdown corpus discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDiscoveryReport {
    records: Vec<MarkdownDiscoveryRecord>,
    issues: Vec<MarkdownDiscoveryIssue>,
}

impl MarkdownDiscoveryReport {
    /// Create a discovery report.
    #[must_use]
    pub const fn new(
        records: Vec<MarkdownDiscoveryRecord>,
        issues: Vec<MarkdownDiscoveryIssue>,
    ) -> Self {
        Self { records, issues }
    }

    /// Return discovered Markdown file records.
    #[must_use]
    pub const fn records(&self) -> &[MarkdownDiscoveryRecord] {
        self.records.as_slice()
    }

    /// Return non-fatal discovery issues.
    #[must_use]
    pub const fn issues(&self) -> &[MarkdownDiscoveryIssue] {
        self.issues.as_slice()
    }
}

/// Stable Markdown file record used by RAG indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDiscoveryRecord {
    package: Option<String>,
    governed_document_stem: String,
    modified_time: Option<SystemTime>,
    content_hash: FileContentHash,
    internal_read_path: IoPath,
}

impl MarkdownDiscoveryRecord {
    /// Create a Markdown discovery record.
    #[must_use]
    pub const fn new(
        package: Option<String>,
        governed_document_stem: String,
        modified_time: Option<SystemTime>,
        content_hash: FileContentHash,
        internal_read_path: IoPath,
    ) -> Self {
        Self { package, governed_document_stem, modified_time, content_hash, internal_read_path }
    }

    /// Return the package identity, or `None` for workspace-local documents.
    #[must_use]
    pub fn package(&self) -> Option<&str> {
        self.package.as_deref()
    }

    /// Return the governed document stem.
    #[must_use]
    pub const fn governed_document_stem(&self) -> &str {
        self.governed_document_stem.as_str()
    }

    /// Return the file modification time when available.
    #[must_use]
    pub const fn modified_time(&self) -> Option<SystemTime> {
        self.modified_time
    }

    /// Return the content hash computed from file bytes only.
    #[must_use]
    pub const fn content_hash(&self) -> &FileContentHash {
        &self.content_hash
    }

    /// Return the path the indexer should use to read the file.
    #[must_use]
    pub const fn internal_read_path(&self) -> &IoPath {
        &self.internal_read_path
    }
}

/// Non-fatal issue found during Markdown discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownDiscoveryIssue {
    /// A package document root could not be traversed.
    PackageStructure {
        /// Package identity associated with the failing root.
        package: String,
        /// Package document root that failed.
        doc_root: IoPath,
        /// Underlying IO error message.
        message: String,
    },

    /// A Markdown file did not use the governed document stem format.
    InvalidGovernedDocumentStem {
        /// Package identity, or `None` for workspace-local documents.
        package: Option<String>,
        /// File path with the invalid stem.
        path: IoPath,
        /// Invalid file stem.
        stem: String,
    },

    /// A candidate file could not be hashed.
    ContentHash {
        /// Package identity, or `None` for workspace-local documents.
        package: Option<String>,
        /// Candidate file path.
        path: IoPath,
        /// Underlying IO error message.
        message: String,
    },
}

/// Fatal Markdown discovery failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownDiscoveryFailure {
    /// A workspace document root could not be traversed.
    WorkspaceDiscovery {
        /// Workspace document root that failed.
        root: IoPath,
        /// Underlying IO error message.
        message: String,
    },
}

impl std::fmt::Display for MarkdownDiscoveryFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceDiscovery { root, message } => write!(
                formatter,
                "workspace Markdown discovery failed for {}: {message}",
                root.as_path().display()
            ),
        }
    }
}

impl std::error::Error for MarkdownDiscoveryFailure {}

/// Package-specific Markdown document root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMarkdownRoot {
    package: String,
    doc_root: IoPath,
}

impl PackageMarkdownRoot {
    /// Create a package Markdown root.
    #[must_use]
    pub fn new(package: impl Into<String>, doc_root: impl AsRef<Path>) -> Self {
        Self { package: package.into(), doc_root: IoPath::new(doc_root) }
    }

    /// Return the package identity associated with this root.
    #[must_use]
    pub const fn package(&self) -> &str {
        self.package.as_str()
    }

    /// Return the package document root.
    #[must_use]
    pub const fn doc_root(&self) -> &IoPath {
        &self.doc_root
    }
}

/// Explicit inputs required by Markdown discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDiscoveryRequest {
    workspace_doc_roots: Vec<IoPath>,
    package_doc_roots: Vec<PackageMarkdownRoot>,
    traversal_options: DirectoryTraversalOptions,
    hashing_mode: MarkdownHashingMode,
}

impl MarkdownDiscoveryRequest {
    /// Create a Markdown discovery request from explicit workspace and package roots.
    #[must_use]
    pub fn new(
        workspace_doc_roots: impl IntoIterator<Item = impl AsRef<Path>>,
        package_doc_roots: impl IntoIterator<Item = PackageMarkdownRoot>,
    ) -> Self {
        Self {
            workspace_doc_roots: workspace_doc_roots.into_iter().map(IoPath::new).collect(),
            package_doc_roots: package_doc_roots.into_iter().collect(),
            traversal_options: DirectoryTraversalOptions::default(),
            hashing_mode: MarkdownHashingMode::default(),
        }
    }

    /// Return the workspace-local document roots.
    #[must_use]
    pub const fn workspace_doc_roots(&self) -> &[IoPath] {
        self.workspace_doc_roots.as_slice()
    }

    /// Return the package document roots.
    #[must_use]
    pub const fn package_doc_roots(&self) -> &[PackageMarkdownRoot] {
        self.package_doc_roots.as_slice()
    }

    /// Return the generic traversal options supplied by the caller.
    #[must_use]
    pub const fn traversal_options(&self) -> &DirectoryTraversalOptions {
        &self.traversal_options
    }

    /// Return the requested hashing behavior.
    #[must_use]
    pub const fn hashing_mode(&self) -> MarkdownHashingMode {
        self.hashing_mode
    }

    /// Return a request with updated generic traversal options.
    #[must_use]
    pub fn with_traversal_options(mut self, traversal_options: DirectoryTraversalOptions) -> Self {
        self.traversal_options = traversal_options;
        self
    }

    /// Return a request with updated hashing behavior.
    #[must_use]
    pub const fn with_hashing_mode(mut self, hashing_mode: MarkdownHashingMode) -> Self {
        self.hashing_mode = hashing_mode;
        self
    }
}

/// Discover Markdown files from explicit workspace and package document roots.
///
/// Package root errors are returned as report issues so callers can distinguish
/// package structure problems from workspace discovery failures.
///
/// # Errors
/// Returns [`MarkdownDiscoveryFailure::WorkspaceDiscovery`] when a workspace
/// document root cannot be traversed.
pub async fn discover_markdown_files(
    request: &MarkdownDiscoveryRequest,
) -> Result<MarkdownDiscoveryReport, MarkdownDiscoveryFailure> {
    let mut records = Vec::new();
    let mut issues = Vec::new();

    for root in request.workspace_doc_roots() {
        let entries = traverse_directory_with_options(root, request.traversal_options())
            .await
            .map_err(|error| MarkdownDiscoveryFailure::WorkspaceDiscovery {
                root: root.clone(),
                message: io_error_message(&error),
            })?;
        discover_entries(None, entries, request.hashing_mode(), &mut records, &mut issues).await;
    }

    for package_root in request.package_doc_roots() {
        match traverse_directory_with_options(package_root.doc_root(), request.traversal_options())
            .await
        {
            Ok(entries) => {
                discover_entries(
                    Some(package_root.package().to_owned()),
                    entries,
                    request.hashing_mode(),
                    &mut records,
                    &mut issues,
                )
                .await;
            }
            Err(error) => issues.push(MarkdownDiscoveryIssue::PackageStructure {
                package: package_root.package().to_owned(),
                doc_root: package_root.doc_root().clone(),
                message: io_error_message(&error),
            }),
        }
    }

    sort_records(&mut records);
    sort_issues(&mut issues);

    Ok(MarkdownDiscoveryReport::new(records, issues))
}

async fn discover_entries(
    package: Option<String>,
    entries: Vec<DirectoryEntry>,
    hashing_mode: MarkdownHashingMode,
    records: &mut Vec<MarkdownDiscoveryRecord>,
    issues: &mut Vec<MarkdownDiscoveryIssue>,
) {
    for entry in entries.into_iter().filter(DirectoryEntry::is_file) {
        if !is_markdown_path(entry.path().as_path()) {
            continue;
        }

        let Some(stem) = path_stem(entry.path().as_path()) else {
            continue;
        };

        if !is_governed_document_stem(&stem) {
            issues.push(MarkdownDiscoveryIssue::InvalidGovernedDocumentStem {
                package: package.clone(),
                path: entry.path().clone(),
                stem,
            });
            continue;
        }

        let content_hash = match hashing_mode {
            MarkdownHashingMode::Content => match hash_file_content(entry.path()).await {
                Ok(hash) => hash,
                Err(error) => {
                    issues.push(MarkdownDiscoveryIssue::ContentHash {
                        package: package.clone(),
                        path: entry.path().clone(),
                        message: io_error_message(&error),
                    });
                    continue;
                }
            },
        };

        records.push(MarkdownDiscoveryRecord::new(
            package.clone(),
            stem,
            entry.modified(),
            content_hash,
            entry.path().clone(),
        ));
    }
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| {
        extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
    })
}

fn path_stem(path: &Path) -> Option<String> {
    path.file_stem().and_then(|stem| stem.to_str()).map(ToOwned::to_owned)
}

fn is_governed_document_stem(stem: &str) -> bool {
    let parts = stem.split('-').collect::<Vec<_>>();
    if parts.len() < 3 {
        return false;
    }

    let Some(code_index) = parts.iter().position(|part| is_code_part(part)) else {
        return false;
    };

    code_index > 0
        && code_index + 1 < parts.len()
        && parts[..code_index].iter().all(|part| is_kebab_part(part))
        && parts[code_index + 1..].iter().all(|part| is_kebab_part(part))
}

fn is_code_part(part: &str) -> bool {
    !part.is_empty() && part.chars().all(|character| character.is_ascii_digit())
}

fn is_kebab_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

fn io_error_message(error: &IoError) -> String {
    error.to_string()
}

fn sort_records(records: &mut [MarkdownDiscoveryRecord]) {
    records.sort_by(|left, right| {
        left.package
            .cmp(&right.package)
            .then_with(|| left.governed_document_stem.cmp(&right.governed_document_stem))
            .then_with(|| left.internal_read_path.as_path().cmp(right.internal_read_path.as_path()))
    });
}

fn sort_issues(issues: &mut [MarkdownDiscoveryIssue]) {
    issues.sort_by(|left, right| issue_sort_key(left).cmp(&issue_sort_key(right)));
}

fn issue_sort_key(issue: &MarkdownDiscoveryIssue) -> IssueSortKey<'_> {
    match issue {
        MarkdownDiscoveryIssue::PackageStructure { package, doc_root, message } => {
            (0, Some(package.as_str()), doc_root.as_path().to_path_buf(), message.as_str())
        }
        MarkdownDiscoveryIssue::InvalidGovernedDocumentStem { package, path, stem } => {
            (1, package.as_deref(), path.as_path().to_path_buf(), stem.as_str())
        }
        MarkdownDiscoveryIssue::ContentHash { package, path, message } => {
            (2, package.as_deref(), path.as_path().to_path_buf(), message.as_str())
        }
    }
}

#[cfg(test)]
#[path = "discovery_test.rs"]
mod tests;
