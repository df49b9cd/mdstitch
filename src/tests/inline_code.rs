use super::*;

// ===========================================================================
// Inline code
// ===========================================================================

#[test]
fn inline_code_incomplete() {
    assert_eq!(r("`code").as_ref(), "`code`");
}

#[test]
fn inline_code_complete() {
    assert_eq!(r("`code`").as_ref(), "`code`");
}

#[test]
fn inline_code_empty() {
    assert_eq!(r("`").as_ref(), "`");
}
