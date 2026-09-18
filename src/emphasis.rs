use std::borrow::Cow;

use super::fence::{FenceScanner, fence_run_length, for_each_byte_outside_fence};
use super::ranges::CodeBlockRanges;
use super::utils::{
    cow_append, ends_with_odd_backslashes, find_trailing_delimiter, is_empty_or_markers,
    is_escaped, is_horizontal_rule, is_list_marker_line, is_word_char, is_ws_byte,
    word_internal_at,
};

// ---------------------------------------------------------------------------
// Asterisk skip logic
// ---------------------------------------------------------------------------

fn should_skip_asterisk(text: &str, index: usize, prev: u8, next: u8) -> bool {
    // Skip if escaped.
    if is_escaped(text.as_bytes(), index) {
        return true;
    }

    let bytes = text.as_bytes();
    let next_next = if index + 2 < bytes.len() {
        bytes[index + 2]
    } else {
        0
    };

    // Special handling for *** sequences.
    if prev != b'*' && next == b'*' {
        if next_next == b'*' {
            // First * in a *** sequence — count it.
            return false;
        }
        // First * in ** — skip.
        return true;
    }

    // Skip if second or third * in a sequence.
    if prev == b'*' {
        return true;
    }

    // Skip if word-internal (use proper Unicode char lookup).
    if word_internal_at(text, index) {
        return true;
    }

    // Asymmetric: SOF is ws (so "* foo" stays a list bullet); EOF is not
    // (so our own trailing `*` remains countable as a closer). CommonMark
    // treats `\r`, `\n`, and `\r\n` as line terminators, so `\r` flanks
    // like `\n` here.
    let prev_ws = prev == 0 || is_ws_byte(prev);
    let next_ws = is_ws_byte(next);
    if prev_ws && next_ws {
        return true;
    }

    false
}

/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn count_single_asterisks(text: &str) -> usize {
    count_single_asterisks_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Counts single asterisks that are not part of `**`/`***`, not escaped,
/// not list markers, not word-internal, and not inside fenced code blocks.
pub(crate) fn count_single_asterisks_with_ranges(text: &str, ranges: &CodeBlockRanges) -> usize {
    count_single_markers_with_ranges(text, ranges, b'*')
}

/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn count_single_underscores(text: &str) -> usize {
    count_single_underscores_with_ranges(text, &CodeBlockRanges::new(text))
}

pub(crate) fn count_single_underscores_with_ranges(text: &str, ranges: &CodeBlockRanges) -> usize {
    count_single_markers_with_ranges(text, ranges, b'_')
}

fn count_single_markers_with_ranges(text: &str, ranges: &CodeBlockRanges, marker: u8) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut count = 0;

    for_each_byte_outside_fence(bytes, |byte, i, _| {
        if byte == marker {
            let prev = if i > 0 { bytes[i - 1] } else { 0 };
            let next = if i + 1 < len { bytes[i + 1] } else { 0 };
            let skip = if marker == b'*' {
                should_skip_asterisk(text, i, prev, next)
            } else {
                should_skip_underscore(text, i, prev, next, ranges)
            };
            if !skip {
                count += 1;
            }
        }
    });
    count
}

// ---------------------------------------------------------------------------
// Underscore skip logic
// ---------------------------------------------------------------------------

fn should_skip_underscore(
    text: &str,
    index: usize,
    prev: u8,
    next: u8,
    ranges: &CodeBlockRanges,
) -> bool {
    if is_escaped(text.as_bytes(), index) {
        return true;
    }
    if ranges.is_within_link_url(index) {
        return true;
    }
    // Only CLOSED tags hide emphasis markers; unterminated tags must not
    // swallow closers appended by a previous stitch pass (idempotency).
    if ranges.is_within_closed_html_tag(index, text.len()) {
        return true;
    }
    // Skip if part of __.
    if prev == b'_' || next == b'_' {
        return true;
    }
    // Skip if word-internal (use proper Unicode char lookup for multi-byte chars).
    if word_internal_at(text, index) {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// Triple asterisks counting
// ---------------------------------------------------------------------------

pub fn count_triple_asterisks(text: &str) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut count = 0;
    let mut consecutive = 0usize;
    let mut scanner = FenceScanner::new();
    let mut i = 0;
    let mut line_start = 0usize;

    while i < len {
        if i == line_start
            && let Some(next) = scanner.consume_fence_at_line_start(bytes, line_start)
        {
            if consecutive >= 3 {
                count += consecutive / 3;
            }
            consecutive = 0;
            i = next;
            continue;
        }
        if scanner.in_code_block() {
            if bytes[i] == b'\n' {
                line_start = i + 1;
            }
            i += 1;
            continue;
        }
        // Backtick/tilde runs that aren't line-start fences: skip past without
        // resetting `consecutive` so a streak of `***` split by a stray `` ` ``
        // (or a mid-line ```) still counts.
        if bytes[i] == b'`' || bytes[i] == b'~' {
            i += fence_run_length(bytes, i, bytes[i]);
            continue;
        }
        if bytes[i] == b'*' {
            // Skip escaped asterisks.
            if is_escaped(bytes, i) {
                if consecutive >= 3 {
                    count += consecutive / 3;
                }
                consecutive = 0;
            } else {
                consecutive += 1;
            }
        } else {
            if consecutive >= 3 {
                count += consecutive / 3;
            }
            consecutive = 0;
        }
        if bytes[i] == b'\n' {
            line_start = i + 1;
        }
        i += 1;
    }
    if consecutive >= 3 {
        count += consecutive / 3;
    }
    count
}

// ---------------------------------------------------------------------------
// Double marker counting (outside code blocks)
// ---------------------------------------------------------------------------

fn count_double_markers_outside_code_blocks(text: &str, marker: u8) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut run_len = 0usize;
    for_each_byte_outside_fence(bytes, |byte, _, _| {
        if byte != marker {
            // ⌊n/2⌋ per run — matches the i+=2 step semantics for any length.
            count += run_len / 2;
            run_len = 0;
        } else {
            run_len += 1;
        }
    });
    count += run_len / 2;
    count
}

