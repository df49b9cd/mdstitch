use super::*;

// ===========================================================================
// Italic (single asterisk)
// ===========================================================================

#[test]
fn italic_asterisk_incomplete() {
    assert_eq!(r("Text with *italic").as_ref(), "Text with *italic*");
}

#[test]
fn italic_asterisk_at_start() {
    assert_eq!(r("*incomplete").as_ref(), "*incomplete*");
}

#[test]
fn italic_asterisk_complete() {
    assert_eq!(
        r("Text with *italic text*").as_ref(),
        "Text with *italic text*"
    );
}

#[test]
fn italic_asterisk_with_bold() {
    assert_eq!(r("**bold** and *italic").as_ref(), "**bold** and *italic*");
}

#[test]
fn italic_asterisk_word_internal_digits() {
    assert_eq!(r("234234*123").as_ref(), "234234*123");
}

#[test]
fn italic_asterisk_word_internal_letters() {
    assert_eq!(r("hello*world").as_ref(), "hello*world");
}

#[test]
fn italic_asterisk_word_internal_mixed() {
    assert_eq!(r("test*123*test").as_ref(), "test*123*test");
}

#[test]
fn italic_asterisk_with_var_names() {
    assert_eq!(
        r("*italic with some*var*name inside").as_ref(),
        "*italic with some*var*name inside*"
    );
}

#[test]
fn italic_asterisk_complete_word() {
    assert_eq!(r("*word* and more text").as_ref(), "*word* and more text");
}
