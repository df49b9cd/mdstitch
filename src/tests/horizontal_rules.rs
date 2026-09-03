use super::*;

// ===========================================================================
// Horizontal rules
// ===========================================================================

#[test]
fn horizontal_rule_dashes() {
    assert_eq!(r("---").as_ref(), "---");
    assert_eq!(r("----").as_ref(), "----");
}

#[test]
fn horizontal_rule_asterisks() {
    assert_eq!(r("***").as_ref(), "***");
    assert_eq!(r("****").as_ref(), "****");
}

#[test]
fn horizontal_rule_underscores() {
    assert_eq!(r("___").as_ref(), "___");
    assert_eq!(r("____").as_ref(), "____");
}

#[test]
fn horizontal_rule_spaced() {
    assert_eq!(r("- - -").as_ref(), "- - -");
    assert_eq!(r("* * *").as_ref(), "* * *");
}

#[test]
fn horizontal_rule_after_text() {
    assert_eq!(r("Some text\n\n---").as_ref(), "Some text\n\n---");
}

#[test]
fn horizontal_rule_between_sections() {
    assert_eq!(
        r("Section 1\n\n---\n\nSection 2").as_ref(),
        "Section 1\n\n---\n\nSection 2"
    );
}

#[test]
fn partial_rules_streaming() {
    assert_eq!(r("--").as_ref(), "--");
    assert_eq!(r("**").as_ref(), "**");
    assert_eq!(r("__").as_ref(), "__");
}