fn count_double_asterisks(text: &str) -> usize {
    count_double_markers_outside_code_blocks(text, b'*')
}

fn count_double_underscores(text: &str) -> usize {
    count_double_markers_outside_code_blocks(text, b'_')
}

// ---------------------------------------------------------------------------
// Pattern matching helpers (replaces regex patterns)
// ---------------------------------------------------------------------------

/// Finds the last `**` matching the TS pattern `/(\*\*)([^*]*\*?)$/`.
/// Content after `**` must contain no `*` except optionally one at the very end.
fn find_trailing_bold(text: &str) -> Option<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut i = bytes.len();
    while i >= 2 {
        i -= 1;
        if bytes[i] == b'*' && bytes[i - 1] == b'*' {
            let marker_start = i - 1;
            // Skip if this is actually `***`.
            if marker_start > 0 && bytes[marker_start - 1] == b'*' {
                continue;
            }
            let content = &text[i + 1..];
            // Match TS pattern [^*]*\*?$ — content must have no `*` except
            // optionally one at the very end.
            if content.is_empty() {
                return Some((marker_start, content));
            }
            let has_inner_star = if let Some(stripped) = content.strip_suffix('*') {
                // Allow trailing `*`, but check rest has no `*`.
                stripped.contains('*')
            } else {
                content.contains('*')
            };
            if has_inner_star {
                continue;
            }
            return Some((marker_start, content));
        }
    }
    None
}

/// Finds the last `__` followed by non-`_` content at the end of text.
/// Matches the TS pattern `/(__)([^_]*?)$/` — content must not contain `_`.
fn find_trailing_double_underscore(text: &str) -> Option<(usize, &str)> {
    find_trailing_delimiter(text, b"__")
}

/// Finds the last `***` followed by non-`*` content at the end of text.
fn find_trailing_bold_italic(text: &str) -> Option<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut i = bytes.len();
    while i >= 3 {
        i -= 1;
        if bytes[i] == b'*' && i >= 2 && bytes[i - 1] == b'*' && bytes[i - 2] == b'*' {
            let marker_start = i - 2;
            // Skip if 4+ asterisks.
            if marker_start > 0 && bytes[marker_start - 1] == b'*' {
                continue;
            }
            let content = &text[i + 1..];
            if content.starts_with('*') {
                continue;
            }
            return Some((marker_start, content));
        }
    }
    None
}

/// Finds the last `~~` followed by non-`~` content at the end of text.
/// Matches the TS pattern `/(~~)([^~]*?)$/` — content must not contain `~`.
pub fn find_trailing_strikethrough(text: &str) -> Option<(usize, &str)> {
    find_trailing_delimiter(text, b"~~")
}

// ---------------------------------------------------------------------------
// Skip completion checks
// ---------------------------------------------------------------------------

fn should_skip_bold_completion(text: &str, content: &str, marker_index: usize) -> bool {
    if content.is_empty() || is_empty_or_markers(content) {
        return true;
    }

    // Check if in a list item with multiline content.
    let before = &text[..marker_index];
    let line_start = before.rfind('\n').map_or(0, |p| p + 1);
    let line_before = &text[line_start..marker_index];
    if is_list_marker_line(line_before) && content.contains('\n') {
        return true;
    }

    is_horizontal_rule(text, marker_index, b'*')
}

