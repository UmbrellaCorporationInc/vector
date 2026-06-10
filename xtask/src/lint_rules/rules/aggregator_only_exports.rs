//! RULE-27: Aggregator files must contain only re-exports.
//!
//! `lib.rs` and `mod.rs` serve as the public surface of a crate or
//! subdirectory module. Their only valid top-level items are:
//!
//! - `mod` declarations
//! - `use` items (including `pub use` and `pub(crate) use`)
//! - `extern crate` declarations
//! - `syn::Item::Verbatim` (macro-generated or raw tokens)
//!
//! Any other item — functions, structs, enums, type aliases, constants,
//! statics, traits, or inherent `impl` blocks — blurs the aggregator /
//! implementation boundary.
//!
//! **Activation:** standard (always enforced).

use std::path::Path;

use syn::spanned::Spanned;

use crate::lint_rules::rule::{Rule, RuleViolation};

pub struct AggregatorOnlyExports;

impl Rule for AggregatorOnlyExports {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        if !is_aggregator(path) {
            return;
        }

        for item in &ast.items {
            if let Some(forbidden) = forbidden_item_kind(item) {
                let loc = forbidden.span.start();
                out.push(RuleViolation {
                    file: path.to_path_buf(),
                    line: Some(loc.line as u32),
                    column: Some(loc.column as u32 + 1),
                    rule_id: "RULE-27",
                    message: format!(
                        "`{}:{}` defines `{}` — aggregator files must contain only `mod` and \
                         `use` declarations; move implementation to a dedicated module",
                        path.display(),
                        loc.line,
                        forbidden.kind,
                    ),
                });
            }
        }
    }
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Metadata about a forbidden item in an aggregator file.
struct ForbiddenItem {
    kind: &'static str,
    span: proc_macro2::Span,
}

/// Returns `true` when the file is an aggregator (`lib.rs` or `mod.rs`).
fn is_aggregator(path: &Path) -> bool {
    matches!(path.file_name().and_then(|n| n.to_str()), Some("lib.rs" | "mod.rs"))
}

/// Returns metadata about a forbidden item if the item is not permitted in an
/// aggregator, or `None` if the item is allowed.
fn forbidden_item_kind(item: &syn::Item) -> Option<ForbiddenItem> {
    let (kind, span) = match item {
        // Permitted items — aggregator-safe.
        syn::Item::Mod(_)
        | syn::Item::Use(_)
        | syn::Item::ExternCrate(_)
        | syn::Item::Verbatim(_) => {
            return None;
        }
        // Forbidden items — implementation belongs in a dedicated module.
        syn::Item::Fn(i) => ("fn", i.span()),
        syn::Item::Struct(i) => ("struct", i.span()),
        syn::Item::Enum(i) => ("enum", i.span()),
        syn::Item::Type(i) => ("type", i.span()),
        syn::Item::Const(i) => ("const", i.span()),
        syn::Item::Static(i) => ("static", i.span()),
        syn::Item::Trait(i) => ("trait", i.span()),
        syn::Item::TraitAlias(i) => ("trait alias", i.span()),
        syn::Item::Impl(i) => ("impl", i.span()),
        syn::Item::Macro(i) => ("macro", i.span()),
        syn::Item::Union(i) => ("union", i.span()),
        // Catch-all for future syn variants.
        _ => return None,
    };
    Some(ForbiddenItem { kind, span })
}

#[cfg(test)]
#[path = "aggregator_only_exports_test.rs"]
mod tests;
