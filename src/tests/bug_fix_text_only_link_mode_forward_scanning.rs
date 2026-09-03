use super::*;

// ===========================================================================
// Bug fix: text-only link mode forward scanning
// ===========================================================================

#[test]
fn text_only_link_with_preceding_complete_link() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    // The first link is complete; only the second bracket should be stripped.
    assert_eq!(
        stitch("[done](http://ok) and [incomplete", &opts).as_ref(),
        "[done](http://ok) and incomplete"
    );
}

#[test]
fn text_only_nested_brackets() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    // Both [outer and [inner are incomplete — all stripped in one pass for idempotency.
    assert_eq!(
        stitch("Text [outer [inner", &opts).as_ref(),
        "Text outer inner"
    );
}