fn should_skip_italic_completion(text: &str, content: &str, marker_index: usize) -> bool {
    if content.is_empty() || is_empty_or_markers(content) {
        return true;
    }

    let before = &text[..marker_index];
    let line_start = before.rfind('\n').map_or(0, |p| p + 1);
    let line_before = &text[line_start..marker_index];
    if is_list_marker_line(line_before) && content.contains('\n') {
        return true;
    }

    is_horizontal_rule(text, marker_index, b'_')
}

// ---------------------------------------------------------------------------
// Find first single marker index
// ---------------------------------------------------------------------------

/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn find_first_single_asterisk_index(text: &str) -> Option<usize> {
    find_first_single_asterisk_index_with_ranges(text, &CodeBlockRanges::new(text))
}

fn find_first_single_asterisk_index_with_ranges(
    text: &str,
    ranges: &CodeBlockRanges,
) -> Option<usize> {
    find_first_single_marker_index_with_ranges(text, ranges, b'*')
}

/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn find_first_single_underscore_index(text: &str) -> Option<usize> {
    find_first_single_underscore_index_with_ranges(text, &CodeBlockRanges::new(text))
}

fn find_first_single_underscore_index_with_ranges(
    text: &str,
    ranges: &CodeBlockRanges,
) -> Option<usize> {
    find_first_single_marker_index_with_ranges(text, ranges, b'_')
}

