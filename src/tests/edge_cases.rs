use super::*;

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
fn standalone_markers_unchanged() {
    assert_eq!(r("**").as_ref(), "**");
    assert_eq!(r("__").as_ref(), "__");
    assert_eq!(r("***").as_ref(), "***");
    assert_eq!(r("*").as_ref(), "*");
    assert_eq!(r("_").as_ref(), "_");
    assert_eq!(r("~~").as_ref(), "~~");
    assert_eq!(r("`").as_ref(), "`");
}

#[test]
fn standalone_markers_with_space() {
    assert_eq!(r("** __").as_ref(), "** __");
    assert_eq!(r("* _ ~~ `").as_ref(), "* _ ~~ `");
}

#[test]
fn unicode_in_bold() {
    assert_eq!(r("**émoji 🎉").as_ref(), "**émoji 🎉**");
}

#[test]
fn unicode_in_code() {
    assert_eq!(r("`código").as_ref(), "`código`");
}

#[test]
fn html_entities_in_bold() {
    assert_eq!(r("**&lt;tag&gt;").as_ref(), "**&lt;tag&gt;**");
}

#[test]
fn whitespace_flanked_asterisks() {
    assert_eq!(r("5 * 0").as_ref(), "5 * 0");
    assert_eq!(r("x * y").as_ref(), "x * y");
    assert_eq!(r("2 * 3 * 4").as_ref(), "2 * 3 * 4");
}

#[test]
fn whitespace_asterisk_with_italic() {
    assert_eq!(r("5 * 0 and *italic").as_ref(), "5 * 0 and *italic*");
}

#[test]
fn escaped_asterisk() {
    assert_eq!(
        r("Text with \\* escaped asterisk").as_ref(),
        "Text with \\* escaped asterisk"
    );
}

#[test]
fn very_long_text() {
    let long = "a".repeat(10_000);
    let text = format!("{long} **bold");
    assert_eq!(stitch(&text, &opts()).as_ref(), format!("{long} **bold**"));
}

#[test]
fn markdown_at_end_unchanged() {
    assert_eq!(r("text**").as_ref(), "text**");
    assert_eq!(r("text*").as_ref(), "text*");
    assert_eq!(r("`text`").as_ref(), "`text`");
    assert_eq!(r("text~~").as_ref(), "text~~");
}

#[test]
fn whitespace_before_incomplete() {
    assert_eq!(r("text **bold").as_ref(), "text **bold**");
    assert_eq!(r("text\n**bold").as_ref(), "text\n**bold**");
    assert_eq!(r("text\t`code").as_ref(), "text\t`code`");
}
