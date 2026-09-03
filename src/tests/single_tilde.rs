use super::*;

// ===========================================================================
// Single tilde
// ===========================================================================

#[test]
fn single_tilde_between_words() {
    assert_eq!(r("20~25").as_ref(), "20\\~25");
}

#[test]
fn single_tilde_double_unchanged() {
    assert_eq!(r("~~strike~~").as_ref(), "~~strike~~");
}

#[test]
fn single_tilde_at_boundary() {
    assert_eq!(r("~start").as_ref(), "~start");
    assert_eq!(r("end~").as_ref(), "end~");
}
