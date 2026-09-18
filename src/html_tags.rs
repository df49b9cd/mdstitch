use std::borrow::Cow;

use super::ranges::CodeBlockRanges;
use super::utils::is_plausible_tag_remainder;

/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
/// The real pipeline calls [`handle_with_ranges`] directly with shared ranges.
#[cfg(test)]
fn handle(text: &str) -> Cow<'_, str> {
    handle_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Strips incomplete HTML tags, using pre-computed code block ranges.
pub(crate) fn handle_with_ranges<'a>(text: &'a str, ranges: &CodeBlockRanges) -> Cow<'a, str> {
    let bytes = text.as_bytes();

    // Scan backward for `<` that starts an incomplete tag, refusing to strip
    // if doing so would expose an *earlier* incomplete tag at the new end
    // (that one would be stripped on the next pass, so stripping would not
    // be self-stable: `<A <A` → strip → `<A` → strip again).
    //
    // `strip_at` records the first strippable `<` we find. After that we
    // keep walking leftward through the remaining prefix; any other `<` that
    // itself would be treated as a strip candidate (same checks, but applied
    // to the *remainder of the whole text*) means stripping `strip_at` would
    // expose an incomplete tag → bail out with Borrowed.
    let mut strip_at: Option<usize> = None;
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b'>' => return Cow::Borrowed(text),
            b'<' => {
                let excludes_via_flank =
                    i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
                let excludes_via_shape = !is_plausible_tag_remainder(&bytes[i + 1..]);
                let excludes_via_code = ranges.is_inside_code(i);
                if excludes_via_flank || excludes_via_shape || excludes_via_code {
                    return Cow::Borrowed(text);
                }
                if strip_at.is_some() {
                    // An earlier `<` forms a strippable tag on its own —
                    // stripping the trailing one would expose it.
                    return Cow::Borrowed(text);
                }
                strip_at = Some(i);
            }
            b'\n' => return Cow::Borrowed(text),
            _ => {}
        }
    }

    match strip_at {
        Some(i) => Cow::Owned(text[..i].trim_end().to_owned()),
        None => Cow::Borrowed(text),
    }
}

#[cfg(test)]
mod tests {
    use super::handle;
    use std::borrow::Cow;

    #[test]
    fn strips_incomplete_tag() {
        assert_eq!(handle("text <custom").as_ref(), "text");
    }

    #[test]
    fn strips_incomplete_closing_tag() {
        assert_eq!(handle("text </div").as_ref(), "text");
    }

    #[test]
    fn leaves_complete_tag() {
        assert!(matches!(
            handle("text <div>content</div>"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn leaves_non_tag() {
        assert!(matches!(handle("5 < 10"), Cow::Borrowed(_)));
    }

    #[test]
    fn inside_code_block() {
        assert!(matches!(handle("```\n<custom\n```"), Cow::Borrowed(_)));
    }

    #[test]
    fn preserves_comparison_like_text() {
        // Issue #15: `a<b` is not a tag — `<` follows a word char.
        assert!(matches!(handle("a<b"), Cow::Borrowed(_)));
    }

    #[test]
    fn preserves_partial_tag_with_invalid_name_char() {
        // Issue #15: `.` is not a valid tag-name char.
        assert!(matches!(handle("name@<example.com"), Cow::Borrowed(_)));
    }

    #[test]
    fn strips_incomplete_tag_with_attributes() {
        assert_eq!(handle("text <div class=\"x\"").as_ref(), "text");
    }

    #[test]
    fn leaves_lone_angle_at_eof() {
        assert!(matches!(handle("text <"), Cow::Borrowed(_)));
    }

    #[test]
    fn leaves_digit_after_angle() {
        assert!(matches!(handle("text <123"), Cow::Borrowed(_)));
    }

    #[test]
    fn strips_bare_incomplete_close_tag() {
        assert_eq!(handle("text </").as_ref(), "text");
    }

    #[test]
    fn strips_incomplete_self_closing() {
        assert_eq!(handle("text <br/").as_ref(), "text");
    }

    #[test]
    fn leaves_digit_before_angle() {
        assert!(matches!(handle("1<b"), Cow::Borrowed(_)));
    }

    #[test]
    fn leaves_underscore_before_angle() {
        assert!(matches!(handle("foo_<b"), Cow::Borrowed(_)));
    }
}
