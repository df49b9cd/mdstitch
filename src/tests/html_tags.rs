use super::*;

// ===========================================================================
// HTML tags
// ===========================================================================

#[test]
fn html_tag_incomplete_opening() {
    assert_eq!(r("Hello <div").as_ref(), "Hello");
}

#[test]
fn html_tag_incomplete_closing() {
    assert_eq!(r("Hello </div").as_ref(), "Hello");
}

#[test]
fn html_tag_incomplete_custom() {
    assert_eq!(r("Hello <custom").as_ref(), "Hello");
}

#[test]
fn html_tag_incomplete_at_start() {
    assert_eq!(r("<div").as_ref(), "");
}

#[test]
fn html_tag_complete_unchanged() {
    assert_eq!(r("Hello <div>").as_ref(), "Hello <div>");
}

#[test]
fn html_tag_complete_pair_unchanged() {
    assert_eq!(r("<div>content</div>").as_ref(), "<div>content</div>");
}

#[test]
fn html_tag_less_than_sign() {
    assert_eq!(r("3 < 5").as_ref(), "3 < 5");
}

#[test]
fn html_tag_partial_attributes() {
    assert_eq!(r("Hello <div class=\"foo").as_ref(), "Hello");
}

#[test]
fn html_tag_inside_code_block() {
    assert_eq!(r("```\n<div\n```").as_ref(), "```\n<div\n```");
}