fn find_first_single_marker_index_with_ranges(
    text: &str,
    ranges: &CodeBlockRanges,
    marker: u8,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let has_dollar = text.contains('$');
    let mut scanner = FenceScanner::new();
    let mut i = 0;
    let mut line_start = 0usize;

    while i < len {
        if i == line_start
            && let Some(next) = scanner.consume_fence_at_line_start(bytes, line_start)
        {
            i = next;
            continue;
        }
        if scanner.in_code_block() {
            if bytes[i] == b'\n' {
                line_start = i + 1;
            }
            i += 1;
            continue;
        }

        if bytes[i] == marker {
            let prev = if i > 0 { bytes[i - 1] } else { 0 };
            let next = if i + 1 < len { bytes[i + 1] } else { 0 };

            // Must be a single marker (not part of a double or triple run).
            if prev == marker || next == marker {
                i += 1;
                continue;
            }
            if prev == b'\\' {
                i += 1;
                continue;
            }
            if has_dollar && ranges.is_within_complete_math(i) {
                i += 1;
                continue;
            }

            // Asterisk-only: flanking whitespace check; underscore-only:
            // link URLs are not emphasis (see should_skip_underscore).
            if marker != b'*' && ranges.is_within_link_url(i) {
                i += 1;
                continue;
            }
            if marker == b'*' {
                // Asymmetric: SOF is ws, EOF is not — see should_skip_asterisk.
                let prev_ws = prev == 0 || is_ws_byte(prev);
                let next_ws = is_ws_byte(next);
                if prev_ws && next_ws {
                    i += 1;
                    continue;
                }
            }

            // Skip if word-internal (Unicode-aware).
            if word_internal_at(text, i) {
                i += 1;
                continue;
            }

            return Some(i);
        }
        if bytes[i] == b'\n' {
            line_start = i + 1;
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Cross-handler swallow gates
// ---------------------------------------------------------------------------
//
// Each appending handler must answer: "after I append, will MY next pass still
// see the closer I just added?" A closer is invisible when a *later* handler
// (katex, inline_code) completes a region that contains the appended byte.
// The gates below refuse the append whenever such a region would swallow it.
// They are content-keyed (positions of `$$`/`$`/backticks outside code), not
// priority-keyed, so pass order stays untouched.

/// Scans `text[start..]` for an unescaped `$$` run outside code, probing both
/// the first and the second byte of each run so a `$$` whose second byte lands
/// in complete math still counts. Returns the index of the first `$`.
///
/// Gate A: if such a `$$` sits after the first counted italic marker, the
/// appended closer would be swallowed by a math span on the next pass —
/// whether katex closes that `$$` in this pass (`*A$$\n`) or it is already
/// closed (`*A$$\n$$`). Either way the appended byte ends up inside a region
/// the counter cannot see into.
fn unclosed_block_math_after(text: &str, start: usize, ranges: &CodeBlockRanges) -> Option<usize> {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = start;
    while i + 1 < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'$' && bytes[i + 1] == b'$' {
            if !ranges.is_inside_code(i) && !ranges.is_inside_code(i + 1) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Counts unescaped, non-`$$` `$` bytes in `text[start..]` that are outside
/// code and outside complete math — i.e. singles inline_katex would still see.
fn count_gate_relevant_single_dollars(text: &str, start: usize, ranges: &CodeBlockRanges) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let mut count = 0;
    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'$' {
            if i + 1 < len && bytes[i + 1] == b'$' {
                i += 2;
                continue;
            }
            if !ranges.is_inside_code(i) && !ranges.is_within_complete_math(i) {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

/// Counts unescaped single backticks in `text[start..]` outside code —
/// candidates inline_code would close *after* the emphasis handlers ran.
/// (Triple runs are fence-ish and inline_code leaves them alone.)
fn count_gate_relevant_single_backticks(text: &str, start: usize, ranges: &CodeBlockRanges) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = start;
    let mut count = 0;
    while i < len {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            let run_start = i;
            while i < len && bytes[i] == b'`' {
                i += 1;
            }
            let run_len = i - run_start;
            if run_len == 1
                && !ranges.is_inside_code(run_start)
                && !ranges.is_within_complete_math(run_start)
            {
                count += 1;
            }
            continue;
        }
        i += 1;
    }
    count
}

/// Gate A (block math): refuse the append if an unclosed `$$` after `first_idx`
/// would make the appended closer invisible inside `$$…$$` on the next pass.
fn gate_block_math_swallow(text: &str, first_idx: usize, ranges: &CodeBlockRanges) -> bool {
    unclosed_block_math_after(text, first_idx + 1, ranges).is_some()
}

/// Gate B (inline math): refuse when an odd number of unclosed single `$`
/// follows `first_idx` — inline_katex would append `$` in the same pass,
/// wrapping the appended closer inside a complete `$…$` span that hides it.
fn gate_inline_math_swallow(text: &str, first_idx: usize, ranges: &CodeBlockRanges) -> bool {
    count_gate_relevant_single_dollars(text, first_idx + 1, ranges) % 2 == 1
}

/// Gate C (inline code): refuse when inline_code (running earlier in the
/// pipeline) already left a dangling single backtick open after `first_idx`
/// — the appended closer would land inside the still-unclosed span, so the
/// emphasis counter cannot see it on the next pass.
fn gate_inline_code_swallow(text: &str, first_idx: usize, ranges: &CodeBlockRanges) -> bool {
    count_gate_relevant_single_backticks(text, first_idx + 1, ranges) % 2 == 1
}

/// Combined per-handler gate: any of A/B/C true → do not append.
fn gate_refuses_emphasis_append(text: &str, first_idx: usize, ranges: &CodeBlockRanges) -> bool {
    gate_block_math_swallow(text, first_idx, ranges)
        || gate_inline_math_swallow(text, first_idx, ranges)
        || gate_inline_code_swallow(text, first_idx, ranges)
}

// ---------------------------------------------------------------------------
// Public handler functions
// ---------------------------------------------------------------------------


/// Completes incomplete bold formatting (`**`).
/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn handle_bold(text: &str) -> Cow<'_, str> {
    handle_bold_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Completes incomplete bold formatting, using pre-computed code block ranges.
pub(crate) fn handle_bold_with_ranges<'a>(text: &'a str, ranges: &CodeBlockRanges) -> Cow<'a, str> {
    let Some((marker_index, content)) = find_trailing_bold(text) else {
        return Cow::Borrowed(text);
    };

    if ranges.is_inside_code(marker_index) || ranges.is_within_complete_inline_code(marker_index) {
        return Cow::Borrowed(text);
    }

    // `**` sitting inside a complete `$...$` / `$$...$$` span is math content,
    // not emphasis — without this, `$**` becomes `$**$` (inline-katex close)
    // then `$**$**` on the next pass (bold treats `$` as non-empty content).
    if ranges.is_within_complete_math(marker_index) {
        return Cow::Borrowed(text);
    }

    if should_skip_bold_completion(text, content, marker_index) {
        return Cow::Borrowed(text);
    }

    let pairs = count_double_asterisks(text);
    if pairs % 2 == 1 {
        if ends_with_odd_backslashes(text) {
            return Cow::Borrowed(text);
        }
        // Half-complete: **content* → **content**
        if content.ends_with('*') {
            return cow_append(text, "*");
        }
        return cow_append(text, "**");
    }

    Cow::Borrowed(text)
}

/// Completes incomplete double-underscore italic (`__`).
/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn handle_double_underscore(text: &str) -> Cow<'_, str> {
    handle_double_underscore_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Completes incomplete double-underscore italic, using pre-computed code block ranges.
pub(crate) fn handle_double_underscore_with_ranges<'a>(
    text: &'a str,
    ranges: &CodeBlockRanges,
) -> Cow<'a, str> {
    // First check for trailing `__content` pattern.
    if let Some((marker_index, content)) = find_trailing_double_underscore(text)
        && !ranges.is_inside_code(marker_index)
        && !ranges.is_within_complete_inline_code(marker_index)
        && !ranges.is_within_complete_math(marker_index)
        && !should_skip_italic_completion(text, content, marker_index)
    {
        let pairs = count_double_underscores(text);
        if pairs % 2 == 1 {
            if ends_with_odd_backslashes(text) {
                return Cow::Borrowed(text);
            }
            return cow_append(text, "__");
        }
    }

    // Check for half-complete: __content_ → __content__
    if let Some(pos) = find_half_complete_underscore(text)
        && !ranges.is_inside_code(pos)
        && !ranges.is_within_complete_inline_code(pos)
        && !ranges.is_within_complete_math(pos)
    {
        let pairs = count_double_underscores(text);
        if pairs % 2 == 1 {
            if ends_with_odd_backslashes(text) {
                return Cow::Borrowed(text);
            }
            return cow_append(text, "_");
        }
    }

    Cow::Borrowed(text)
}

/// Finds `__content_` pattern (half-complete closing).
/// Returns the marker index of the opening `__`.
fn find_half_complete_underscore(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    // Must end with single `_` but not `__`.
    if !text.ends_with('_') || text.ends_with("__") {
        return None;
    }
    let content_end = bytes.len() - 1; // index of trailing `_`
    // Look backward for opening `__`.
    if content_end < 3 {
        return None;
    }
    let mut i = content_end - 1; // start before the trailing `_`
    while i >= 1 {
        if bytes[i] == b'_' && bytes[i - 1] == b'_' {
            // Check that content between `__` and the trailing `_` doesn't have `_`.
            let between = &text[i + 1..content_end];
            if !between.contains('_') {
                return Some(i - 1);
            }
        }
        i -= 1;
    }
    None
}

/// Completes incomplete italic with single asterisk (`*`).
/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn handle_italic_asterisk(text: &str) -> Cow<'_, str> {
    handle_italic_asterisk_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Completes incomplete italic with single asterisk, using pre-computed code block ranges.
pub(crate) fn handle_italic_asterisk_with_ranges<'a>(
    text: &'a str,
    ranges: &CodeBlockRanges,
) -> Cow<'a, str> {
    // Check for trailing single asterisk pattern.
    let bytes = text.as_bytes();
    // Quick check: does text end with a non-`*` followed sometime by a `*` earlier?
    if !bytes.contains(&b'*') {
        return Cow::Borrowed(text);
    }

    let Some(first_idx) = find_first_single_asterisk_index_with_ranges(text, ranges) else {
        return Cow::Borrowed(text);
    };

    if ranges.is_inside_code(first_idx) || ranges.is_within_complete_inline_code(first_idx) {
        return Cow::Borrowed(text);
    }

    let content_after = &text[first_idx + 1..];
    if content_after.is_empty() || is_empty_or_markers(content_after) {
        return Cow::Borrowed(text);
    }

    let count = count_single_asterisks_with_ranges(text, ranges);
    if count % 2 == 1 {
        if ends_with_odd_backslashes(text) {
            return Cow::Borrowed(text);
        }
        // A trailing `***` run already contains a `*` that can serve as the
        // italic closer. Appending another `*` extends the run; the new byte
        // has `prev == '*'` so the counter ignores it, the count stays odd,
        // and the next pass appends again — violating idempotency.
        if text.ends_with("***") {
            return Cow::Borrowed(text);
        }
        // When text ends with `**` that closes a balanced bold pair AND the
        // orphan `*` is right-flanking only (prev=non-ws, next=ws/EOF — it
        // can close but not open emphasis), appending `*` at EOF would turn
        // the bold closer into `***` with no opener to pair with. On the
        // next pass italic-underscore may insert `_` before the `**`,
        // restoring a trailing `**` that re-triggers this branch and
        // appends again — breaking idempotency. Leave literal `*` alone.
        if text.ends_with("**") && count_double_asterisks(text).is_multiple_of(2) {
            let bytes = text.as_bytes();
            let prev = if first_idx > 0 {
                bytes[first_idx - 1]
            } else {
                0
            };
            let next = if first_idx + 1 < bytes.len() {
                bytes[first_idx + 1]
            } else {
                0
            };
            let prev_ws = prev == 0 || is_ws_byte(prev);
            let next_ws = next == 0 || is_ws_byte(next);
            if !prev_ws && next_ws {
                return Cow::Borrowed(text);
            }
        }
        // A trailing `*` preceded by an unescaped `\` is escaped — the counter
        // skips it, so the italic run that includes it is invisible.
        // Appending `*` extends an invisible run (proptest `"*>\*"`:
        // `"*>\*"` → `"*>\**"` → `"*>\***"` → …). Leave the text alone.
        if text.ends_with('*') && text.len() >= 2 {
            let bytes = text.as_bytes();
            let before_last = bytes[bytes.len() - 2];
            if before_last == b'\\' && !is_escaped(bytes, bytes.len() - 2) {
                return Cow::Borrowed(text);
            }
        }
        // Swallow gates: refuse `*` if a later handler (katex_block /
        // inline_katex / inline_code) would close a region over it,
        // hiding the appended closer from this counter on the next pass.
        if gate_refuses_emphasis_append(text, first_idx, ranges) {
            return Cow::Borrowed(text);
        }
        return cow_append(text, "*");
    }

    Cow::Borrowed(text)
}

