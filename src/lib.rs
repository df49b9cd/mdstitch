#![doc(
    html_logo_url = "https://raw.githubusercontent.com/df49b9cd/mdstitch/master/branding/mdstitch-icon.png",
    html_favicon_url = "https://raw.githubusercontent.com/df49b9cd/mdstitch/master/branding/mdstitch-icon.png"
)]
//! Streaming markdown preprocessor that auto-completes incomplete syntax.
//!
//! Runs on raw markdown strings **before** the pulldown-cmark parser, detecting
//! and closing unterminated formatting markers so content renders correctly
//! during token-by-token streaming.

mod bracket;
mod fence;
mod options;
mod ranges;
mod utils;

mod comparison_operators;
mod emphasis;
mod html_tags;
mod inline_code;
mod katex;
mod link_image;
mod setext_heading;
mod single_tilde;
mod strikethrough;

mod detect_direction;
mod incomplete_code;
mod preprocess;

pub use options::{LinkMode, StitchHandler, StitchOptions, priority};
pub use ranges::CodeBlockRanges;

// Re-export public items from internal modules.
pub use detect_direction::{TextDirection, detect_text_direction};
pub use incomplete_code::{has_incomplete_code_fence, has_table, open_fence};
pub use preprocess::{
    normalize_html_indentation, preprocess_custom_tags, preprocess_literal_tag_content,
};

// Re-export utility functions for use by custom handlers.
// These four are the most commonly needed when implementing `StitchHandler`:
// code block detection, link/image URL detection, math block detection, and
// word character classification.
pub use utils::{
    is_inside_code_block, is_within_link_or_image_url, is_within_math_block, is_word_char,
};

use std::borrow::Cow;

pub(crate) const INCOMPLETE_LINK_MARKER: &str = "](stitch:incomplete-link)";

/// Which builtin trigger families are present in the text — the per-group
/// projection of the x1 marker-absence scan (x3). When all bits are false,
/// `stitch` skips `CodeBlockRanges::new` and the whole builtin pipeline,
/// returning the input unchanged (`Cow::Borrowed`).
///
/// Option-aware: a disabled handler's trigger bytes never block the fast path.
/// Custom handlers (`!options.handlers.is_empty()`) have unknown triggers, so
/// the caller must not take the fast path when any are registered.
///
/// Trigger set (per enabled builtin, conservatively inclusive to preserve
/// parity — `]` is a trigger because the link handler rewrites the
/// `](stitch:incomplete-link)` sentinel it inserts, and `\` because handlers
/// consult `is_escaped`):
///   - emphasis / italic / bold / strikethrough / single_tilde / inline_code:
///     `*` `_` `` ` `` `~`
///   - katex / inline_katex / comparison_operators: `$`
///   - html_tags / comparison_operators: `>` `<`
///   - links / images: `[` `]` `!` `(`
///   - setext_headings: `=`
///   - escapes (consulted by `should_skip_*`): `\`
///
/// Same scans, same early exits,
/// but the per-group bits are RETURNED instead of `||`-folded, so the builtin
/// pipeline can skip handlers whose trigger byte cannot occur (a code-only
/// reply then skips the html/link/math/setext passes without a single extra
/// byte scan). `false` means "provably absent" — never a false negative.
#[derive(Clone, Copy, Default)]
struct TriggerPresence {
    /// `*` `_` `` ` `` `~` `\` — emphasis / strikethrough / inline code.
    emphasis: bool,
    /// `$` — katex / inline katex / comparison `$`.
    math: bool,
    /// `>` `<` — html tags / comparison operators.
    html: bool,
    /// `[` `]` `!` `(` — links / images.
    link: bool,
    /// setext `-`/`=` underline at a line start with <4 indent cols.
    setext: bool,
}

impl TriggerPresence {
    fn none(&self) -> bool {
        !(self.emphasis || self.math || self.html || self.link || self.setext)
    }
}

