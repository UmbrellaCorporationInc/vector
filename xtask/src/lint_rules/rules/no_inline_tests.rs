//! RULE-13: Canonical Test Module Contract.
//!
//! Four conditions are checked:
//!
//! - **RULE-13A**: `#[cfg(test)]` appears in a non-`*_test.rs` file (inline test module).
//! - **RULE-13B**: the test module identifier is not exactly `tests`.
//! - **RULE-13C**: the `#[path]` attribute value is not `<file_stem>_test.rs`.
//! - **RULE-13D**: no `#[cfg(test)]` mod exists at file level — standard enforcement.
//!   Exempt: `main.rs`, `lib.rs`, `mod.rs`, `*_test.rs`, files under `tests/`.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::lint_rules::rule::{Rule, RuleViolation};

pub struct NoInlineTests;

impl Rule for NoInlineTests {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        let mut visitor = Visitor { path, out };
        visitor.visit_file(ast);

        // RULE-13D: every source file must declare at least one #[cfg(test)] mod.
        let is_test_file = path.to_string_lossy().ends_with("_test.rs");
        let file_name = path.file_name().map(|n| n.to_string_lossy());
        let is_main = file_name.as_deref() == Some("main.rs");
        let is_lib = file_name.as_deref() == Some("lib.rs");
        let is_mod = file_name.as_deref() == Some("mod.rs");
        // Cargo integration test files live under a `tests/` directory at crate
        // root. They are not source modules — they cannot declare `mod tests`.
        let is_integration_test = path.components().any(|c| c.as_os_str() == "tests");

        if !is_test_file && !is_main && !is_lib && !is_mod && !is_integration_test {
            let has_test_mod = ast.items.iter().any(|item| {
                if let syn::Item::Mod(m) = item { has_cfg_test(&m.attrs) } else { false }
            });
            if !has_test_mod {
                out.push(RuleViolation {
                    file: path.to_path_buf(),
                    line: None,
                    column: None,
                    rule_id: "RULE-13D",
                    message: format!(
                        "`{}` has no `#[cfg(test)] mod tests` — create a sibling `{}_test.rs` add tests `#[test]` fn (not empty functions, real tests), then declare `#[cfg(test)] #[path = \"{}_test.rs\"] mod tests;` in this file",
                        path.display(),
                        file_stem(path),
                        file_stem(path),
                    ),
                });
            }
        }
    }
}

// ─── visitor ────────────────────────────────────────────────────────────────

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
}

impl Visit<'_> for Visitor<'_> {
    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        if !has_cfg_test(&node.attrs) {
            // Not a test module — recurse in case there are nested mods.
            syn::visit::visit_item_mod(self, node);
            return;
        }

        let span = node.span();
        let line = Some(span.start().line as u32);
        let column = Some(span.start().column as u32 + 1);
        let file_stem = file_stem(self.path);
        let is_test_file = self.path.to_string_lossy().ends_with("_test.rs");

        // RULE-13A: cfg(test) mod with an inline body in a non-test file.
        // An inline body means `mod tests { ... }` (content is Some). The canonical form
        // `mod tests;` (content is None, with a #[path] attr) is NOT a violation.
        let has_inline_body = node.content.is_some();
        if !is_test_file && has_inline_body {
            self.out.push(RuleViolation {
                file: self.path.to_path_buf(),
                line,
                column,
                rule_id: "RULE-13A",
                message: format!(
                    "#[cfg(test)] mod found in `{}` — move tests to a sibling `*_test.rs` file",
                    self.path.display()
                ),
            });
        }

        // RULE-13B: identifier is not `tests`.
        let mod_name = node.ident.to_string();
        if mod_name != "tests" {
            self.out.push(RuleViolation {
                file: self.path.to_path_buf(),
                line,
                column,
                rule_id: "RULE-13B",
                message: format!("#[cfg(test)] mod is named `{mod_name}` — rename it to `tests`"),
            });
        }

        // RULE-13C: #[path] value is not `<stem>_test.rs`.
        if let Some(path_value) = extract_path_attr(&node.attrs) {
            let expected = format!("{file_stem}_test.rs");
            if path_value != expected {
                self.out.push(RuleViolation {
                    file: self.path.to_path_buf(),
                    line,
                    column,
                    rule_id: "RULE-13C",
                    message: format!(
                        "#[path = \"{path_value}\"] must be `\"{expected}\"` (got `\"{path_value}\"`)"
                    ),
                });
            }
        }

        // Do not recurse into the test module body — rules target the mod item itself.
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Returns `true` if the attribute list contains `#[cfg(test)]`.
fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        // Accept both `#[cfg(test)]` (parsed as a Meta list containing the ident `test`)
        // and malformed attrs where we fall back to token-stream comparison.
        match &attr.meta {
            syn::Meta::List(list) => {
                let tokens = list.tokens.to_string();
                tokens.trim() == "test"
            }
            _ => false,
        }
    })
}

/// Extracts the string value of `#[path = "..."]`, if present.
fn extract_path_attr(attrs: &[syn::Attribute]) -> Option<String> {
    attrs.iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value
        {
            return Some(s.value());
        }
        None
    })
}

/// Returns the file stem (filename without extension) as a `String`.
fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[cfg(test)]
#[path = "no_inline_tests_test.rs"]
mod tests;
