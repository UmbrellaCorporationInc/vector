//! RULE-F1: No unauthorized `#[allow(...)]` anywhere (`--future`).
//!
//! In non-test code: any `#[allow(...)]` is flagged regardless of argument,
//! except in CLI binary crates (`cli/` path prefix) where `clippy::print_stderr`
//! and `clippy::print_stdout` are permitted (CLI binaries write to stderr/stdout
//! by design).
//! In test contexts (`*_test.rs` files or `#[cfg(test)]` modules): only
//! arguments outside the permitted set are flagged.
//! Files that contain a `#[grammar = "..."]` attribute (pest-derive parser
//! modules) are entirely exempt — their `#[allow]` suppressions are a direct
//! consequence of pest-generated code that cannot carry doc comments.
//!
//! Permitted set in test contexts: `clippy::unwrap_used`, `clippy::expect_used`,
//! `clippy::print_stdout`, `clippy::similar_names`,
//! `clippy::permissions_set_readonly_false`.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

const PERMITTED_IN_TESTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::print_stdout",
    "clippy::similar_names",
    "clippy::permissions_set_readonly_false",
    "clippy::indexing_slicing",
    "clippy::as_conversions",
];

/// Lints that are currently allowed everywhere but will be forbidden in the
/// future (activated for validation when `--future` is used).
const PERMITTED: &[&str] = &["clippy::as_conversions", "clippy::indexing_slicing"];

/// Allowed in production code of CLI binary crates (`cli/` path prefix).
const PERMITTED_IN_CLI: &[&str] = &["clippy::print_stderr", "clippy::print_stdout"];

/// Allowed in pest-derive parser modules (files with `#[grammar = "..."]`).
/// Only doc-related suppressions are permitted — pest generates undocumented items.
const PERMITTED_IN_PEST: &[&str] = &["missing_docs", "clippy::missing_docs_in_private_items"];

pub struct NoAllowAnywhere {
    pub(crate) is_future: bool,
}

impl Rule for NoAllowAnywhere {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let is_test_file = path_str.ends_with("_test.rs") || path_str.contains("/tests/");
        let is_cli = path_str.contains("/cli/");
        let is_pest = is_pest_grammar_file(ast);

        let mut visitor = Visitor {
            path,
            out,
            in_test_context: is_test_file,
            is_cli,
            is_pest,
            is_future: self.is_future,
        };
        visitor.visit_file(ast);
    }
}

// ─── visitor ────────────────────────────────────────────────────────────────

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
    in_test_context: bool,
    is_cli: bool,
    is_pest: bool,
    is_future: bool,
}

impl Visit<'_> for Visitor<'_> {
    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        let old_context = self.in_test_context;
        if has_cfg_test(&node.attrs) {
            self.in_test_context = true;
        }
        self.check_attrs(&node.attrs);
        visit::visit_item_mod(self, node);
        self.in_test_context = old_context;
    }

    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        let old_context = self.in_test_context;
        if has_cfg_test(&node.attrs) || has_test_attr(&node.attrs) {
            self.in_test_context = true;
        }
        self.check_attrs(&node.attrs);
        visit::visit_item_fn(self, node);
        self.in_test_context = old_context;
    }

    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        self.check_attrs(&node.attrs);
        visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &syn::ItemEnum) {
        self.check_attrs(&node.attrs);
        visit::visit_item_enum(self, node);
    }

    fn visit_item_impl(&mut self, node: &syn::ItemImpl) {
        self.check_attrs(&node.attrs);
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        self.check_attrs(&node.attrs);
        visit::visit_item_trait(self, node);
    }

    fn visit_file(&mut self, node: &syn::File) {
        self.check_attrs(&node.attrs);
        visit::visit_file(self, node);
    }
}

