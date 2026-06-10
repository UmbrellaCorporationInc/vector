use super::NoStdProcessCommand;
use crate::lint_rules::rule::{Rule, RuleViolation};
use std::path::Path;

fn check_rule(path: &str, content: &str) -> Vec<RuleViolation> {
    let ast = syn::parse_file(content).expect("test source must parse");
    let mut violations = Vec::new();
    NoStdProcessCommand.check_rust(Path::new(path), &ast, content, &mut violations);
    violations
}

#[test]
fn rule_14_detects_direct_use_statement() {
    let content = r#"
        use std::process::Command;
        fn main() {}
    "#;
    let violations = check_rule("src/main.rs", content);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("direct use of std::process::Command"));
}

#[test]
fn rule_14_detects_wildcard_use_statement() {
    let content = r#"
        use std::process::*;
        fn main() {}
    "#;
    let violations = check_rule("src/main.rs", content);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("direct use of std::process::Command"));
}

#[test]
fn rule_14_detects_qualified_call() {
    let content = r#"
        fn main() {
            let _ = std::process::Command::new("ls").spawn();
        }
    "#;
    let violations = check_rule("src/main.rs", content);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("Command::new"));
}

#[test]
fn rule_14_detects_unqualified_call() {
    let content = r#"
        fn main() {
            let _ = Command::new("ls").spawn();
        }
    "#;
    let violations = check_rule("src/main.rs", content);
    assert!(!violations.is_empty());
    assert!(violations[0].message.contains("Command::new"));
}

#[test]
fn rule_14_exempts_xtask_src() {
    let content = r#"
        use std::process::Command;
        fn main() {
            Command::new("ls").spawn();
        }
    "#;
    let violations = check_rule("xtask/src/main.rs", content);
    assert!(violations.is_empty());
}

#[test]
fn rule_14_allows_io_command_builder() {
    let content = r#"
        use io::CommandBuilder;
        fn main() {
            CommandBuilder::new("ls").run();
        }
    "#;
    let violations = check_rule("src/main.rs", content);
    assert!(violations.is_empty());
}
