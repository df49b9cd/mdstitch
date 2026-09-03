use super::*;

// ===========================================================================
// Code blocks
// ===========================================================================

#[test]
fn code_block_content_untouched() {
    let text = "```\n**bold\n*italic\n~~strike\n```";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn code_block_python_underscores() {
    let text = "```python\ndef __init__(self):\n    pass\n```";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn code_block_brackets_not_links() {
    let text = "```javascript\nconst arr = [1, 2, 3];\nconsole.log(arr[0]);\n```";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn code_block_mermaid_star_syntax() {
    let text = "```mermaid\nstateDiagram-v2\n    [*] --> Idle\n    Idle --> Loading\n```";
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn incomplete_bold_after_code_block() {
    let text = "```css\ncode here\n```\n\n**incomplete bold";
    assert_eq!(
        r(text).as_ref(),
        "```css\ncode here\n```\n\n**incomplete bold**"
    );
}

#[test]
fn incomplete_italic_after_code_block() {
    let text = "```mermaid\nstateDiagram-v2\n    [*] --> Idle\n```\n\nHere is *incomplete italic";
    assert_eq!(
        r(text).as_ref(),
        "```mermaid\nstateDiagram-v2\n    [*] --> Idle\n```\n\nHere is *incomplete italic*"
    );
}
