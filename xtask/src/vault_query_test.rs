#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_yaml::{Mapping, Value};

use super::*;

#[test]
fn parses_or_across_groups() {
    let q = parse_query_expression("{type='task'}{type='adr'}").unwrap();
    assert_eq!(q.groups.len(), 2);
    assert_eq!(
        q.groups[0].predicates,
        vec![Predicate::Eq { field: "type".into(), value: "task".into() }]
    );
    assert_eq!(
        q.groups[1].predicates,
        vec![Predicate::Eq { field: "type".into(), value: "adr".into() }]
    );
}

#[test]
fn parses_and_inside_group() {
    let q = parse_query_expression("{type='task' tag=['urgent']}").unwrap();
    assert_eq!(q.groups.len(), 1);
    assert_eq!(q.groups[0].predicates.len(), 2);
}

#[test]
fn parses_substring() {
    let q = parse_query_expression("{description@='foo'}").unwrap();
    assert_eq!(
        q.groups[0].predicates[0],
        Predicate::Substring { field: "description".into(), needle: "foo".into() }
    );
}

#[test]
fn rejects_unclosed_brace() {
    assert!(parse_query_expression("{type='x'").is_err());
}

#[test]
fn rejects_empty_group() {
    assert!(parse_query_expression("{}").is_err());
}

#[test]
fn list_allows_empty() {
    let q = parse_query_expression("{tag=[]}").unwrap();
    match &q.groups[0].predicates[0] {
        Predicate::InList { values, .. } => assert!(values.is_empty()),
        _ => unreachable!("expected InList"),
    }
}

#[test]
fn eval_eq_scalar() {
    let mut m = Mapping::new();
    m.insert(Value::String("type".into()), Value::String("guide".into()));
    let q = parse_query_expression("{type='guide'}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn eval_or_across_groups() {
    let mut m = Mapping::new();
    m.insert(Value::String("type".into()), Value::String("adr".into()));
    let q = parse_query_expression("{type='task'}{type='adr'}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn eval_in_list_matches_tag_sequence() {
    let mut m = Mapping::new();
    m.insert(
        Value::String("tags".into()),
        Value::Sequence(vec![Value::String("a".into()), Value::String("b".into())]),
    );
    let q = parse_query_expression("{tags=['b']}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn eval_substring_on_description() {
    let mut m = Mapping::new();
    m.insert(Value::String("description".into()), Value::String("hello world".into()));
    let q = parse_query_expression("{description@='world'}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn parses_list_and_with_nested_braces() {
    let q = parse_query_expression("{type='adr' tag={'babel','emitter'}}").unwrap();
    assert_eq!(q.groups.len(), 1);
    assert_eq!(q.groups[0].predicates.len(), 2);
    assert_eq!(
        q.groups[0].predicates[1],
        Predicate::AllInList {
            field: "tag".into(),
            values: vec!["babel".into(), "emitter".into()],
        }
    );
}

#[test]
fn eval_all_in_list_requires_every_tag() {
    let mut m = Mapping::new();
    m.insert(
        Value::String("tags".into()),
        Value::Sequence(vec![Value::String("a".into()), Value::String("b".into())]),
    );
    let q = parse_query_expression("{tags={'a','b'}}").unwrap();
    assert!(eval_query_expr(&q, &m));
    let q2 = parse_query_expression("{tags={'a','c'}}").unwrap();
    assert!(!eval_query_expr(&q2, &m));
}

#[test]
fn eval_all_in_list_on_scalar_string() {
    let mut m = Mapping::new();
    m.insert(Value::String("tags".into()), Value::String("a".into()));
    let q = parse_query_expression("{tags={'a'}}").unwrap();
    assert!(eval_query_expr(&q, &m));
    let q2 = parse_query_expression("{tags={'a','b'}}").unwrap();
    assert!(!eval_query_expr(&q2, &m));
}

#[test]
fn eval_or_group_with_list_and() {
    let mut m = Mapping::new();
    m.insert(Value::String("type".into()), Value::String("adr".into()));
    m.insert(
        Value::String("tag".into()),
        Value::Sequence(vec![Value::String("x".into()), Value::String("y".into())]),
    );
    let q = parse_query_expression("{type='task'}{type='adr' tag={'x','y'}}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn empty_or_list_matches_nothing_when_field_present() {
    let mut m = Mapping::new();
    m.insert(Value::String("tag".into()), Value::Sequence(vec![Value::String("a".into())]));
    let q = parse_query_expression("{tag=[]}").unwrap();
    assert!(!eval_query_expr(&q, &m));
}

#[test]
fn empty_and_list_matches_when_field_present() {
    let mut m = Mapping::new();
    m.insert(Value::String("tag".into()), Value::Sequence(vec![Value::String("a".into())]));
    let q = parse_query_expression("{tag={}}").unwrap();
    assert!(eval_query_expr(&q, &m));
}

#[test]
fn empty_and_list_does_not_match_missing_field() {
    let m = Mapping::new();
    let q = parse_query_expression("{tag={}}").unwrap();
    assert!(!eval_query_expr(&q, &m));
}
