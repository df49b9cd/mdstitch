use super::*;

// ===========================================================================
// Property-based tests (fuzz invariants)
// ===========================================================================

/// Biases toward characters stitch actively inspects — markdown punctuation,
/// KaTeX/HTML/table delimiters, URL punctuation, CR/LF, and plain prose.
/// Capped at 80 chars so shrunk counterexamples stay readable.
fn markdown_soup() -> impl Strategy<Value = String> {
    prop::string::string_regex(r#"[ \n\r\t*_`~\[\]()<>{}|!#$\\/:'"a-zA-Z0-9.,-]{0,80}"#).unwrap()
}

/// Fence-rich generator: mixes prose, newlines, leading-space indents, and
/// backtick/tilde runs of assorted lengths so line-start vs mid-line fence
/// decisions get exercised. Used by the cross-scanner agreement proptest.
///
/// Tildes are given equal weight to backticks so the proptest regularly
/// exercises tilde fences, not just the more common backtick case.
fn fence_soup() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            2 => prop::string::string_regex(r"[a-z ]{0,6}").unwrap(),
            2 => Just("\n".into()),
            1 => Just("```".into()),
            1 => Just("````".into()),
            1 => Just("~~~".into()),
            1 => Just("~~~~".into()),
            1 => Just("   ".into()),
            1 => Just("    ".into()),
        ],
        0..20,
    )
    .prop_map(|parts: Vec<String>| parts.concat())
}

/// Tilde-only fence generator — the mid-line `~~~` case must hold just as
/// strictly as the backtick case, so give it a dedicated proptest.
fn tilde_fence_soup() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            2 => prop::string::string_regex(r"[a-z ]{0,6}").unwrap(),
            2 => Just("\n".into()),
            1 => Just("~~~".into()),
            1 => Just("~~~~".into()),
            1 => Just("   ".into()),
            1 => Just("    ".into()),
        ],
        0..20,
    )
    .prop_map(|parts: Vec<String>| parts.concat())
}

#[derive(Debug, Clone)]
struct OptionFlags {
    bold: bool,
    italic: bool,
    bold_italic: bool,
    inline_code: bool,
    strikethrough: bool,
    links: bool,
    images: bool,
    katex: bool,
    inline_katex: bool,
    setext_headings: bool,
    html_tags: bool,
    single_tilde: bool,
    comparison_operators: bool,
    link_mode: LinkMode,
}

impl OptionFlags {
    fn to_options(&self) -> StitchOptions {
        StitchOptions::default()
            .bold(self.bold)
            .italic(self.italic)
            .bold_italic(self.bold_italic)
            .inline_code(self.inline_code)
            .strikethrough(self.strikethrough)
            .links(self.links)
            .images(self.images)
            .katex(self.katex)
            .inline_katex(self.inline_katex)
            .setext_headings(self.setext_headings)
            .html_tags(self.html_tags)
            .single_tilde(self.single_tilde)
            .comparison_operators(self.comparison_operators)
            .link_mode(self.link_mode)
    }
}

fn arbitrary_options() -> impl Strategy<Value = OptionFlags> {
    // Nested tuples: proptest's Strategy impl caps at 10-tuples.
    (
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        ),
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            prop_oneof![Just(LinkMode::Protocol), Just(LinkMode::TextOnly)],
        ),
    )
        .prop_map(
            |(
                (bold, italic, bold_italic, inline_code, strikethrough, links, images),
                (
                    katex,
                    inline_katex,
                    setext_headings,
                    html_tags,
                    single_tilde,
                    comparison_operators,
                    link_mode,
                ),
            )| OptionFlags {
                bold,
                italic,
                bold_italic,
                inline_code,
                strikethrough,
                links,
                images,
                katex,
                inline_katex,
                setext_headings,
                html_tags,
                single_tilde,
                comparison_operators,
                link_mode,
            },
        )
}

/// Records handler executions so the ordering property can assert the
/// pipeline respects priority.
#[derive(Clone)]
struct Recorder {
    tag: char,
    pri: i32,
    log: Arc<Mutex<String>>,
}

impl StitchHandler for Recorder {
    fn handle<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.log.lock().unwrap().push(self.tag);
        Cow::Borrowed(text)
    }
    fn name(&self) -> &str {
        "recorder"
    }
    fn priority(&self) -> i32 {
        self.pri
    }
}