fn scan_triggers(text: &str, options: &StitchOptions) -> TriggerPresence {
    let emphasis_like = options.bold
        || options.italic
        || options.bold_italic
        || options.inline_code
        || options.strikethrough
        || options.single_tilde;
    let math_like = options.katex || options.inline_katex || options.comparison_operators;
    let html_like = options.html_tags || options.comparison_operators;
    let link_like = options.links || options.images;

    let mut p = TriggerPresence::default();

    // Per-group early-exit memchr passes (x2's SIMD scans, kept): on
    // marker-heavy text each scan exits within the first ~20 bytes, so the
    // total cost is no worse than the old short-circuit loop; on absent-group
    // text the full-width SIMD scan is ~26us/256KiB — far below one handler's
    // O(n) pass it lets us skip.
    if emphasis_like
        && (memchr::memchr3(b'*', b'_', b'`', text.as_bytes()).is_some()
            || memchr::memchr2(b'~', b'\\', text.as_bytes()).is_some())
    {
        p.emphasis = true;
    }
    if math_like && memchr::memchr(b'$', text.as_bytes()).is_some() {
        p.math = true;
    }
    if html_like && memchr::memchr2(b'>', b'<', text.as_bytes()).is_some() {
        p.html = true;
    }
    if link_like
        && (memchr::memchr3(b'[', b']', b'!', text.as_bytes()).is_some()
            || memchr::memchr(b'(', text.as_bytes()).is_some())
    {
        p.link = true;
    }

    // Setext underlines (`-`/`=`) are NOT plain byte triggers — the handler
    // only acts when one sits at the START of a line (<4 indent cols). Drive
    // the scan off newline positions via memchr — O(newlines), not O(bytes).
    if options.setext_headings {
        let bytes = text.as_bytes();
        let starts =
            std::iter::once(0usize).chain(memchr::memchr_iter(b'\n', bytes).map(|q| q + 1));
        for start in starts {
            if start >= bytes.len() {
                continue;
            }
            let line = &text[start..];
            if utils::leading_indent_cols(line) < utils::CODE_INDENT_COLS
                && matches!(
                    line.trim_start_matches([' ', '\t']).as_bytes(),
                    [b'=' | b'-', ..]
                )
            {
                p.setext = true;
                break;
            }
        }
    }
    p
}

/// Preprocesses streaming markdown text, auto-completing any incomplete syntax.
///
/// Returns `Cow::Borrowed` when no changes are needed (zero-allocation fast path).
pub fn stitch<'a>(text: &'a str, options: &StitchOptions) -> Cow<'a, str> {
    if text.is_empty() {
        return Cow::Borrowed(text);
    }

    // Strip trailing single space (preserve double space for line breaks).
    let initial: Cow<'a, str> = if text.ends_with(' ') && !text.ends_with("  ") {
        Cow::Borrowed(&text[..text.len() - 1])
    } else {
        Cow::Borrowed(text)
    };

    let first = run_pipeline_entry(initial, options);

    if options.handlers.is_empty() {
        // Idempotency fixed point: re-run the builtin pipeline until two
        // consecutive passes agree. When handlers oscillate cross-state
        // (`` ``_*>__`` / `$*A**\n` families), fall back to the smallest
        // snapshot reached that is itself a fixed point (verify by one
        // extra stitch). The verify keeps the guarantee honest: any
        // returned `result` satisfies `stitch(result) == result`.
        use std::collections::HashSet;
        let mut seen: HashSet<String> = HashSet::new();
        let seed = text.to_owned();
        let mut current = match first {
            Cow::Borrowed(b) if std::ptr::eq(b, text) => seed.clone(),
            Cow::Borrowed(b) => b.to_owned(),
            Cow::Owned(s) => s,
        };
        seen.insert(current.clone());
        let mut candidates: Vec<String> = vec![current.clone()];
        let mut converged = false;
        for _ in 0..8 {
            let next = run_pipeline_entry(Cow::Borrowed(current.as_str()), options).into_owned();
            if next == current {
                converged = true;
                break;
            }
            if !seen.insert(next.clone()) {
                break;
            }
            candidates.push(next.clone());
            current = next;
        }
        let result = if converged {
            current
        } else {
            // Find the smallest candidate that fixes itself under stitch.
            // If none do (theoretical — the gate layers should have made at
            // least one of them stable), return the smallest, which is the
            // state closest to the input.
            candidates.sort_by_key(|c| c.len());
            candidates
                .iter()
                .find(|c| {
                    run_pipeline_entry(Cow::Borrowed(c.as_str()), options).as_ref() == c.as_str()
                })
                .cloned()
                .unwrap_or_else(|| candidates[0].clone())
        };
        if result == seed {
            return Cow::Borrowed(text);
        }
        return Cow::Owned(result);
    }

    first
}

