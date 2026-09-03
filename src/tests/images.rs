use super::*;

// ===========================================================================
// Images
// ===========================================================================

#[test]
fn image_incomplete_removed() {
    assert_eq!(r("text ![alt](http://").as_ref(), "text");
}

#[test]
fn image_incomplete_text_removed() {
    assert_eq!(r("text ![alt").as_ref(), "text");
}

#[test]
fn image_partial_removed() {
    assert_eq!(r("![partial").as_ref(), "");
}

#[test]
fn image_complete_unchanged() {
    assert_eq!(
        r("Text with ![alt text](image.png)").as_ref(),
        "Text with ![alt text](image.png)"
    );
}

#[test]
fn image_nested_brackets_removed() {
    assert_eq!(r("Text ![outer [inner]").as_ref(), "Text");
}

#[test]
fn image_url_with_underscores_unchanged() {
    let text = "textContent ![image](https://img.example.com/path_name.png)";
    assert_eq!(r(text).as_ref(), text);
}
