use super::*;

// ===========================================================================
// Italic (double underscores)
// ===========================================================================

#[test]
fn italic_double_underscore_incomplete() {
    assert_eq!(r("Text with __italic").as_ref(), "Text with __italic__");
}

#[test]
fn italic_double_underscore_at_start() {
    assert_eq!(r("__incomplete").as_ref(), "__incomplete__");
}

#[test]
fn italic_double_underscore_complete() {
    assert_eq!(
        r("Text with __italic text__").as_ref(),
        "Text with __italic text__"
    );
}

#[test]
fn italic_double_underscore_odd() {
    assert_eq!(
        r("__first__ and __second").as_ref(),
        "__first__ and __second__"
    );
}

#[test]
fn italic_double_underscore_half_close() {
    assert_eq!(r("__xxx_").as_ref(), "__xxx__");
}

#[test]
fn italic_double_underscore_half_close_phrase() {
    assert_eq!(r("__bold text_").as_ref(), "__bold text__");
}