/// Completes incomplete italic with single underscore (`_`).
/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn handle_italic_underscore(text: &str) -> Cow<'_, str> {
    handle_italic_underscore_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Completes incomplete italic with single underscore, using pre-computed code block ranges.
pub(crate) fn handle_italic_underscore_with_ranges<'a>(
    text: &'a str,
    ranges: &CodeBlockRanges,
) -> Cow<'a, str> {
    if !text.as_bytes().contains(&b'_') {
        return Cow::Borrowed(text);
    }

    let Some(first_idx) = find_first_single_underscore_index_with_ranges(text, ranges) else {
        return Cow::Borrowed(text);
    };

    let content_after = &text[first_idx + 1..];
    if content_after.is_empty() || is_empty_or_markers(content_after) {
        return Cow::Borrowed(text);
    }

    if ranges.is_inside_code(first_idx) || ranges.is_within_complete_inline_code(first_idx) {
        return Cow::Borrowed(text);
    }

    let count = count_single_underscores_with_ranges(text, ranges);
    if count % 2 == 1 {
        // Must gate before the helpers: they produce non-suffix insertions
        // that a trailing-backslash escape would invalidate on the next pass.
        if ends_with_odd_backslashes(text) {
            return Cow::Borrowed(text);
        }
        // Check if we need to insert `_` before trailing `**` for proper nesting.
        if let Some(result) = handle_trailing_asterisks_for_underscore(text, ranges) {
            return Cow::Owned(result);
        }
        // Same idea for a single trailing `*`: appending `_` after it would
        // reclassify the `*` as word-internal on the next pass (breaking
        // idempotency), so insert `_` before the `*` instead.
        if let Some(result) = handle_trailing_single_asterisk_for_underscore(text) {
            return Cow::Owned(result);
        }
        // A trailing `_` that immediately follows `_` or an escaped `\` would
        // extend an inert run: the counter skips `prev == '_'` members and any
        // escaped opener, so two trailing `_`s already form a complete double
        // (`_0__` → `_0___` → `_0____`… — italic never converges). The closer
        // is the trailing `_` itself; leave the text alone.
        if text.ends_with('_') && text.len() >= 2 && {
            let before = &text.as_bytes()[..text.len() - 1];
            before.last() == Some(&b'_')
                || (before.last() == Some(&b'\\') && !is_escaped(text.as_bytes(), text.len() - 2))
        } {
            return Cow::Borrowed(text);
        }
        // Swallow gates: refuse `_` if a later handler (katex_block /
        // inline_katex) would close a region over it, hiding the appended
        // closer from this counter on the next pass.
        if gate_refuses_emphasis_append(text, first_idx, ranges) {
            return Cow::Borrowed(text);
        }
        return insert_closing_underscore(text);
    }

    Cow::Borrowed(text)
}

