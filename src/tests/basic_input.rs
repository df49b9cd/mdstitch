use super::*;

// ===========================================================================
// Basic input
// ===========================================================================

#[test]
fn empty_string() {
    assert!(matches!(r(""), Cow::Borrowed(_)));
}

#[test]
fn plain_text() {
    assert_eq!(r("hello world").as_ref(), "hello world");
}

#[test]
fn strips_trailing_single_space() {
    assert_eq!(r("hello ").as_ref(), "hello");
}

#[test]
fn preserves_double_trailing_space() {
    assert_eq!(r("hello  ").as_ref(), "hello  ");
}
