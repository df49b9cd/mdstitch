use super::*;

// ===========================================================================
// KaTeX (inline — opt-in)
// ===========================================================================

#[test]
fn katex_inline_default_no_completion() {
    // Inline KaTeX is disabled by default.
    assert_eq!(r("Text with $formula").as_ref(), "Text with $formula");
    assert_eq!(r("$incomplete").as_ref(), "$incomplete");
}

#[test]
fn katex_inline_enabled_completes() {
    let opts = StitchOptions::default().inline_katex(true);
    assert_eq!(
        stitch("Text with $formula", &opts).as_ref(),
        "Text with $formula$"
    );
    assert_eq!(stitch("$incomplete", &opts).as_ref(), "$incomplete$");
}

#[test]
fn katex_inline_enabled_complete_unchanged() {
    let opts = StitchOptions::default().inline_katex(true);
    assert_eq!(
        stitch("$x^2 + y^2 = z^2$", &opts).as_ref(),
        "$x^2 + y^2 = z^2$"
    );
}

#[test]
fn katex_inline_enabled_odd() {
    let opts = StitchOptions::default().inline_katex(true);
    assert_eq!(
        stitch("$first$ and $second", &opts).as_ref(),
        "$first$ and $second$"
    );
}

#[test]
fn katex_inline_enabled_escaped() {
    let opts = StitchOptions::default().inline_katex(true);
    assert_eq!(stitch("Price is \\$100", &opts).as_ref(), "Price is \\$100");
}

#[test]
fn katex_math_with_underscores_unchanged() {
    assert_eq!(r("$$x_1 + y_2 = z_3$$").as_ref(), "$$x_1 + y_2 = z_3$$");
}

#[test]
fn katex_dollar_in_inline_code() {
    assert_eq!(
        r("Markdown uses double dollar signs (`$$`) to delimit mathematical expressions.").as_ref(),
        "Markdown uses double dollar signs (`$$`) to delimit mathematical expressions."
    );
}

#[test]
fn katex_asterisks_in_math() {
    assert_eq!(r("$$\\mathbf{w}^{*}$$").as_ref(), "$$\\mathbf{w}^{*}$$");
}