/// If text ends with a single `*` (not part of `**`/`***`) that is preceded
/// by a word char, appending `_` at EOF would make the `*` word-internal on
/// the next pass. Insert `_` before the `*` to preserve its flanking.
fn handle_trailing_single_asterisk_for_underscore(text: &str) -> Option<String> {
    if !text.ends_with('*') || text.ends_with("**") {
        return None;
    }
    let before = &text[..text.len() - 1];
    if !before.chars().next_back().is_some_and(is_word_char) {
        return None;
    }
    let mut result = String::with_capacity(text.len() + 1);
    result.push_str(before);
    result.push('_');
    result.push('*');
    Some(result)
}

/// If text ends with `**` that was added to close an unclosed bold,
/// and there's an unclosed `_` before it, insert `_` before the `**`.
///
/// Reuses the caller's `ranges`: `without` is a strict prefix of `text` (only
/// trailing `**` dropped), so no position shifts and the precomputed ranges
/// stay valid.
fn handle_trailing_asterisks_for_underscore(
    text: &str,
    ranges: &CodeBlockRanges,
) -> Option<String> {
    if !text.ends_with("**") {
        return None;
    }

    let without = &text[..text.len() - 2];
    let pairs = count_double_markers_outside_code_blocks(without, b'*');
    if pairs % 2 != 1 {
        return None;
    }

    let first_double = without.find("**")?;
    let underscore_idx = find_first_single_underscore_index_with_ranges(without, ranges)?;

    if first_double < underscore_idx {
        let mut result = String::with_capacity(text.len() + 1);
        result.push_str(without);
        result.push('_');
        result.push_str("**");
        return Some(result);
    }

    None
}

