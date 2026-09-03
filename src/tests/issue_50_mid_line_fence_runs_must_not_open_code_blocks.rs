use super::*;

// ===========================================================================
// Issue #50: mid-line fence runs must not open code blocks
// ===========================================================================

// Per CommonMark §4.5, a fenced code block opens only when 3+ backticks (or
// tildes) appear at the start of a line with ≤3 leading spaces. A mid-line
// run is literal text and must leave downstream emphasis counters untouched.

#[test]
fn mid_line_backtick_run_does_not_open_fence_for_italic() {
    assert_eq!(r("hello ```\n*italic").as_ref(), "hello ```\n*italic*");
}

#[test]
fn mid_line_tilde_run_does_not_open_fence_for_bold() {
    // Disable strikethrough so the test isolates the fence-vs-prose decision:
    // the `~~~` must NOT open a fenced code block, so `**bold` gets closed.
    let opts = StitchOptions::default().strikethrough(false);
    assert_eq!(
        stitch("text ~~~ more\n**bold", &opts).as_ref(),
        "text ~~~ more\n**bold**"
    );
}

#[test]
fn indented_fence_up_to_three_spaces_still_opens() {
    let text = "   ```\n**bold";
    // Leading 3 spaces is a valid fence per CommonMark §4.5; bold stays inside
    // the unclosed block and is NOT completed.
    assert_eq!(r(text).as_ref(), text);
}

#[test]
fn four_space_indent_is_not_a_fence_so_bold_is_completed() {
    // 4 leading spaces = indented code block, not a fenced one. The `**bold`
    // on the next line is prose and gets a closing `**`.
    assert_eq!(r("    ```\n**bold").as_ref(), "    ```\n**bold**");
}

#[test]
fn mid_line_fence_inside_same_line_as_emphasis() {
    // Mid-line ``` between two asterisks: the markers should complete.
    assert_eq!(r("a ``` *italic").as_ref(), "a ``` *italic*");
}
