use super::*;

// ===========================================================================
// Bold-italic
// ===========================================================================

#[test]
fn bold_italic_incomplete() {
    assert_eq!(
        r("Text with ***bold-italic").as_ref(),
        "Text with ***bold-italic***"
    );
}

#[test]
fn bold_italic_at_start() {
    assert_eq!(r("***incomplete").as_ref(), "***incomplete***");
}

#[test]
fn bold_italic_complete() {
    assert_eq!(
        r("Text with ***bold and italic text***").as_ref(),
        "Text with ***bold and italic text***"
    );
}

#[test]
fn bold_italic_multiple_complete() {
    assert_eq!(
        r("***first*** and ***second***").as_ref(),
        "***first*** and ***second***"
    );
}

#[test]
fn bold_italic_odd() {
    assert_eq!(
        r("***first*** and ***second").as_ref(),
        "***first*** and ***second***"
    );
}

#[test]
fn bold_italic_four_asterisks_text() {
    assert_eq!(r("****").as_ref(), "****");
}

#[test]
fn bold_italic_five_asterisks() {
    assert_eq!(r("*****").as_ref(), "*****");
}

#[test]
fn bold_italic_trailing_asterisks_unchanged() {
    assert_eq!(r("text ***").as_ref(), "text ***");
    assert_eq!(r("text ****").as_ref(), "text ****");
    assert_eq!(r("text *****").as_ref(), "text *****");
}

#[test]
fn bold_italic_overlapping_302() {
    // Overlapping bold + italic: already balanced.
    assert_eq!(
        r("Combined **bold and *italic*** text").as_ref(),
        "Combined **bold and *italic*** text"
    );
}

#[test]
fn bold_italic_overlapping_already_complete() {
    assert_eq!(
        r("**bold and *italic*** more text").as_ref(),
        "**bold and *italic*** more text"
    );
}