/// Inserts closing `_`, placing it before any trailing newlines.
fn insert_closing_underscore(text: &str) -> Cow<'_, str> {
    let bytes = text.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    if end < bytes.len() {
        let mut result = String::with_capacity(text.len() + 1);
        result.push_str(&text[..end]);
        result.push('_');
        result.push_str(&text[end..]);
        Cow::Owned(result)
    } else {
        cow_append(text, "_")
    }
}

/// Completes incomplete bold-italic formatting (`***`).
/// Test-only convenience wrapper that builds `CodeBlockRanges` on the fly.
#[cfg(test)]
fn handle_bold_italic(text: &str) -> Cow<'_, str> {
    handle_bold_italic_with_ranges(text, &CodeBlockRanges::new(text))
}

/// Completes incomplete bold-italic formatting, using pre-computed code block ranges.
pub(crate) fn handle_bold_italic_with_ranges<'a>(
    text: &'a str,
    ranges: &CodeBlockRanges,
) -> Cow<'a, str> {
    // Don't process if text is only 4+ asterisks.
    if is_asterisk_run(text) {
        return Cow::Borrowed(text);
    }

    let Some((marker_index, content)) = find_trailing_bold_italic(text) else {
        return Cow::Borrowed(text);
    };

    if content.is_empty() || is_empty_or_markers(content) {
        return Cow::Borrowed(text);
    }

    if ranges.is_inside_code(marker_index) || ranges.is_within_complete_inline_code(marker_index) {
        return Cow::Borrowed(text);
    }

    if ranges.is_within_complete_math(marker_index) {
        return Cow::Borrowed(text);
    }

    if is_horizontal_rule(text, marker_index, b'*') {
        return Cow::Borrowed(text);
    }

    let triples = count_triple_asterisks(text);
    if triples % 2 == 1 {
        // If both ** and * are already balanced, don't add ***.
        let double_pairs = count_double_asterisks(text);
        let single_count = count_single_asterisks_with_ranges(text, ranges);
        if double_pairs.is_multiple_of(2) && single_count.is_multiple_of(2) {
            return Cow::Borrowed(text);
        }
        if ends_with_odd_backslashes(text) {
            return Cow::Borrowed(text);
        }
        return cow_append(text, "***");
    }

    Cow::Borrowed(text)
}

/// Returns `true` if `text` is a run of four or more `*` and nothing else
/// (already a horizontal rule; nothing to complete).
fn is_asterisk_run(text: &str) -> bool {
    text.len() >= 4 && text.bytes().all(|b| b == b'*')
}

#[cfg(test)]
mod tests {
    use super::{
        count_double_asterisks, count_double_underscores, count_single_asterisks,
        count_single_underscores, count_triple_asterisks, find_first_single_asterisk_index,
        find_first_single_underscore_index, handle_bold, handle_bold_italic,
        handle_double_underscore, handle_italic_asterisk, handle_italic_underscore,
    };
    use std::borrow::Cow;

    // Direct counter coverage (issue #50 follow-up): verify the six fence-aware
    // emphasis helpers all treat mid-line 3+ runs as prose and honor line-start
    // fences identically.

    #[test]
    fn single_asterisks_counted_outside_mid_line_run() {
        // `*italic` sits after a mid-line ``` which must NOT open a fence.
        assert_eq!(count_single_asterisks("hello ```\n*italic"), 1);
    }

    #[test]
    fn single_asterisks_ignored_inside_fenced_block() {
        assert_eq!(count_single_asterisks("```\n*italic\n```"), 0);
    }

    #[test]
    fn single_underscores_counted_outside_mid_line_run() {
        assert_eq!(count_single_underscores("hello ```\n_italic"), 1);
    }

    #[test]
    fn single_underscores_ignored_inside_fenced_block() {
        assert_eq!(count_single_underscores("```\n_italic\n```"), 0);
    }

    #[test]
    fn triple_asterisks_counted_across_mid_line_run() {
        // Mid-line ``` splits a `***` streak; run is inert, streak continues.
        assert_eq!(count_triple_asterisks("**```*"), 1);
    }

    #[test]
    fn triple_asterisks_ignored_inside_fenced_block() {
        assert_eq!(count_triple_asterisks("```\n***\n```"), 0);
    }

    #[test]
    fn double_markers_counted_outside_mid_line_run() {
        // Mid-line ``` must not open a fence, so **bold is countable.
        assert_eq!(count_double_asterisks("x ```\n**bold"), 1);
        assert_eq!(count_double_underscores("x ```\n__bold"), 1);
    }

    #[test]
    fn double_markers_ignored_inside_fenced_block() {
        assert_eq!(count_double_asterisks("```\n**bold\n```"), 0);
        assert_eq!(count_double_underscores("```\n__bold\n```"), 0);
    }

