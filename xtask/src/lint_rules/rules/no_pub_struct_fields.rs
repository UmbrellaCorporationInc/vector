//! RULE-10: No `pub` struct fields outside DTOs.
//!
//! Enforces encapsulation by flagging public struct fields.
//! Fields are allowed to be public if:
//! 1. The struct doc comment contains a `# DTO` section.
//! 2. The entire file is a test context (*_test.rs or tests/ directory).

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

pub(crate) struct NoPubStructFields;

impl Rule for NoPubStructFields {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let is_test_context = path_str.ends_with("_test.rs") || path_str.contains("/tests/");

        if is_test_context {
            return;
        }

        let mut visitor = Visitor { path, out };
        visitor.visit_file(ast);
    }
}

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
}

impl<'a> Visit<'_> for Visitor<'a> {
    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        if is_dto_struct(node) {
            return;
        }

        for field in &node.fields {
            if let syn::Visibility::Public(_) = field.vis {
                let span = field.span();
                let field_name = field
                    .ident
                    .as_ref()
                    .map(|i| i.to_string())
                    .unwrap_or_else(|| "tuple field".to_string());

                self.out.push(RuleViolation {
                    file: self.path.to_path_buf(),
                    line: Some(span.start().line as u32),
                    column: Some(span.start().column as u32 + 1),
                    rule_id: "RULE-10",
                    message: format!(
                        "public field `{field_name}` in struct `{}` found — use private fields with getters/setters or pub(crate) visibility. For Data Transfer Objects, add a `# DTO(explain why)` section to the struct doc comment to exempt it.",
                        node.ident
                    ),
                });
            }
        }

        visit::visit_item_struct(self, node);
    }
}

fn is_dto_struct(node: &syn::ItemStruct) -> bool {
    node.attrs.iter().any(|attr| {
        if !attr.path().is_ident("doc") {
            return false;
        }

        if let syn::Meta::NameValue(nv) = &attr.meta
            && let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value
        {
            let val = s.value();
            if let Some(start) = val.find("# DTO(") {
                let rest = &val[start + 6..];
                if let Some(end) = rest.find(')') {
                    let reason = rest[..end].trim();
                    return !reason.is_empty();
                }
            }
        }
        false
    })
}

#[cfg(test)]
#[path = "no_pub_struct_fields_test.rs"]
mod tests;