/// Single pipeline invocation: fast-path trigger scan + builtin stages.
fn run_pipeline_entry<'a>(initial: Cow<'a, str>, options: &StitchOptions) -> Cow<'a, str> {
    // Marker-absence fast path: if no enabled builtin handler has a trigger
    // byte in the text, there is nothing to complete and no code/math/region
    // to track — skip the O(n) `CodeBlockRanges::new` (6 full-text scans) and
    // every handler pass, returning the input unchanged. Custom handlers have
    // unknown triggers, so this only fires on the builtin-only pipeline.
    let presence = if options.handlers.is_empty() {
        let p = scan_triggers(initial.as_ref(), options);
        if p.none() {
            return initial;
        }
        p
    } else {
        // Custom handlers may introduce any trigger byte — keep every gate open.
        TriggerPresence::default()
    };

    run_pipeline(initial, options, presence)
}

/// A shared `CodeBlockRanges`, built lazily on first use and invalidated after
/// any handler that may rewrite bytes in the middle of the string (custom
/// handlers are opaque mutators). Invalidation is by pointer-compare of the
/// before/after `Cow` payloads — cheap and conservative (a fresh buffer never
/// shifts detection by address).
#[derive(Default)]
struct SharedRanges(Option<ranges::CodeBlockRanges>);

impl SharedRanges {
    /// Returns a reference to the ranges, squinting them over `text` if absent.
    fn get_or_init<'r>(&'r mut self, text: &str) -> &'r ranges::CodeBlockRanges {
        self.0
            .get_or_insert_with(|| ranges::CodeBlockRanges::new(text))
    }

    /// Drop the cached ranges if the handler moved `result` to a new buffer.
    fn invalidate_if_moved(&mut self, before: *const u8, result: &str) {
        if !std::ptr::eq(result.as_ptr(), before) {
            self.0 = None;
        }
    }

    /// Opaque-mutator invalidation: always drop after a custom handler ran.
    fn invalidate(&mut self) {
        self.0 = None;
    }
}

/// A built-in pipeline stage. `BUILTIN_ORDER` (below) is the single source of
/// truth for builtin execution order in BOTH the no-custom-handlers fast path
/// and the custom+priority-merged path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Builtin {
    ComparisonOperators,
    HtmlTags,
    SetextHeadings,
    Links,
    /// Re-run `html_tags` on link-unwrapped residue (`[<a](` → `<a`), so the
    /// pipeline reaches its fixed point in one call. Only fires after links
    /// actually rewrote text in `TextOnly` mode.
    HtmlRerunAfterLinks,
    InlineCode,
    SingleTilde,
    ItalicBoldItalic,
    ItalicBold,
    ItalicDoubleUnderscore,
    ItalicSingleAsterisk,
    ItalicSingleUnderscore,
    Strikethrough,
    KatexBlock,
    KatexInline,
}

/// Which trigger-group gate (from `TriggerPresence`) a builtin stage waits on.
/// A stage is skipped when the gate's presence bit is false (provably no
/// trigger byte in the input). The custom-handlers path opens every gate,
/// since custom handlers' triggers are by definition unknown.
#[derive(Clone, Copy)]
enum Gate {
    Emphasis,
    Math,
    Html,
    Link,
    Setext,
}