    #[test]
    fn first_single_asterisk_index_skips_fenced_block() {
        // Asterisk inside the fence is ignored; the one on the "after" line wins.
        assert_eq!(
            find_first_single_asterisk_index("```\n*inside\n```\n*after"),
            Some(16),
        );
    }

    #[test]
    fn first_single_underscore_index_skips_fenced_block() {
        assert_eq!(
            find_first_single_underscore_index("```\n_inside\n```\n_after"),
            Some(16),
        );
    }

    // Bold tests
    #[test]
    fn bold_completes_incomplete() {
        assert_eq!(handle_bold("**bold text").as_ref(), "**bold text**");
    }

    #[test]
    fn bold_leaves_complete() {
        assert!(matches!(handle_bold("**bold**"), Cow::Borrowed(_)));
    }

    #[test]
    fn bold_half_complete() {
        assert_eq!(handle_bold("**bold*").as_ref(), "**bold**");
    }

    #[test]
    fn bold_inside_code_block() {
        assert!(matches!(handle_bold("```\n**bold\n```"), Cow::Borrowed(_)));
    }

    #[test]
    fn bold_empty_content() {
        assert!(matches!(handle_bold("**"), Cow::Borrowed(_)));
    }

    // Italic asterisk tests
    #[test]
    fn italic_asterisk_completes() {
        assert_eq!(
            handle_italic_asterisk("*italic text").as_ref(),
            "*italic text*"
        );
    }

    #[test]
    fn italic_asterisk_leaves_complete() {
        assert!(matches!(
            handle_italic_asterisk("*italic*"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn italic_asterisk_completes_tab_content() {
        assert_eq!(handle_italic_asterisk("*0\t").as_ref(), "*0\t*");
    }

    #[test]
    fn italic_asterisk_idempotent_with_tab() {
        assert!(matches!(handle_italic_asterisk("*0\t*"), Cow::Borrowed(_)));
    }

    #[test]
    fn count_single_asterisks_counts_closer_after_tab() {
        assert_eq!(count_single_asterisks("*0\t*"), 2);
    }

    // Italic underscore tests
    #[test]
    fn italic_underscore_completes() {
        assert_eq!(
            handle_italic_underscore("_italic text").as_ref(),
            "_italic text_"
        );
    }

    #[test]
    fn italic_underscore_word_internal() {
        // user_name should not be treated as italic.
        assert!(matches!(
            handle_italic_underscore("user_name"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn italic_underscore_inserts_before_trailing_single_asterisk() {
        // Appending `_` after `0*` would make `*` word-internal between `0`
        // and `_` on the next pass. Insert `_` before the `*` instead.
        assert_eq!(handle_italic_underscore("_*>0*").as_ref(), "_*>0_*");
    }

    #[test]
    fn italic_underscore_falls_through_when_trailing_asterisk_non_word() {
        // handle_trailing_single_asterisk_for_underscore returns None (space
        // before *), so insert_closing_underscore appends _ at EOF.
        assert_eq!(handle_italic_underscore("_. *").as_ref(), "_. *_");
    }

    // Double underscore tests
    #[test]
    fn double_underscore_completes() {
        assert_eq!(
            handle_double_underscore("__italic text").as_ref(),
            "__italic text__"
        );
    }

    #[test]
    fn double_underscore_half_complete() {
        assert_eq!(handle_double_underscore("__italic_").as_ref(), "__italic__");
    }

    // Bold-italic tests
    #[test]
    fn bold_italic_completes() {
        assert_eq!(
            handle_bold_italic("***bold italic text").as_ref(),
            "***bold italic text***"
        );
    }

    #[test]
    fn bold_italic_four_asterisks() {
        assert!(matches!(handle_bold_italic("****"), Cow::Borrowed(_)));
    }

    // Tilde fence tests
    #[test]
    fn bold_inside_tilde_fence() {
        assert!(matches!(handle_bold("~~~\n**bold\n~~~"), Cow::Borrowed(_)));
    }

    #[test]
    fn italic_asterisk_inside_tilde_fence() {
        assert!(matches!(
            handle_italic_asterisk("~~~\n*italic\n~~~"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn italic_underscore_inside_tilde_fence() {
        assert!(matches!(
            handle_italic_underscore("~~~\n_italic\n~~~"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn double_underscore_inside_tilde_fence() {
        assert!(matches!(
            handle_double_underscore("~~~\n__italic\n~~~"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn bold_italic_inside_tilde_fence() {
        assert!(matches!(
            handle_bold_italic("~~~\n***bold italic\n~~~"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn four_tilde_fence_not_closed_by_three() {
        assert!(matches!(handle_bold("~~~~\n~~~\n**bold"), Cow::Borrowed(_)));
    }
}
