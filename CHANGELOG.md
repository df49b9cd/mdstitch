# Changelog

All notable changes to `mdstitch` will be documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Idempotency violations under the idempotent fuzz sweep — five failure
  families fixed via handler-side gates that refuse an append a later
  handler would swallow into a region, plus a bounded fixpoint loop in
  `run_pipeline` for the cross-handler shapes no single gate can settle:
  - `*A$$\n` (italic + block katex): italic's `*` landed inside the `$$…$$`
    the katex_block pass closed in the same sweep.
  - `_$,` / `$$*$` / `$$$A$*$a` (italic + inline_katex): italic's closer
    landed inside a `$…$` / `$$…$$` span once inline_katex ran.
  - `$``$$**` (italic + inline_katex): bold's `**` and inline_katex's `$`
    flipped the italic single-`*` counter between passes.
  - `<A <A` (html_tags): stripping the trailing incomplete tag exposed an
    earlier strippable one, so the strip was not self-stable.
  - "` $``,`" shape (inline_code + inline_katex): each handler's
    open-state check ignored the other's.
- Carries the earlier seed fixes for `_0__`, `*>\*`, `_\<A\t`, `a~[A`,
  `*A***a`, `[](`, `__$$*\r*\n`, `_\r$`, `$\`n\\`, and `` `$\n$* ``.

### Added

- `stitch` runs a bounded fixpoint loop over the builtin pipeline until two
  consecutive passes agree (≤ 8 iterations, cycle-detected); custom
  handlers are excluded — their idempotency is per-handler.
- CI `seeds` job replays the persisted proptest corpus at reduced case
  count on every push, so the regression seeds keep exercising even when
  the 50 k sweep stays local-only.

### Changed

- `options::priority` doc clarifies that priorities are still ascending;
  idempotency comes from the handler gates, not the ordering.

### Fixed

- Math-range construction (`CodeBlockRanges::compute_math_ranges_impl` and
  `utils::is_within_math_block`) now treats `$` inside fenced code blocks and
  inline code spans as literal text, matching the katex handlers' counters.
  Previously a code-hidden `$` could mis-pair the surrounding dollars so that
  neither `is_within_math` nor `is_within_complete_math` ever reported the
  real math span — emphasis handlers then re-appended closers on every pass
  (`stitch "`$\n$*"` was not idempotent).
- Proptest regression seeds were not being replayed: the persistence file
  lived at `proptest-regressions/tests.txt` from when the test suite was a
  single `src/tests.rs`; after the `src/tests/` split, proptest's
  `SourceParallel` mode looks for a file mirroring the source path. Moved to
  `proptest-regressions/tests/property_based_tests_fuzz_invariants.txt`. Run
  the full sweep with `cargo test -- --ignored`.

### Changed

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