impl Builtin {
    fn priority(self) -> i32 {
        match self {
            Builtin::ComparisonOperators => options::priority::COMPARISON_OPERATORS,
            Builtin::HtmlTags => options::priority::HTML_TAGS,
            Builtin::SetextHeadings => options::priority::SETEXT_HEADINGS,
            Builtin::Links => options::priority::LINKS,
            Builtin::HtmlRerunAfterLinks => options::priority::LINKS + 1,
            Builtin::InlineCode => options::priority::INLINE_CODE,
            Builtin::SingleTilde => options::priority::SINGLE_TILDE,
            Builtin::ItalicBoldItalic => options::priority::BOLD_ITALIC,
            Builtin::ItalicBold => options::priority::BOLD,
            Builtin::ItalicDoubleUnderscore => options::priority::ITALIC_DOUBLE_UNDERSCORE,
            Builtin::ItalicSingleAsterisk => options::priority::ITALIC_SINGLE_ASTERISK,
            Builtin::ItalicSingleUnderscore => options::priority::ITALIC_SINGLE_UNDERSCORE,
            Builtin::Strikethrough => options::priority::STRIKETHROUGH,
            Builtin::KatexBlock => options::priority::KATEX,
            Builtin::KatexInline => options::priority::INLINE_KATEX,
        }
    }

    fn enabled(self, o: &StitchOptions) -> bool {
        match self {
            Builtin::ComparisonOperators => o.comparison_operators,
            Builtin::HtmlTags => o.html_tags,
            Builtin::SetextHeadings => o.setext_headings,
            Builtin::Links => o.links || o.images,
            // Compensating pass for Links — only relevant when both run.
            Builtin::HtmlRerunAfterLinks => o.html_tags && (o.links || o.images),
            Builtin::InlineCode => o.inline_code,
            Builtin::SingleTilde => o.single_tilde,
            Builtin::ItalicBoldItalic => o.bold_italic,
            Builtin::ItalicBold => o.bold,
            Builtin::ItalicDoubleUnderscore => o.italic,
            Builtin::ItalicSingleAsterisk => o.italic,
            Builtin::ItalicSingleUnderscore => o.italic,
            Builtin::Strikethrough => o.strikethrough,
            Builtin::KatexBlock => o.katex,
            Builtin::KatexInline => o.inline_katex,
        }
    }

    fn gate(self) -> Gate {
        match self {
            Builtin::ComparisonOperators => Gate::Html,
            Builtin::HtmlTags => Gate::Html,
            // The html-rerun only fires when links unwrapped a `[`, which
            // proves `<` appeared in the source (gate mirrors HtmlTags).
            Builtin::HtmlRerunAfterLinks => Gate::Html,
            Builtin::SetextHeadings => Gate::Setext,
            Builtin::Links => Gate::Link,
            // inline_code runs BEFORE emphasis so emphasis handlers see closed
            // code spans (idempotency; proptest `"*A***`a"`); its trigger byte
            // is the backtick, folded into the emphasis group.
            Builtin::InlineCode => Gate::Emphasis,
            // single_tilde runs AFTER links: TextOnly unwrapping exposes lone
            // `~`s the emphasis-group trigger already covers (it scans for `~`).
            Builtin::SingleTilde => Gate::Emphasis,
            Builtin::ItalicBoldItalic => Gate::Emphasis,
            Builtin::ItalicBold => Gate::Emphasis,
            Builtin::ItalicDoubleUnderscore => Gate::Emphasis,
            Builtin::ItalicSingleAsterisk => Gate::Emphasis,
            Builtin::ItalicSingleUnderscore => Gate::Emphasis,
            Builtin::Strikethrough => Gate::Emphasis,
            Builtin::KatexBlock => Gate::Math,
            Builtin::KatexInline => Gate::Math,
        }
    }

