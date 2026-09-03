use super::*;

// ===========================================================================
// Strikethrough
// ===========================================================================

#[test]
fn strikethrough_incomplete() {
    assert_eq!(r("Text with ~~strike").as_ref(), "Text with ~~strike~~");
}

#[test]
fn strikethrough_at_start() {
    assert_eq!(r("~~incomplete").as_ref(), "~~incomplete~~");
}

#[test]
fn strikethrough_complete() {
    assert_eq!(
        r("~~strikethrough text~~").as_ref(),
        "~~strikethrough text~~"
    );
}

#[test]
fn strikethrough_multiple_complete() {
    assert_eq!(
        r("~~strike1~~ and ~~strike2~~").as_ref(),
        "~~strike1~~ and ~~strike2~~"
    );
}

#[test]
fn strikethrough_odd() {
    assert_eq!(
        r("~~first~~ and ~~second").as_ref(),
        "~~first~~ and ~~second~~"
    );
}

#[test]
fn strikethrough_half_close() {
    assert_eq!(r("~~xxx~").as_ref(), "~~xxx~~");
}

#[test]
fn strikethrough_half_close_phrase() {
    assert_eq!(r("~~strike text~").as_ref(), "~~strike text~~");
}
