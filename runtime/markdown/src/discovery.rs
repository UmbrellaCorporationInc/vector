//! Markdown discovery request types.

use runtime_io::{DirectoryTraversalOptions, IoPath};
use std::path::Path;

/// Hashing behavior requested by Markdown discovery callers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum MarkdownHashingMode {
    /// Compute content hashes for discovered Markdown files.
    #[default]
    Content,
}

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

#[cfg(test)]
#[path = "discovery_test.rs"]
mod tests;