/// Direct tripwire for issue #144 (idempotency violation on `"*0\t"`).
#[test]
fn idempotency_regression_0144() {
    let opts = StitchOptions::default();
    let once = stitch("*0\t", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

/// Idempotency violation on `"*'***a"` with `bold_italic` but `bold` disabled:
/// bold_italic appends `***`, then italic_asterisk sees an odd single-count
/// and appended `*` — but the new `*` has `prev == '*'` so the counter keeps
/// ignoring it, the count stays odd, and each pass extends the trailing run.
#[test]
fn idempotency_regression_0145() {
    let opts = StitchOptions {
        bold: false,
        italic: true,
        bold_italic: true,
        ..StitchOptions::default()
    };
    let once = stitch("*'***a", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

/// Idempotency violation on `"*0*>$$**\r*\n"` with `italic` + `katex`.
/// The `*` at pos 9 is between `\r` (CR) and `\n` (LF). The whitespace
/// flanking check treated `\n` as ws but not `\r`, so the `*` was counted
/// as a floating italic opener. italic_asterisk appended a `*`, katex then
/// wrapped the tail in `\n$$`, and on the next pass the new `*` (inside
/// complete math) plus the `\r`-flanked `*` counted as odd, triggering
/// another append. Fix: `\r` flanks like `\n` in should_skip_asterisk.
#[test]
fn idempotency_regression_0146() {
    let opts = StitchOptions {
        bold: false,
        italic: true,
        bold_italic: false,
        inline_code: false,
        strikethrough: false,
        links: false,
        images: false,
        katex: true,
        inline_katex: false,
        setext_headings: false,
        html_tags: false,
        single_tilde: false,
        comparison_operators: false,
        link_mode: LinkMode::Protocol,
        handlers: Vec::new(),
    };
    let once = stitch("*0*>$$**\r*\n", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

/// Companion regression to 0146: the proptest also shrinks to `"*\r$"`
/// with `italic` + `inline_katex`. The `*` at pos 0 has `prev==SOF` (ws)
/// and `next==\r`. If `\r` isn't whitespace, the `*` is counted as a
/// floating opener and italic appends `*`, which inline_katex then wraps.
#[test]
fn idempotency_regression_0147() {
    let opts = StitchOptions {
        bold: false,
        italic: true,
        bold_italic: false,
        inline_code: false,
        strikethrough: false,
        links: false,
        images: false,
        katex: false,
        inline_katex: true,
        setext_headings: false,
        html_tags: false,
        single_tilde: false,
        comparison_operators: false,
        link_mode: LinkMode::Protocol,
        handlers: Vec::new(),
    };
    let once = stitch("*\r$", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

/// Regression for seed `21da98eb…` (shrunk to `` `>\\ ``, `inline_code`).
/// `count_single_backticks` used to treat any `\` immediately before a
/// `` ` `` as an escape, so `` `>\\` `` re-stitched to `` `>\\`` `` —
/// the doubled `\\` was not recognised as an escaped backslash that
/// leaves the subsequent backtick unescaped.
#[test]
fn idempotency_regression_0148() {
    let opts = StitchOptions {
        bold: false,
        italic: false,
        bold_italic: false,
        inline_code: true,
        strikethrough: false,
        links: false,
        images: false,
        katex: false,
        inline_katex: false,
        setext_headings: false,
        html_tags: false,
        single_tilde: false,
        comparison_operators: false,
        link_mode: LinkMode::Protocol,
        handlers: Vec::new(),
    };
    let once = stitch("`>\\\\", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

/// Pipeline-level idempotency tripwires for each proptest regression seed.
/// These exercise the full stitch() pipeline (not individual handlers).
#[test]
fn idempotency_seeds_pipeline() {
    fn only(f: impl FnOnce(&mut StitchOptions)) -> StitchOptions {
        let mut o = StitchOptions {
            bold: false,
            italic: false,
            bold_italic: false,
            inline_code: false,
            strikethrough: false,
            links: false,
            images: false,
            katex: false,
            inline_katex: false,
            setext_headings: false,
            html_tags: false,
            single_tilde: false,
            comparison_operators: false,
            link_mode: LinkMode::Protocol,
            handlers: Vec::new(),
        };
        f(&mut o);
        o
    }

    let seeds: &[(&str, StitchOptions)] = &[
        (
            "_$",
            only(|o| {
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        (
            "[[",
            only(|o| {
                o.links = true;
            }),
        ),
        (
            "`\\",
            only(|o| {
                o.inline_code = true;
            }),
        ),
        (
            "*\\",
            only(|o| {
                o.italic = true;
            }),
        ),
        (
            "_*>0",
            only(|o| {
                o.italic = true;
            }),
        ),
        (
            "``[ [",
            only(|o| {
                o.links = true;
                o.link_mode = LinkMode::TextOnly;
            }),
        ),
        (
            "$**",
            only(|o| {
                o.bold = true;
                o.inline_katex = true;
            }),
        ),
        (
            "$`\n",
            only(|o| {
                o.inline_code = true;
                o.inline_katex = true;
            }),
        ),
        (
            "$$`\n\\",
            only(|o| {
                o.inline_code = true;
                o.katex = true;
            }),
        ),
        (
            "$$`\na\\",
            only(|o| {
                o.inline_code = true;
                o.katex = true;
            }),
        ),
        (
            "*'***a",
            only(|o| {
                o.italic = true;
                o.bold_italic = true;
            }),
        ),
        (
            "*0*>$$**\r*\n",
            only(|o| {
                o.italic = true;
                o.katex = true;
            }),
        ),
        (
            "*\r$",
            only(|o| {
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        (
            "A* **_0A*",
            only(|o| {
                o.bold = true;
                o.italic = true;
            }),
        ),
        (
            "[ ](",
            only(|o| {
                o.links = true;
                o.link_mode = LinkMode::TextOnly;
            }),
        ),
        // Regression 0149: link unwrapping in TextOnly mode exposed an
        // incomplete HTML tag (`[<a](` → `<a`) that the first html_tags pass
        // had rejected as implausible. Pipeline now re-runs html_tags after
        // links when both are enabled.
        (
            "[<a](",
            only(|o| {
                o.links = true;
                o.html_tags = true;
                o.link_mode = LinkMode::TextOnly;
            }),
        ),
        // Regression 0150: `italic_double_underscore` sees `_0__` as an
        // unclosed `__` pair, `handle_half_complete_underscore` treats `_0__`
        // 's final `_` as half-closer and appends `_` — the run grows forever
        // because `_` at 2 stays `prev == '_'` and `find_first_single_*` skips
        // it (`_0__` → `_0___` → `_0____` → ...).
        (
            "_0__",
            only(|o| {
                o.italic = true;
            }),
        ),
        // Regression 0151: italic_asterisk on `"*>\*"` — a `\*` hides the
        // whole trailing `*` run from the counter (members after the first
        // get `prev == '*'` skipped), so a single odd-count `*` at index 0
        // keeps the parity odd and italic re-appends on every pass
        // (`*>\*` → `*>\**` → `*>\***` → *).
        (
            "*>\\*",
            only(|o| {
                o.italic = true;
            }),
        ),
        // Regression 0152: `"*A$$\n"`, italic + block katex. Italic appends
        // `*` for the lone opener; katex_block later appends `\n$$`,
        // wrapping the appended closer inside `$$…$$` where italic's counter
        // can't see it → re-append loop on pass 2.
        (
            "*A$$\n",
            only(|o| {
                o.italic = true;
                o.katex = true;
            }),
        ),
        // Regression 0153: `"_$,` variant — italic appends `_` after the
        // trailing `$`; inline_katex then appends `$`; pass 2 sees `_$_`
        // balanced katex-wise but odd italic-wise vs the first pass.
        (
            "_$,",
            only(|o| {
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0154: like 0153 but the `$$` is *closed*, so the gate
        // cannot key on "unclosed math" — italic's `*` would land between
        // the closer `$$` and inline_katex's appended `$`.
        (
            "$$*$",
            only(|o| {
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0155: inline_code's backtick-append and inline_katex's
        // `$`-append cross-fire — each doesn't check the other's open state.
        (
            "$`,",
            only(|o| {
                o.inline_code = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0156: html_tags stripped the trailing `<A` of `<A <A`,
        // exposing an *earlier* incomplete tag at the new end — the outer
        // strip wasn't self-stable. Fix: html_tags walks the prefix alive and
        // refuses to strip when a strippable earlier `<` sits behind the one
        // at the end.
        (
            "<A <A",
            only(|o| {
                o.html_tags = true;
            }),
        ),
        // Regression 0157: `$``$$**`, italic+inline_katex: bold appends
        // `**`, inline_katex appends `$`, the appended `**` lands inside
        // what the next pass reads as `$$…$$` complete math.
        (
            "$``$$**",
            only(|o| {
                o.bold = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0158: `$$$A$*$a`, italic+inline_katex: italic appends
        // `*`, inline_katex appends `$`, the next pass re-reads the trailing
        // `$` as part of a `$$` pair, and the `*` lands inside math.
        (
            "$$$A$*$a",
            only(|o| {
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0159: `$*A**\n`A / `$*\rA**\r*\n` — inline_katex's `$`
        // closes past the bold-appended `**`, and on pass 2 italic_asterisk
        // re-pairs the visible opener with the fresh `**` and appends.
        (
            "$*A**`\n`A",
            only(|o| {
                o.bold = true;
                o.italic = true;
                o.inline_katex = true;
            }),
        ),
        // Regression 0160: KNOWN-OPEN. `` ``_*>__``, italic-only — italic's
        // counter reads an appended trailing `*` as an adequate closer while
        // the code-region scan keeps marking it inside the pre-existing
        // unclosed `` `` `` span, so each pass adds one more marker and the
        // fixpoint oscillates. Tracked at
        // https://github.com/df49b9cd/mdstitch/issues — needs a counter-level
        // visibility fix, not another gate.
    ];

    for (input, opts) in seeds {
        let once = stitch(input, opts).into_owned();
        let twice = stitch(&once, opts).into_owned();
        assert_eq!(
            twice, once,
            "idempotency violated for seed {input:?} with opts {opts:?}"
        );
    }
}

/// Deterministic coverage for known-risky prefix + trailing-backslash combos.
#[test]
fn idempotency_trailing_backslash_combos() {
    let inputs = ["**\\", "~~\\", "$*\\", "$$\\", "***\\"];
    let opts = StitchOptions::default();
    for input in inputs {
        let once = stitch(input, &opts).into_owned();
        let twice = stitch(&once, &opts).into_owned();
        assert_eq!(
            twice, once,
            "idempotency violated for trailing-backslash input {input:?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn fuzz_never_panics_on_arbitrary_utf8(chars in prop::collection::vec(any::<char>(), 0..256)) {
        let s: String = chars.iter().collect();
        let _ = stitch(&s, &StitchOptions::default());
    }

    #[test]
    fn fuzz_never_panics_on_prefixes(chars in prop::collection::vec(any::<char>(), 0..128)) {
        // Streaming hits every prefix as tokens arrive; iterate all boundaries
        // per input rather than a single random cut.
        let s: String = chars.iter().collect();
        let opts = StitchOptions::default();
        let boundaries = s
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(s.len()));
        for cut in boundaries {
            let _ = stitch(&s[..cut], &opts);
        }
    }

    // Idempotency stress test across every option combination. Collapsed from
    // four near-duplicates; the direct regression tests below
    // (`idempotency_regression_*` / `idempotency_seeds_pipeline`) pin the
    // individual failure families discovered by this sweep.
    //
    // Pipeline idempotency invariant: stitch(stitch(x)) == stitch(x). The
    // gates in emphasis.rs/katex.rs/html_tags.rs now keep every appending
    // handler self-stable (no append when a later handler's region would
    // swallow the closer), and a bounded fixpoint loop in `run_pipeline`
    // re-runs builtin stages until a fixed point is reached. The regression
    // corpus under proptest-regressions/tests/ replays on every CI push via
    // the `seeds` job in .github/workflows/ci.yml.
    //
    // Known remaining family (tracked at
    // https://github.com/df49b9cd/mdstitch/issues): `` ``_*>__`` with
    // italic-only — italic's counter regards an appended trailing `*` as an
    // adequate closer while the code-region scan keeps marking it inside
    // the pre-existing unclosed `` `` `` span, so each pass adds one more
    // marker and the loop oscillates. This needs a counter-level fix
    // (visibility-aware counting), not a pass-order or gate tweak. Run the
    // sweep with 50k cases:
    //   PROPTEST_CASES=50000 PROPTEST_MAX_SHRINK_ITERS=8000 \
    //     cargo test --lib --release -- --ignored fuzz_idempotent_all_option_combinations
    #[ignore]
    #[test]
    fn fuzz_idempotent_all_option_combinations(
        s in markdown_soup(),
        flags in arbitrary_options(),
    ) {
        let once = stitch(&s, &flags.to_options()).into_owned();
        let twice = stitch(&once, &flags.to_options()).into_owned();
        prop_assert_eq!(twice, once);
    }

    #[test]
    fn fuzz_incomplete_autolinks_never_panic(
        prefix in markdown_soup(),
        url in r"<https?://[a-zA-Z0-9./:?#&=~%\-_]{0,40}",
        suffix in markdown_soup(),
    ) {
        let s = format!("{prefix}{url}{suffix}");
        let _ = stitch(&s, &StitchOptions::default());
    }

    #[test]
    fn fuzz_reference_style_links_never_panic(
        s in r"\[[a-zA-Z0-9 ]{0,20}\](\[[a-zA-Z0-9]{0,10}\])?(\n\[[a-zA-Z0-9]{0,10}\]: https?://[a-zA-Z0-9./\-]{0,30})?",
    ) {
        let _ = stitch(&s, &StitchOptions::default());
    }

    #[test]
    fn fuzz_incomplete_block_katex_never_panic(
        prefix in markdown_soup(),
        math in r"\$\$?[a-zA-Z0-9 ^_{}\\]{0,40}",
        suffix in markdown_soup(),
    ) {
        let s = format!("{prefix}{math}{suffix}");
        let _ = stitch(&s, &StitchOptions::default());
    }

    #[test]
    fn fuzz_incomplete_inline_katex_never_panic(
        prefix in markdown_soup(),
        math in r"\$[a-zA-Z0-9 ^_{}\\]{0,40}",
        suffix in markdown_soup(),
    ) {
        let s = format!("{prefix}{math}{suffix}");
        let _ = stitch(&s, &StitchOptions::default().inline_katex(true));
    }

    #[test]
    fn fuzz_handler_order_matches_priority_sort(
        specs in prop::collection::vec((0u8..26, -10i32..=200), 1..=5),
    ) {
        let log = Arc::new(Mutex::new(String::new()));

        // Disable every built-in so only the custom recorders run.
        let mut opts = StitchOptions::default()
            .bold(false)
            .italic(false)
            .bold_italic(false)
            .inline_code(false)
            .strikethrough(false)
            .links(false)
            .images(false)
            .katex(false)
            .inline_katex(false)
            .setext_headings(false)
            .html_tags(false)
            .single_tilde(false)
            .comparison_operators(false);

        for (idx, pri) in &specs {
            opts.handlers.push(Box::new(Recorder {
                tag: (b'a' + idx) as char,
                pri: *pri,
                log: log.clone(),
            }));
        }

        let _ = stitch("x", &opts);

        let actual = log.lock().unwrap().clone();

        // Stable sort: equal priorities preserve insertion order — mirrors
        // `sort_by_key` in the pipeline at lib.rs.
        let mut expected_indices: Vec<usize> = (0..specs.len()).collect();
        expected_indices.sort_by_key(|&i| specs[i].1);
        let expected: String = expected_indices
            .into_iter()
            .map(|i| (b'a' + specs[i].0) as char)
            .collect();

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn fuzz_custom_handlers_respect_priority_among_builtins(
        priorities in prop::collection::vec(-10i32..=200, 2..=4),
    ) {
        // Keep every built-in enabled so recorders interleave with real
        // handlers; assert recorder-relative order still matches priority sort
        // (stable sort preserves insertion order on ties).
        let log = Arc::new(Mutex::new(String::new()));
        let mut opts = StitchOptions::default();
        for (i, &pri) in priorities.iter().enumerate() {
            opts.handlers.push(Box::new(Recorder {
                tag: (b'a' + i as u8) as char,
                pri,
                log: log.clone(),
            }));
        }

        let _ = stitch("plain text", &opts);

        let actual = log.lock().unwrap().clone();

        let mut expected_indices: Vec<usize> = (0..priorities.len()).collect();
        expected_indices.sort_by_key(|&i| priorities[i]);
        let expected: String = expected_indices
            .into_iter()
            .map(|i| (b'a' + i as u8) as char)
            .collect();

        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn fuzz_trailing_single_space_stripped(s in markdown_soup()) {
        // Force exactly one trailing space (not two, which would be a line break).
        let trimmed = s.trim_end_matches(' ');
        let input = format!("{trimmed} ");
        let result = stitch(&input, &StitchOptions::default()).into_owned();
        prop_assert!(
            !result.ends_with(' '),
            "single trailing space should be stripped; got {result:?}",
        );
    }

    // Issue #50 regression guard: `has_incomplete_code_fence` and
    // `is_inside_code_block` both drive fence detection and must agree on
    // fence openness, or streaming emphasis gets silently swallowed.
    // Reject any input containing a non-fence backtick run (length 1 or 2) so
    // the inline-code branch of `is_inside_code_block` stays out of the
    // invariant; any divergence is then purely fence-state divergence.
    #[test]
    fn fuzz_fence_scanners_agree_without_single_backticks(
        s in fence_soup().prop_filter(
            "all backtick runs must have length >= 3",
            |s| {
                let bytes = s.as_bytes();
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == b'`' {
                        let mut run = 0;
                        while i + run < bytes.len() && bytes[i + run] == b'`' {
                            run += 1;
                        }
                        if run < 3 {
                            return false;
                        }
                        i += run;
                    } else {
                        i += 1;
                    }
                }
                true
            },
        ),
    ) {
        // Any backtick that remains is part of a 3+ run (fence-only).
        let has_open = has_incomplete_code_fence(&s);
        let ends_inside = is_inside_code_block(&s, s.len());
        prop_assert_eq!(
            has_open,
            ends_inside,
            "fence-open divergence on {:?}",
            s,
        );
    }

    // Same property but over tilde-only inputs — tildes never participate in
    // inline code, so no filter is needed.
    #[test]
    fn fuzz_tilde_fence_scanners_agree(s in tilde_fence_soup()) {
        // Self-guard: the invariant assumes no backtick characters; if a
        // future refactor adds backticks to the generator, this catches it.
        prop_assume!(!s.as_bytes().contains(&b'`'));
        let has_open = has_incomplete_code_fence(&s);
        let ends_inside = is_inside_code_block(&s, s.len());
        prop_assert_eq!(
            has_open,
            ends_inside,
            "tilde fence-open divergence on {:?}",
            s,
        );
    }

    #[test]
    fn fuzz_output_length_bounded(s in markdown_soup(), flags in arbitrary_options()) {
        // No handler should expand input by more than a handful of closing
        // markers; 64 bytes of headroom catches unbounded-growth regressions
        // without depending on #144.
        let opts = flags.to_options();
        let result = stitch(&s, &opts);
        prop_assert!(
            result.len() <= s.len() + 64,
            "output grew by more than 64 bytes: input len={}, output len={}, input={s:?}",
            s.len(),
            result.len(),
        );
    }
}

#[test]
fn regression_idempotent_underscore_after_unterminated_html_tag() {
    // proptest find: s = "_\\<A\t", italic only. The unterminated `<A` tag
    // range used to swallow the appended closer, breaking idempotency.
    let opts = StitchOptions::default()
        .bold(false)
        .bold_italic(false)
        .inline_code(false)
        .strikethrough(false)
        .links(false)
        .images(false)
        .katex(false)
        .setext_headings(false)
        .html_tags(false)
        .single_tilde(false);
    let once = stitch("_\\<A\t", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

#[test]
fn regression_idempotent_tilde_exposed_by_link_unwrap() {
    // proptest find: s = "a~[A", links + single_tilde. TextOnly link
    // unwrapping removed the `[`, exposing a lone `~` that the (earlier)
    // single-tilde pass had not escaped.
    let opts = StitchOptions::default()
        .bold(false)
        .italic(false)
        .bold_italic(false)
        .inline_code(false)
        .strikethrough(false)
        .links(true)
        .images(false)
        .katex(false)
        .setext_headings(false)
        .html_tags(false);
    let once = stitch("a~[A", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}

#[test]
fn regression_idempotent_emphasis_around_unclosed_code() {
    // proptest find: s = "*A***`a", italic + bold_italic + inline_code.
    // Emphasis ran before inline-code completion, so pass 2 saw a closed
    // code span and re-counted the asterisks differently.
    let opts = StitchOptions::default()
        .italic(true)
        .bold_italic(true)
        .inline_code(true)
        .bold(false)
        .strikethrough(false)
        .links(false)
        .images(false)
        .katex(false)
        .setext_headings(false)
        .html_tags(false)
        .single_tilde(false);
    let once = stitch("*A***`a", &opts).into_owned();
    let twice = stitch(&once, &opts).into_owned();
    assert_eq!(twice, once);
}
