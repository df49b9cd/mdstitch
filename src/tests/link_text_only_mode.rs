use super::*;

// ===========================================================================
// Link text-only mode
// ===========================================================================

#[test]
fn link_text_only_mode() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    assert_eq!(
        stitch("Text with [incomplete link", &opts).as_ref(),
        "Text with incomplete link"
    );
}

#[test]
fn link_text_only_incomplete_url() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    assert_eq!(
        stitch("Visit [our site](https://exa", &opts).as_ref(),
        "Visit our site"
    );
}

#[test]
fn link_text_only_complete_unchanged() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    assert_eq!(
        stitch("[text](http://example.com)", &opts).as_ref(),
        "[text](http://example.com)"
    );
}

#[test]
fn link_text_only_image_removed() {
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    assert_eq!(stitch("Text ![incomplete image", &opts).as_ref(), "Text");
}

#[test]
fn link_text_only_rebuilds_ranges_after_bracket_strip() {
    // Regression: TextOnly mode strips the `[` byte mid-text, which shifts every
    // subsequent byte and invalidates pre-computed CodeBlockRanges. The italic
    // handler must see fresh ranges so it correctly classifies `*` as outside
    // the inline-code span and closes the emphasis.
    let opts = StitchOptions::default().link_mode(LinkMode::TextOnly);
    assert_eq!(stitch("[abc`def`*xyz", &opts).as_ref(), "abc`def`*xyz*");
}
