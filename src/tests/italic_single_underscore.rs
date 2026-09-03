use super::*;

// ===========================================================================
// Italic (single underscore)
// ===========================================================================

#[test]
fn italic_underscore_incomplete() {
    assert_eq!(r("Text with _italic").as_ref(), "Text with _italic_");
}

#[test]
fn italic_underscore_at_start() {
    assert_eq!(r("_incomplete").as_ref(), "_incomplete_");
}

#[test]
fn italic_underscore_complete() {
    assert_eq!(
        r("Text with _italic text_").as_ref(),
        "Text with _italic text_"
    );
}

#[test]
fn italic_underscore_with_bold() {
    assert_eq!(r("__bold__ and _italic").as_ref(), "__bold__ and _italic_");
}

#[test]
fn italic_underscore_word_internal_cafe() {
    assert_eq!(r("café_price").as_ref(), "café_price");
}

#[test]
fn italic_underscore_word_internal_naive() {
    assert_eq!(r("naïve_approach").as_ref(), "naïve_approach");
}

#[test]
fn italic_underscore_word_internal_variable() {
    assert_eq!(r("some_variable_name").as_ref(), "some_variable_name");
}

#[test]
fn italic_underscore_word_internal_digits() {
    assert_eq!(r("test_123_value").as_ref(), "test_123_value");
}

#[test]
fn italic_underscore_with_var_names() {
    assert_eq!(
        r("_italic with some_var_name inside").as_ref(),
        "_italic with some_var_name inside_"
    );
}

#[test]
fn italic_underscore_trailing_newline() {
    assert_eq!(r("Text with _italic\n").as_ref(), "Text with _italic_\n");
}

#[test]
fn italic_underscore_trailing_double_newline() {
    assert_eq!(r("_incomplete\n\n").as_ref(), "_incomplete_\n\n");
}
