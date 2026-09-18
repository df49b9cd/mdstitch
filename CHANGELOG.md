# Changelog

All notable changes to `mdstitch` will be documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3]

The idempotency release: `stitch(stitch(x)) == stitch(x)` now holds across the
full option matrix, enforced rather than incidental.

### Fixed

- Idempotency violations under the fuzz sweep — every failure family closed.
  Append-side gates refuse an append a later handler would swallow into a
  region, and a bounded fixpoint loop in `run_pipeline` settles the
  cross-handler shapes no single gate can:
  - `*A$$\n` (italic + block katex): italic's `*` landed inside the `$$…$$`
    the katex_block pass closed in the same sweep.
  - `_$,` / `$$*$` / `$$$A$*$a` (italic + inline_katex): italic's closer
    landed inside a `$…$` / `$$…$$` span once inline_katex ran.
  - `` $``$$** `` (italic + inline_katex): bold's `**` and inline_katex's `$`
    flipped the italic single-`*` counter between passes.
  - `<A <A` (html_tags): stripping the trailing incomplete tag exposed an
    earlier strippable one, so the strip was not self-stable.
  - "backtick dollar comma" (inline_code + inline_katex): each handler's
    open-state check ignored the other's.
  - `` ``_*>__ `` and `$$_\`\n` (italic): the underscore completer
    re-inserted `_` beside an existing `_` run or an unescaped `\` on every
    pass, and `should_skip_underscore` skips exactly those, so each insert
    was invisible from birth and the run grew without bound.
- Carries the earlier seed fixes for `_0__`, `*>\*`, `_\<A\t`, `a~[A`,
  `*A***a`, `[](`, `__$$*\r*\n`, `_\r$`, `$\`n\\`, and `` `$\n$* ``.
- Math-range construction (`CodeBlockRanges::compute_math_ranges_impl` and
  `utils::is_within_math_block`) now treats `$` inside fenced code blocks and
  inline code spans as literal text, matching the katex handlers' counters.
  Previously a code-hidden `$` could mis-pair the surrounding dollars so that
  neither `is_within_math` nor `is_within_complete_math` ever reported the
  real math span — emphasis handlers then re-appended closers on every pass
  (`` stitch "`$\n$*" `` was not idempotent).
- Proptest regression seeds were not being replayed: the persistence file
  lived at `proptest-regressions/tests.txt` from when the test suite was a
  single `src/tests.rs`; after the `src/tests/` split, proptest's
  `SourceParallel` mode looks for a file mirroring the source path. Moved to
  `proptest-regressions/tests/property_based_tests_fuzz_invariants.txt`.

### Added

- The idempotency sweep `fuzz_idempotent_all_option_combinations` is a normal
  test now (was `#[ignore]`d). It passes 300 000 cases against the full
  option matrix, and 50 000 is the routine on-demand run.
- `stitch` runs a bounded fixpoint loop over the builtin pipeline until two
  consecutive passes agree (≤ 8 iterations, cycle-detected; falls back to the
  shortest candidate that verifies as a fixed point). Custom handlers are
  excluded — their idempotency is per-handler.
- CI `seeds` job re-runs the sweep at a larger case budget on every push, so
  the persisted corpus under `proptest-regressions/tests/` acts as a gate.

### Changed

- `options::priority` doc clarifies that priorities remain ascending;
  idempotency comes from the handler gates, not the ordering.
- The fuzz module no longer pins an explicit `cases:` literal. It was silently
  overriding `PROPTEST_CASES`, capping every sweep at 128 cases regardless of
  the requested count — so earlier "50k sweep" runs exercised only 128 random
  cases.
- Internal: `ranges::compute_code_ranges` is now a thin adapter over the new
  `fence::code_interior_ranges` helper, which also backs both math scanners —
  code-interior boundary math exists in one place.

No public API changes.

## [0.1.2]

### Changed

- Internal refactor: the two handler pipelines (priority-sorted dynamic
  path for the custom-handlers case, statically-ordered fast path for the
  builtin-only case) unified into a single `Builtin` stage table executed by
  one `run_pipeline`. The paths had already drifted once on the
  `inline_code` / `single_tilde` ordering; the priorities are now renumbered
  so execution order is strictly ascending and pinned by a unit test ([#1](https://github.com/df49b9cd/mdstitch/pull/1)).
- Ranges hygiene: `CodeBlockRanges` cache + invalidation centralized in
  `SharedRanges`; `handle_trailing_asterisks_for_underscore` no longer
  rebuilds ranges for a strict prefix of the text ([#1](https://github.com/df49b9cd/mdstitch/pull/1)).
- Shared helper extraction: `leading_indent_cols`, `is_ws_byte`,
  `word_internal_at`, and the link-URL same-line probe now live in one place
  each instead of three copies ([#1](https://github.com/df49b9cd/mdstitch/pull/1)).
- New `stitch_plain_incremental` bench group measuring the plain-prose
  whole-pipeline early-out on the streaming-token shape ([#1](https://github.com/df49b9cd/mdstitch/pull/1)).

No public API changes.

## [0.1.1]

### Changed

- Bump `criterion` to 0.8, pin MSRV and Rust toolchain (1.98.1), add CI
  build/test/clippy + benches-compile workflows and the crates.io publish
  workflow.

## [0.1.0]

Initial release on crates.io. Streaming Markdown preprocessor that closes
unterminated syntax (emphasis, code spans, links, fenced code blocks, tables,
lists) token-by-token so a partial stream stays renderable.
