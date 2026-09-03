use super::*;

// ===========================================================================
// Links
// ===========================================================================

#[test]
fn link_incomplete_url() {
    assert_eq!(
        r("[Click here](http://exam").as_ref(),
        "[Click here](stitch:incomplete-link)"
    );
}

#[test]
fn link_incomplete_text() {
    assert_eq!(
        r("[Click here").as_ref(),
        "[Click here](stitch:incomplete-link)"
    );
}

#[test]
fn link_complete() {
    assert_eq!(
        r("[text](http://example.com)").as_ref(),
        "[text](http://example.com)"
    );
}

#[test]
fn link_multiple_complete() {
    assert_eq!(
        r("[link1](url1) and [link2](url2)").as_ref(),
        "[link1](url1) and [link2](url2)"
    );
}

#[test]
fn link_nested_brackets_incomplete_url() {
    assert_eq!(
        r("[outer [nested] text](incomplete").as_ref(),
        "[outer [nested] text](stitch:incomplete-link)"
    );
}

#[test]
fn link_nested_brackets_complete() {
    assert_eq!(
        r("[link with [brackets] inside](https://example.com)").as_ref(),
        "[link with [brackets] inside](https://example.com)"
    );
}

#[test]
fn link_partial_boundary() {
    assert_eq!(
        r("Check out [this lin").as_ref(),
        "Check out [this lin](stitch:incomplete-link)"
    );
}

#[test]
fn link_partial_url_boundary() {
    assert_eq!(
        r("Visit [our site](https://exa").as_ref(),
        "Visit [our site](stitch:incomplete-link)"
    );
}

#[test]
fn link_no_matching_bracket() {
    assert_eq!(
        r("Text [outer [inner").as_ref(),
        "Text [outer [inner](stitch:incomplete-link)"
    );
}
