use quote::ToTokens;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint_rules::rule::{Rule, RuleViolation};

/// [RULE-14] No direct `std::process::Command` — use `io::CommandBuilder` or other repository abstractions instead.
///
/// Direct process spawning via `std` is discouraged to ensure:
/// 1. Command output is captured/logged consistently.
/// 2. Testing mocks (via `io::stub_shell`) can intercept the execution.
///
/// Files inside `xtask/src/` are exempted.
pub struct NoStdProcessCommand;

impl Rule for NoStdProcessCommand {
    fn is_active(&self, _future: bool) -> bool {
        true
    }

    fn check_rust(&self, path: &Path, ast: &syn::File, _raw: &str, out: &mut Vec<RuleViolation>) {
        // Exempt xtask/src/ and runtime/io/src/shell.rs
        let path_str = path.to_string_lossy().replace('\\', "/");
        if path_str.contains("xtask/src") || path_str.contains("runtime/io/src/shell.rs") {
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

impl Visit<'_> for Visitor<'_> {
    fn visit_item_use(&mut self, node: &syn::ItemUse) {
        let use_str = node.to_token_stream().to_string();
        // Quote serializes as "std :: process :: Command"
        if use_str.contains("std :: process :: Command") || use_str.contains("std :: process :: *")
        {
            self.push_violation(
                node.span(),
                "direct use of std::process::Command found — use io::CommandBuilder instead",
            );
        }
        visit::visit_item_use(self, node);
    }

    fn visit_expr_path(&mut self, node: &syn::ExprPath) {
        let path_str = node.to_token_stream().to_string();
        if path_str.contains("std :: process :: Command :: new") || path_str == "Command :: new" {
            self.push_violation(
                node.span(),
                "direct call to std::process::Command::new found — use io::CommandBuilder instead",
            );
        }
        visit::visit_expr_path(self, node);
    }

    fn visit_expr_call(&mut self, node: &syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = &*node.func {
            let path_str = expr_path.to_token_stream().to_string();
            if path_str.contains("std :: process :: Command :: new") || path_str == "Command :: new"
            {
                self.push_violation(expr_path.span(), "direct call to std::process::Command::new found — use io::CommandBuilder instead");
            }
        }
        visit::visit_expr_call(self, node);
    }
}

impl Visitor<'_> {
    fn push_violation(&mut self, span: proc_macro2::Span, message: &str) {
        let start = span.start();
        self.out.push(RuleViolation {
            file: self.path.to_path_buf(),
            line: Some(start.line as u32),
            column: Some(start.column as u32 + 1),
            rule_id: "RULE-14",
            message: message.to_string(),
        });
    }
}

#[cfg(test)]
#[path = "no_std_process_command_test.rs"]
mod tests;
