use super::*;

// ===========================================================================
// Options disabled
// ===========================================================================

#[test]
fn bold_disabled() {
    let opts = StitchOptions::default().bold(false);
    assert_eq!(stitch("**bold text", &opts).as_ref(), "**bold text");
}

#[test]
fn links_disabled() {
    let opts = StitchOptions::default().links(false).images(false);
    assert_eq!(
        stitch("[Click here](http://exam", &opts).as_ref(),
        "[Click here](http://exam"
    );
}

#[test]
fn all_disabled() {
    let opts = StitchOptions::default()
        .bold(false)
        .italic(false)
        .bold_italic(false)
        .inline_code(false)
        .strikethrough(false)
        .links(false)
        .images(false)
        .katex(false)
        .setext_headings(false)
        .html_tags(false)
        .single_tilde(false)
        .comparison_operators(false);
    assert_eq!(
        stitch("**bold *italic `code [link", &opts).as_ref(),
        "**bold *italic `code [link"
    );
}