    /// Apply the stage to `result`, threading `shared` ranges and re-running
    /// html_tags' fixed-point pass where required. `Stage::EarlyReturn` signals
    /// the protocol-mode links sentinel.
    fn run<'a>(
        self,
        mut result: Cow<'a, str>,
        options: &StitchOptions,
        shared: &mut SharedRanges,
        gate_open: bool,
    ) -> Stage<'a> {
        if !gate_open {
            return Stage::Next(result);
        }
        match self {
            Builtin::ComparisonOperators => {
                Stage::Next(apply(result, comparison_operators::handle))
            }
            Builtin::HtmlTags => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| html_tags::handle_with_ranges(t, r)))
            }
            Builtin::SetextHeadings => Stage::Next(apply(result, setext_heading::handle)),
            Builtin::Links => {
                let link_mode = options.link_mode;
                let links_enabled = options.links;
                let images_enabled = options.images;
                let before = result.as_ref().as_ptr();
                let r = shared.get_or_init(&result);
                result = apply_with(result, |t| {
                    link_image::handle_with_ranges(t, link_mode, links_enabled, images_enabled, r)
                });
                if link_mode == options::LinkMode::Protocol
                    && result.ends_with(INCOMPLETE_LINK_MARKER)
                {
                    return Stage::EarlyReturn(result);
                }
                if link_mode == options::LinkMode::TextOnly {
                    shared.invalidate_if_moved(before, &result);
                }
                Stage::Next(result)
            }
            Builtin::HtmlRerunAfterLinks => {
                // Only meaningful after a TextOnly rewrite (the only links
                // mutation that can expose a mid-text `<`). With Protocol mode
                // nothing shifts, so the first HtmlTags pass already saw a
                // final string; skip.
                if options.link_mode == options::LinkMode::TextOnly {
                    let r = shared.get_or_init(&result);
                    result = apply_with(result, |t| html_tags::handle_with_ranges(t, r));
                }
                Stage::Next(result)
            }
            Builtin::InlineCode => Stage::Next(apply(result, inline_code::handle)),
            Builtin::SingleTilde => Stage::Next(apply(result, single_tilde::handle)),
            Builtin::ItalicBoldItalic => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    emphasis::handle_bold_italic_with_ranges(t, r)
                }))
            }
            Builtin::ItalicBold => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    emphasis::handle_bold_with_ranges(t, r)
                }))
            }
            Builtin::ItalicDoubleUnderscore => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    emphasis::handle_double_underscore_with_ranges(t, r)
                }))
            }
            Builtin::ItalicSingleAsterisk => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    emphasis::handle_italic_asterisk_with_ranges(t, r)
                }))
            }
            Builtin::ItalicSingleUnderscore => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    emphasis::handle_italic_underscore_with_ranges(t, r)
                }))
            }
            Builtin::Strikethrough => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    strikethrough::handle_with_ranges(t, r)
                }))
            }
            Builtin::KatexBlock => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    katex::handle_block_with_ranges(t, r)
                }))
            }
            Builtin::KatexInline => {
                let r = shared.get_or_init(&result);
                Stage::Next(apply_with(result, |t| {
                    katex::handle_inline_with_ranges(t, r)
                }))
            }
        }
    }
}

/// Builtin execution order — ascending `priority()` except for the inline_code
/// (26) / single_tilde (25) swap, which the idempotency regression
/// `"*A***`a"` needs (see the doc on `options::priority::INLINE_CODE`).
/// When custom handlers join, they merge into this order by priority; the
/// custom sort is stable, so the relative order of equal-priority builtins is
/// preserved as written here.
const BUILTIN_ORDER: &[Builtin] = &[
    Builtin::ComparisonOperators,
    Builtin::HtmlTags,
    Builtin::SetextHeadings,
    Builtin::Links,
    Builtin::HtmlRerunAfterLinks,
    Builtin::InlineCode,
    Builtin::SingleTilde,
    Builtin::ItalicBoldItalic,
    Builtin::ItalicBold,
    Builtin::ItalicDoubleUnderscore,
    Builtin::ItalicSingleAsterisk,
    Builtin::ItalicSingleUnderscore,
    Builtin::Strikethrough,
    Builtin::KatexBlock,
    Builtin::KatexInline,
];

