#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn reader_alias_compiles() {
    let reader: Reader = Box::new(std::io::Cursor::new(Vec::<u8>::new()));
    let _ = reader;
}
