use super::*;

// ===========================================================================
// Bold
// ===========================================================================

#[test]
fn bold_incomplete() {
    assert_eq!(r("Text with **bold").as_ref(), "Text with **bold**");
}

#[test]
fn bold_incomplete_at_start() {
    assert_eq!(r("**incomplete").as_ref(), "**incomplete**");
}

#[test]
fn bold_complete() {
    assert_eq!(
        r("Text with **bold text**").as_ref(),
        "Text with **bold text**"
    );
}

#[test]
fn bold_multiple_complete() {
    assert_eq!(
        r("**bold1** and **bold2**").as_ref(),
        "**bold1** and **bold2**"
    );
}

#[test]
fn bold_odd_markers() {
    assert_eq!(
        r("**first** and **second").as_ref(),
        "**first** and **second**"
    );
}

#[test]
fn bold_partial_boundary() {
    assert_eq!(
        r("Here is some **bold tex").as_ref(),
        "Here is some **bold tex**"
    );
}

#[test]
fn bold_half_close_simple() {
    assert_eq!(r("**xxx*").as_ref(), "**xxx**");
}

#[test]
fn bold_half_close_phrase() {
    assert_eq!(r("**bold text*").as_ref(), "**bold text**");
}

#[test]
fn bold_half_close_sentence() {
    assert_eq!(r("Text with **bold*").as_ref(), "Text with **bold**");
}

#[test]
fn bold_half_close_full() {
    assert_eq!(r("This is **bold text*").as_ref(), "This is **bold text**");
}
