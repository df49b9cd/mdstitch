use super::*;

// ===========================================================================
// Mixed formatting
// ===========================================================================

#[test]
fn mixed_all_complete() {
    let text = "**bold** and *italic* and `code` and ~~strike~~";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn mixed_bold_and_italic_incomplete() {
    assert_eq!(r("**bold and *italic").as_ref(), "**bold and *italic*");
}

#[test]
fn mixed_italic_with_bold() {
    assert_eq!(r("*italic with **bold").as_ref(), "*italic with **bold***");
}

#[test]
fn mixed_bold_with_code() {
    // inline_code now runs before emphasis: the backtick closes first, so the
    // bold markers close OUTSIDE the code span (the semantically correct
    // reading, and the idempotent one).
    assert_eq!(r("**bold with `code").as_ref(), "**bold with `code`**");
}

#[test]
fn mixed_strikethrough_with_bold() {
    assert_eq!(
        r("~~strike with **bold").as_ref(),
        "~~strike with **bold**~~"
    );
}

#[test]
fn mixed_underscore_inside_bold() {
    assert_eq!(r("**_text").as_ref(), "**_text_**");
}

#[test]
fn mixed_underscore_italic_before_bold() {
    assert_eq!(r("_italic and **bold").as_ref(), "_italic and **bold**_");
}

#[test]
fn mixed_link_priority() {
    // Link handler has early return — further handlers don't run.
    assert_eq!(
        r("Text with [link and **bold").as_ref(),
        "Text with [link and **bold](stitch:incomplete-link)"
    );
}

#[test]
fn mixed_bold_italic_complete() {
    assert_eq!(
        r("**bold with *italic* inside**").as_ref(),
        "**bold with *italic* inside**"
    );
}

#[test]
fn mixed_complex_complete() {
    let text = "# Heading\n\n**Bold text** with *italic* and `code`.\n\n- List item\n- Another item with ~~strike~~";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn mixed_dollar_inside_bold() {
    assert_eq!(r("**bold with $x^2").as_ref(), "**bold with $x^2**");
}
