use super::*;

// ===========================================================================
// Streaming simulation
// ===========================================================================

#[test]
fn streaming_nested_formatting() {
    // Bold outer, italic inner — only inner closes (outer stays open).
    assert_eq!(
        r("This is **bold with *ital").as_ref(),
        "This is **bold with *ital*"
    );
}

#[test]
fn streaming_heading_with_emphasis() {
    assert_eq!(
        r("# Main Title\n## Subtitle with **emph").as_ref(),
        "# Main Title\n## Subtitle with **emph**"
    );
}

#[test]
fn streaming_blockquote_with_bold() {
    assert_eq!(r("> Quote with **bold").as_ref(), "> Quote with **bold**");
}

#[test]
fn streaming_table_with_bold() {
    assert_eq!(
        r("| Col1 | Col2 |\n|------|------|\n| **dat").as_ref(),
        "| Col1 | Col2 |\n|------|------|\n| **dat**"
    );
}

#[test]
fn streaming_crlf_between_bracket_and_url() {
    // Stream-chunk boundary splitting `](` from the URL with CRLF in between
    // must not mis-complete a URL whose `)` is on the following line.
    assert!(matches!(
        r("[text](\r\nhttp://example.com)"),
        Cow::Borrowed(_)
    ));
}
