use super::*;

#[test]
fn test_bytes_alias_compiles() {
    let b: Bytes = vec![1, 2, 3];
    assert_eq!(b.len(), 3);
}
