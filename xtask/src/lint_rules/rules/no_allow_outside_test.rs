//! RULE-7: No `#[allow(clippy::unwrap_used)]` or `#[allow(clippy::expect_used)]` outside tests.
//!
//! Enforces Rule 7 by flagging explicit `unwrap`/`expect` allow-attributes
//! in production code. These are only permitted within `#[cfg(test)]` modules
//! or `*_test.rs` files.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

pub(crate) struct NoAllowOutsideTest;

impl Rule for NoAllowOutsideTest {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let is_test_context = path_str.ends_with("_test.rs") || path_str.contains("/tests/");

        let mut visitor = Visitor { path, out, in_test_context: is_test_context };
        visitor.visit_file(ast);
    }
}

// ─── visitor ────────────────────────────────────────────────────────────────

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
    in_test_context: bool,
}

impl<'a> Visit<'_> for Visitor<'a> {
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

impl<'a> Visitor<'a> {
    fn check_attrs(&mut self, attrs: &[syn::Attribute]) {
        if self.in_test_context {
            return;
        }

        for attr in attrs {
            if is_restricted_allow_attr(attr) {
                let span = attr.span();
                self.out.push(RuleViolation {
                    file: self.path.to_path_buf(),
                    line: Some(span.start().line as u32),
                    column: Some(span.start().column as u32 + 1),
                    rule_id: "RULE-7",
                    message: "explicit #[allow(clippy::unwrap_used / expect_used)] found in production code — these are only permitted in test modules (*_test.rs or #[cfg(test)])".into(),
                });
            }
        }
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        if let syn::Meta::List(list) = &attr.meta {
            // Using a simple token string check for robustness.
            let tokens = list.tokens.to_string();
            tokens.trim() == "test"
        } else {
            false
        }
    })
}

fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("test"))
}

fn is_restricted_allow_attr(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("allow") {
        return false;
    }

    let mut restricted = false;
    // parse_nested_meta handles comma-separated lists like #[allow(a, b, c)]
    let _ = attr.parse_nested_meta(|meta| {
        use quote::ToTokens;
        let path_str = meta.path.to_token_stream().to_string().replace(' ', "");
        if path_str == "clippy::unwrap_used" || path_str == "clippy::expect_used" {
            restricted = true;
        }
        Ok(())
    });
    restricted
}

#[cfg(test)]
#[path = "no_allow_outside_test_test.rs"]
mod tests;
