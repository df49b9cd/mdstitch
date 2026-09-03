use super::*;

// ===========================================================================
// Setext headings
// ===========================================================================

#[test]
fn setext_heading_dash() {
    assert_eq!(r("Heading\n-").as_ref(), "Heading\n-\u{200B}");
}

#[test]
fn setext_heading_double_dash() {
    assert_eq!(r("Heading\n--").as_ref(), "Heading\n--\u{200B}");
}

#[test]
fn setext_heading_equals() {
    assert_eq!(r("Heading\n=").as_ref(), "Heading\n=\u{200B}");
}

#[test]
fn setext_heading_triple_dash_unchanged() {
    // Three dashes is a valid horizontal rule.
    assert_eq!(r("Heading\n---").as_ref(), "Heading\n---");
}

#[test]
fn setext_heading_four_space_indent_is_code_block() {
    assert_eq!(r("Head\n    -").as_ref(), "Head\n    -");
}

#[test]
fn setext_heading_three_space_indent_fires() {
    assert_eq!(r("Head\n   -").as_ref(), "Head\n   -\u{200B}");
}

#[test]
fn setext_heading_tab_indent_is_code_block() {
    assert_eq!(r("Head\n\t-").as_ref(), "Head\n\t-");
}

#[test]
fn setext_heading_blank_line_between_unchanged() {
    assert_eq!(r("a\n\n-").as_ref(), "a\n\n-");
}
