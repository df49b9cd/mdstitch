use super::*;

// ===========================================================================
// Lists
// ===========================================================================

#[test]
fn list_asterisk_unchanged() {
    assert_eq!(
        r("* Item 1\n* Item 2\n* Item 3").as_ref(),
        "* Item 1\n* Item 2\n* Item 3"
    );
}

#[test]
fn list_single_item() {
    assert_eq!(r("* Single item").as_ref(), "* Single item");
}

#[test]
fn list_nested_unchanged() {
    assert_eq!(
        r("* Parent item\n  * Nested item 1\n  * Nested item 2").as_ref(),
        "* Parent item\n  * Nested item 1\n  * Nested item 2"
    );
}

#[test]
fn list_with_complete_italic() {
    assert_eq!(
        r("* Item with *italic* text\n* Another item").as_ref(),
        "* Item with *italic* text\n* Another item"
    );
}

#[test]
fn list_dash_with_bold() {
    assert_eq!(
        r("- Item 1\n- Item 2 with **bol").as_ref(),
        "- Item 1\n- Item 2 with **bol**"
    );
}

#[test]
fn list_emphasis_only_markers() {
    assert_eq!(r("- __").as_ref(), "- __");
    assert_eq!(r("- **").as_ref(), "- **");
    assert_eq!(r("- ***").as_ref(), "- ***");
    assert_eq!(r("- *").as_ref(), "- *");
    assert_eq!(r("- _").as_ref(), "- _");
    assert_eq!(r("- ~~").as_ref(), "- ~~");
}

#[test]
fn list_emphasis_with_text() {
    assert_eq!(r("- ** text after").as_ref(), "- ** text after**");
}
