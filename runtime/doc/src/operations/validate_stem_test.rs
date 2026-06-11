#![allow(clippy::unwrap_used)]

use super::{
    BareStemMatch, DocumentStemIndex, find_bare_governed_stems, protected_ranges_for_line,
};

#[test]
fn test_find_bare_governed_stems_detects_unlinked_stem_in_body() {
    let index = DocumentStemIndex { stems: vec!["rfc-00001-example".to_owned()] };
    let content = "---\ntitle: Test\n---\n\nSee rfc-00001-example for details.\n";
    let matches = find_bare_governed_stems(content, &index);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].stem, "rfc-00001-example");
    assert_eq!(matches[0].replacement(), "[[rfc-00001-example]]");
}

#[test]
fn test_find_bare_governed_stems_skips_already_linked_stem() {
    let index = DocumentStemIndex { stems: vec!["rfc-00001-example".to_owned()] };
    let content = "---\ntitle: Test\n---\n\nSee [[rfc-00001-example]] for details.\n";
    let matches = find_bare_governed_stems(content, &index);
    assert!(matches.is_empty(), "linked stem should not be flagged");
}

#[test]
fn test_find_bare_governed_stems_skips_stem_in_code_span() {
    let index = DocumentStemIndex { stems: vec!["rfc-00001-example".to_owned()] };
    let content = "---\ntitle: Test\n---\n\nRun `rfc-00001-example` to test.\n";
    let matches = find_bare_governed_stems(content, &index);
    assert!(matches.is_empty(), "stem inside backtick code span should not be flagged");
}

#[test]
fn test_find_bare_governed_stems_skips_stem_in_fenced_code_block() {
    let index = DocumentStemIndex { stems: vec!["rfc-00001-example".to_owned()] };
    let content = "---\ntitle: Test\n---\n\n```\nrfc-00001-example\n```\n\nSome normal text.\n";
    let matches = find_bare_governed_stems(content, &index);
    assert!(matches.is_empty(), "stem inside fenced code block should not be flagged");
}

#[test]
fn test_find_bare_governed_stems_skips_frontmatter() {
    let index = DocumentStemIndex { stems: vec!["rfc-00001-example".to_owned()] };
    let content = "---\nslug: rfc-00001-example\n---\n\nBody without any stem.\n";
    let matches = find_bare_governed_stems(content, &index);
    assert!(matches.is_empty(), "stem in frontmatter should not be flagged");
}

#[test]
fn test_bare_stem_match_accessors() {
    let m = BareStemMatch {
        stem: "rfc-00001".to_owned(),
        replacement: "[[rfc-00001]]".to_owned(),
        start: 5,
        end: 14,
    };
    assert_eq!(m.replacement(), "[[rfc-00001]]");
    assert_eq!(m.start(), 5);
    assert_eq!(m.end(), 14);
}

#[test]
fn test_protected_ranges_for_line_identifies_wikilink_ranges() {
    let line = "See [[rfc-00001-example]] here.";
    let ranges = protected_ranges_for_line(line);
    assert!(!ranges.is_empty(), "wikilink should be in protected ranges");
    let (start, end) = ranges[0];
    assert!(start <= 4);
    assert!(end >= 25);
}
