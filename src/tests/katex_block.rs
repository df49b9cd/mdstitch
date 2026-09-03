use super::*;

// ===========================================================================
// KaTeX (block)
// ===========================================================================

#[test]
fn katex_block_incomplete() {
    assert_eq!(r("$$x + y").as_ref(), "$$x + y$$");
}

#[test]
fn katex_block_at_start() {
    assert_eq!(r("$$incomplete").as_ref(), "$$incomplete$$");
}

#[test]
fn katex_block_complete() {
    assert_eq!(r("$$E = mc^2$$").as_ref(), "$$E = mc^2$$");
}

#[test]
fn katex_block_multiple() {
    assert_eq!(
        r("$$formula1$$ and $$formula2$$").as_ref(),
        "$$formula1$$ and $$formula2$$"
    );
}

#[test]
fn katex_block_odd() {
    assert_eq!(
        r("$$first$$ and $$second").as_ref(),
        "$$first$$ and $$second$$"
    );
}

#[test]
fn katex_block_half_dollar() {
    assert_eq!(r("$$formula$").as_ref(), "$$formula$$");
}

#[test]
fn katex_block_multiline() {
    assert_eq!(r("$$\nx = 1\ny = 2").as_ref(), "$$\nx = 1\ny = 2\n$$");
}