/// Outcome of running one stage: `Next` continues the pipeline; `EarlyReturn`
/// is the Protocol-mode incomplete-link sentinel, and the pipeline must stop
/// before downstream handlers mangle the placeholder.
enum Stage<'a> {
    Next(Cow<'a, str>),
    EarlyReturn(Cow<'a, str>),
}

impl Builtin {
    fn gate_open(self, presence: TriggerPresence) -> bool {
        match self.gate() {
            Gate::Emphasis => presence.emphasis,
            Gate::Math => presence.math,
            Gate::Html => presence.html,
            Gate::Link => presence.link,
            Gate::Setext => presence.setext,
        }
    }
}

/// Which pipeline mode: no-custom fast path (per-stage gates consulted) or
/// custom-merged path (interleaved by priority; gates treated as OPEN because
/// custom handlers have unknown triggers — matches the original semantics).
fn run_pipeline<'a>(
    mut result: Cow<'a, str>,
    options: &StitchOptions,
    presence: TriggerPresence,
) -> Cow<'a, str> {
    let mut shared = SharedRanges::default();
    let custom = !options.handlers.is_empty();

    // Customs sorted by priority (stable → ties keep registration order, as
    // did the old pipeline's `sort_by_key` over push order).
    let mut customs: Vec<&dyn StitchHandler> = options.handlers.iter().map(|h| &**h).collect();
    customs.sort_by_key(|h| h.priority());
    let mut custom_idx = 0usize;

    // Builtin stages in `BUILTIN_ORDER`, spliced with customs at each custom's
    // `priority()`. A custom at exactly the builtin's priority runs AFTER the
    // builtin (matches the old "builtins pushed first, customs pushed last,
    // stable sort" ordering).
    for &b in BUILTIN_ORDER {
        while let Some(&h) = customs.get(custom_idx)
            && h.priority() < b.priority()
        {
            result = apply_with(result, |t| h.handle(t));
            shared.invalidate();
            custom_idx += 1;
        }
        if !b.enabled(options) {
            continue;
        }
        // Gate: fast path consults the trigger scan; the custom path leaves
        // every gate open (customs may introduce triggers mid-pipeline).
        let open = custom || b.gate_open(presence);
        match b.run(result, options, &mut shared, open) {
            Stage::Next(next) => result = next,
            Stage::EarlyReturn(done) => return done,
        }
    }
    if custom {
        // Trailing customs (priority above every builtin's).
        while let Some(&h) = customs.get(custom_idx) {
            result = apply_with(result, |t| h.handle(t));
            custom_idx += 1;
        }
    }
    result
}

/// Applies a handler to a `Cow<str>`, threading ownership efficiently.
fn apply<'a>(input: Cow<'a, str>, handler: fn(&str) -> Cow<'_, str>) -> Cow<'a, str> {
    apply_with(input, handler)
}

/// Applies a closure handler to a `Cow<str>`, threading ownership efficiently.
fn apply_with<'a>(input: Cow<'a, str>, handler: impl FnOnce(&str) -> Cow<'_, str>) -> Cow<'a, str> {
    match handler(&input) {
        Cow::Borrowed(b) if std::ptr::eq(b, input.as_ref() as &str) => {
            // Handler returned its input unchanged — preserve the original Cow.
            input
        }
        Cow::Borrowed(b) => {
            // Handler returned a borrowed sub-slice (e.g. trimmed) — must own it.
            Cow::Owned(b.to_owned())
        }
        Cow::Owned(s) => Cow::Owned(s),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pipeline_tests {
    use super::BUILTIN_ORDER;

    /// Execution order must match ascending `priority()` — the priority sort
    /// in the custom-handler path is only correct if the base table already
    /// ships sorted.
    #[test]
    fn builtin_order_is_priority_sorted() {
        let mut prev = i32::MIN;
        for &b in BUILTIN_ORDER {
            assert!(
                b.priority() >= prev,
                "BUILTIN_ORDER entry {b:?} has priority {} after {prev}",
                b.priority()
            );
            prev = b.priority();
        }
        // Sanity: the table and the enum's variants are in 1:1 correspondence
        // (minus none, plus the html-rerun pseudo-stage).
        assert_eq!(BUILTIN_ORDER.len(), 15);
    }
}
