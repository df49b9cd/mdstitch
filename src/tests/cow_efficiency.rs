use super::*;

// ===========================================================================
// Cow efficiency
// ===========================================================================

#[test]
fn cow_borrowed_for_complete_markdown() {
    let text = "Hello **bold** and *italic* and `code` done.";
    assert!(matches!(r(text), Cow::Borrowed(_)));
}

#[test]
fn cow_borrowed_for_plain_text() {
    assert!(matches!(r("just plain text"), Cow::Borrowed(_)));
}