impl Visitor<'_> {
    fn check_attrs(&mut self, attrs: &[syn::Attribute]) {
        for attr in attrs {
            if !attr.path().is_ident("allow") {
                continue;
            }
            if self.in_test_context {
                self.check_attr_in_test_context(attr);
            } else if self.is_pest {
                self.check_attr_in_pest_context(attr);
            } else if self.is_cli {
                self.check_attr_in_cli_context(attr);
            } else {
                self.check_attr_in_production_context(attr);
            }
        }
    }

    fn check_attr_in_pest_context(&mut self, attr: &syn::Attribute) {
        let mut unauthorized: Vec<String> = Vec::new();
        let _ = attr.parse_nested_meta(|meta| {
            use quote::ToTokens;
            let path_str = meta.path.to_token_stream().to_string().replace(' ', "");
            if !self.is_permitted(&path_str, PERMITTED_IN_PEST) {
                unauthorized.push(path_str);
            }
            Ok(())
        });
        for arg in unauthorized {
            self.flag(
                attr,
                "RULE-F1",
                format!(
                    "#[allow({arg})] is not in the permitted pest-grammar allow-list \
                     (permitted: missing_docs, clippy::missing_docs_in_private_items)"
                ),
            );
        }
    }

    fn check_attr_in_cli_context(&mut self, attr: &syn::Attribute) {
        let mut unauthorized: Vec<String> = Vec::new();
        let _ = attr.parse_nested_meta(|meta| {
            use quote::ToTokens;
            let path_str = meta.path.to_token_stream().to_string().replace(' ', "");
            if !self.is_permitted(&path_str, PERMITTED_IN_CLI) {
                unauthorized.push(path_str);
            }
            Ok(())
        });
        for arg in unauthorized {
            self.flag(
                attr,
                "RULE-F1",
                format!(
                    "#[allow({arg})] is not in the permitted CLI allow-list \
                     (permitted: clippy::print_stderr, clippy::print_stdout)"
                ),
            );
        }
    }

    fn check_attr_in_test_context(&mut self, attr: &syn::Attribute) {
        // Collect all arguments in the allow list; flag those not in permitted set.
        let mut unauthorized: Vec<String> = Vec::new();
        let _ = attr.parse_nested_meta(|meta| {
            use quote::ToTokens;
            let path_str = meta.path.to_token_stream().to_string().replace(' ', "");
            if !self.is_permitted(&path_str, PERMITTED_IN_TESTS) {
                unauthorized.push(path_str);
            }
            Ok(())
        });
        for arg in unauthorized {
            self.flag(
                attr,
                "RULE-F1",
                format!(
                    "#[allow({arg})] is not in the permitted test allow-list \
                     (permitted: clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)"
                ),
            );
        }
    }

    fn check_attr_in_production_context(&mut self, attr: &syn::Attribute) {
        let mut unauthorized: Vec<String> = Vec::new();
        let _ = attr.parse_nested_meta(|meta| {
            use quote::ToTokens;
            let path_str = meta.path.to_token_stream().to_string().replace(' ', "");
            if !self.is_permitted(&path_str, &[]) {
                unauthorized.push(path_str);
            }
            Ok(())
        });
        for arg in unauthorized {
            self.flag(
                attr,
                "RULE-F1",
                format!(
                    "#[allow({arg})] is forbidden in production code — use a targeted suppression comment or fix the underlying issue"
                ),
            );
        }
    }

    fn is_permitted(&self, path_str: &str, context_permitted: &[&str]) -> bool {
        context_permitted.contains(&path_str) || (!self.is_future && PERMITTED.contains(&path_str))
    }

    fn flag(&mut self, attr: &syn::Attribute, rule_id: &'static str, message: String) {
        let span = attr.span();
        self.out.push(RuleViolation {
            file: self.path.to_path_buf(),
            line: Some(span.start().line as u32),
            column: Some(span.start().column as u32 + 1),
            rule_id,
            message,
        });
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Returns `true` when the file contains a `#[grammar = "..."]` attribute on any
/// struct — the canonical marker of a `pest_derive`-generated parser module.
/// All `#[allow]` suppressions in such files are exempt from RULE-F1 because
/// the generated `Rule` enum cannot carry doc comments.
fn is_pest_grammar_file(file: &syn::File) -> bool {
    file.items.iter().any(|item| {
        let syn::Item::Struct(s) = item else { return false };
        s.attrs.iter().any(|attr| attr.path().is_ident("grammar"))
    })
}

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        if let syn::Meta::List(list) = &attr.meta {
            list.tokens.to_string().trim() == "test"
        } else {
            false
        }
    })
}

fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

#[cfg(test)]
#[path = "no_allow_anywhere_test.rs"]
mod tests;
