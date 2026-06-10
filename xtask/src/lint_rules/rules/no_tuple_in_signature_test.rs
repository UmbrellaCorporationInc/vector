#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use std::path::Path;

use super::NoTupleInSignature;
use crate::lint_rules::rule::{Rule, RuleViolation};

// ─── helpers ────────────────────────────────────────────────────────────────

fn check(path: &str, source: &str) -> Vec<RuleViolation> {
    let ast = syn::parse_file(source).expect("test source must parse");
    let mut out = Vec::new();
    NoTupleInSignature.check_rust(Path::new(path), &ast, source, &mut out);
    out
}

fn rule_ids(violations: &[RuleViolation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule_id).collect()
}

// ─── RULE-5A: Tuples in signatures ──────────────────────────────────────────

#[test]
fn rule_5a_fires_for_tuple_in_parameters() {
    let source = r"
        fn foo(x: (i32, i32)) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5A"), "expected RULE-5A, got: {ids:?}");
}

#[test]
fn rule_5a_fires_for_tuple_in_return_type() {
    let source = r"
        fn foo() -> (i32, i32) { (1, 2) }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5A"), "expected RULE-5A, got: {ids:?}");
}

#[test]
fn rule_5a_fires_for_nested_tuple() {
    let source = r"
        fn foo(x: Vec<(i32, i32)>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5A"), "expected RULE-5A (nested), got: {ids:?}");
}

#[test]
fn rule_5a_does_not_fire_for_single_element_tuple() {
    let source = r"
        fn foo(x: (i32,)) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5A"), "RULE-5A must not fire for 1-element tuple, got: {ids:?}");
}

#[test]
fn rule_5a_does_not_fire_in_test_file() {
    let source = r"
        fn foo(x: (i32, i32)) {}
    ";
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-5A must not fire in _test.rs, got: {ids:?}");
}

// ─── RULE-5B: Public complex generics ───────────────────────────────────────

#[test]
fn rule_5b_fires_for_pub_fn_with_2_type_params() {
    let source = r"
        pub fn foo(x: std::collections::HashMap<String, String>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5B"), "expected RULE-5B, got: {ids:?}");
}

#[test]
fn rule_5b_does_not_fire_for_private_fn() {
    let source = r"
        fn foo(x: std::collections::HashMap<String, String>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5B"), "RULE-5B must not fire for private fn, got: {ids:?}");
}

#[test]
fn rule_5b_does_not_fire_for_exempt_wrappers() {
    let source = r"
        pub fn foo(
            a: Arc<String>,
            m: Mutex<String>,
            r: Rc<String>,
            l: RwLock<String>,
            re: Result<String, String>,
            o: Option<String>
        ) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        ids.is_empty(),
        "RULE-5B must not fire for exempt wrappers (now including Result/Option), got: {ids:?}"
    );
}

#[test]
fn rule_5b_fires_for_complex_type_inside_exempt_wrapper() {
    let source = r"
        pub fn foo(a: Arc<(i32, i32)>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5A"), "expected RULE-5A for nested tuple in Arc, got: {ids:?}");
}

#[test]
fn rule_5b_fires_for_complex_generic_inside_exempt_wrapper() {
    let source = r"
        pub fn foo(a: Arc<std::collections::HashMap<String, String>>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5B"), "expected RULE-5B for nested HashMap in Arc, got: {ids:?}");
}

#[test]
fn rule_5b_fires_in_impl() {
    let source = r"
        impl Foo {
            pub fn foo(&self, x: std::collections::HashMap<String, String>) {}
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5B"), "expected RULE-5B in impl, got: {ids:?}");
}

#[test]
fn rule_5b_does_not_fire_for_type_alias_with_forwarded_type_params() {
    // `ObservedRequest<RQ, M>` is a type alias — its params are bare generic params
    // of the caller, not concrete types. RULE-5B must not fire.
    let source = r"
        pub fn emit(req: ObservedRequest<RQ, M>) {}
        pub fn emit_pair(req: ObservedRequest<RQ, M>, res: ObservedResponse<RS, M>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-5B"),
        "RULE-5B must not fire when all type args are bare type params, got: {ids:?}"
    );
}

#[test]
fn rule_5b_still_fires_for_two_concrete_type_args() {
    // HashMap<String, String> — both args are concrete types (mixed-case names), not type params.
    let source = r"
        pub fn foo(x: std::collections::HashMap<String, String>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5B"), "RULE-5B must still fire for two concrete type args, got: {ids:?}");
}

#[test]
fn rule_5b_does_not_fire_when_one_arg_is_concrete_and_one_is_type_param() {
    // HashMap<String, V> — only one concrete arg; rule requires 2+ concrete to fire.
    let source = r"
        pub fn foo<V>(x: std::collections::HashMap<String, V>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-5B"),
        "RULE-5B must not fire when only one arg is concrete, got: {ids:?}"
    );
}

// ─── RULE-5A / impl Trait interaction ───────────────────────────────────────

#[test]
fn rule_5a_does_not_fire_for_tuple_of_impl_traits() {
    // (impl Sender<T>, impl Receiver<T>) cannot be a type alias — must be exempt.
    let source = r"
        pub fn channel<T: Send + 'static>() -> (impl Sender<T>, impl Receiver<T>) {
            unimplemented!()
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(
        !ids.contains(&"RULE-5A"),
        "RULE-5A must not fire for a tuple of impl Trait, got: {ids:?}"
    );
}

#[test]
fn rule_5a_still_fires_for_tuple_of_concrete_types() {
    let source = r"
        pub fn foo() -> (String, i32) { unimplemented!() }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5A"), "RULE-5A must still fire for concrete tuple, got: {ids:?}");
}

// ─── RULE-5C: impl Trait with 2+ concrete generics ──────────────────────────

#[test]
fn rule_5c_fires_for_impl_trait_with_two_concrete_assoc_types() {
    // Hypothetical trait with two associated type bindings both concrete.
    let source = r"
        fn foo() -> impl Sink<Item = String, Error = MyError> { unimplemented!() }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5C"), "expected RULE-5C for two concrete assoc types, got: {ids:?}");
}

#[test]
fn rule_5c_fires_for_impl_fn_with_two_concrete_args() {
    let source = r"
        fn foo(f: impl Fn(String, i32) -> bool) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5C"), "expected RULE-5C for impl Fn(String, i32), got: {ids:?}");
}

#[test]
fn rule_5c_fires_for_impl_fn_with_concrete_input_and_output() {
    let source = r"
        fn foo(f: impl Fn(String) -> i32) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.contains(&"RULE-5C"), "expected RULE-5C for impl Fn(String) -> i32, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_for_impl_trait_no_args() {
    let source = r"
        fn foo(x: impl Display) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5C"), "RULE-5C must not fire for bare impl Trait, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_for_single_concrete_arg() {
    // impl Into<String> has only 1 concrete arg — idiomatic, should not fire.
    let source = r"
        fn foo(x: impl Into<String>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5C"), "RULE-5C must not fire for 1 concrete arg, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_for_single_concrete_assoc_type() {
    // impl Iterator<Item = String> has only 1 concrete binding — idiomatic.
    let source = r"
        fn foo() -> impl Iterator<Item = String> { std::iter::empty() }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5C"), "RULE-5C must not fire for 1 concrete assoc type, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_for_impl_trait_with_bare_type_param() {
    let source = r"
        fn foo<T>(x: impl Into<T>) {}
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5C"), "RULE-5C must not fire when arg is bare type param, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_for_impl_assoc_type_bare_param() {
    let source = r"
        fn foo<T>() -> impl Iterator<Item = T> { std::iter::empty() }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(!ids.contains(&"RULE-5C"), "RULE-5C must not fire for Item = T, got: {ids:?}");
}

#[test]
fn rule_5c_does_not_fire_in_test_file() {
    // Even with 2 concrete args, test files are exempt.
    let source = r"
        fn foo(f: impl Fn(String) -> i32) {}
    ";
    let violations = check("src/foo_test.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-5C must not fire in _test.rs, got: {ids:?}");
}

#[test]
fn rule_5a_does_not_fire_for_let_binding_destructuring() {
    let source = r"
        fn foo() {
            let (a, b) = (1, 2);
            let x: (i32, i32) = (3, 4);
        }
    ";
    let violations = check("src/foo.rs", source);
    let ids = rule_ids(&violations);
    assert!(ids.is_empty(), "RULE-5A must not fire for let bindings, got: {ids:?}");
}
