use super::*;

// ===========================================================================
// Comparison operators
// ===========================================================================

#[test]
fn comparison_in_list() {
    assert_eq!(r("- > 25").as_ref(), "- \\> 25");
}

#[test]
fn comparison_gte_in_list() {
    assert_eq!(r("- >= 25").as_ref(), "- \\>= 25");
}

#[test]
fn comparison_ordered_list() {
    assert_eq!(r("1. > 25").as_ref(), "1. \\> 25");
}

#[test]
fn comparison_not_blockquote() {
    // Not followed by digit — not a comparison.
    assert_eq!(r("- > text").as_ref(), "- > text");
}

#[test]
fn comparison_not_in_list() {
    assert_eq!(r("> 25").as_ref(), "> 25");
}
