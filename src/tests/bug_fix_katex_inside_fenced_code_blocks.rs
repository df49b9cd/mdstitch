use super::*;

// ===========================================================================
// Bug fix: KaTeX inside fenced code blocks
// ===========================================================================

#[test]
fn katex_dollar_pairs_inside_fenced_code() {
    // $$ inside ``` should not be treated as math delimiters.
    assert_eq!(r("```\n$$x + y\n```").as_ref(), "```\n$$x + y\n```");
}

#[test]
fn katex_escaped_dollar_pairs() {
    // Escaped \$$ should not trigger math completion.
    assert_eq!(r("\\$$100").as_ref(), "\\$$100");
}

#[test]
fn inline_katex_inside_fenced_code() {
    let opts = StitchOptions::default().inline_katex(true);
    assert_eq!(
        stitch("```\n$x + y\n```", &opts).as_ref(),
        "```\n$x + y\n```"
    );
}
