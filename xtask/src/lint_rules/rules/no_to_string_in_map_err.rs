//! RULE-12: No `.to_string()` in `.map_err()`.
//!
//! Flag `.map_err(|e| e.to_string())` anti-patterns — use proper error types
//! and mapping instead of converting to a raw String error.

use std::path::Path;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

pub struct NoToStringInMapErr;

impl Rule for NoToStringInMapErr {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        let mut visitor = Visitor { path, out };
        visitor.visit_file(ast);
    }
}

// ─── visitor ────────────────────────────────────────────────────────────────

struct Visitor<'a> {
    path: &'a Path,
    out: &'a mut Vec<RuleViolation>,
}

impl Visit<'_> for Visitor<'_> {
    fn visit_expr_method_call(&mut self, node: &syn::ExprMethodCall) {
        if node.method == "map_err"
            && node.args.len() == 1
            && let Some(arg) = node.args.first()
            && contains_to_string_call(arg)
        {
            let span = node.span();
            self.out.push(RuleViolation {
                file: self.path.to_path_buf(),
                line: Some(span.start().line as u32),
                column: Some(span.start().column as u32 + 1),
                rule_id: "RULE-12",
                message: ".map_err(|e| e.to_string()) found — use idiomatic error mapping or explicit error types instead of converting to String".into(),
            });
        }
        visit::visit_expr_method_call(self, node);
    }
}

fn contains_to_string_call(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Closure(closure) => {
            // Check closure body for .to_string()
            is_to_string_call(&closure.body)
        }
        syn::Expr::Path(path) if path.path.is_ident("to_string") => {
            // .map_err(to_string) -- rare but possible if to_string was a standalone fn
            true
        }
        _ => false,
    }
}

fn is_to_string_call(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(m) if m.method == "to_string" => true,
        // Also check if it's wrapped in a block: `|e| { e.to_string() }`
        syn::Expr::Block(b) => {
            if let Some(syn::Stmt::Expr(e, _)) = b.block.stmts.last() {
                is_to_string_call(e)
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "no_to_string_in_map_err_test.rs"]
mod tests;
