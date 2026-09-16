# Changelog

All notable changes to `mdstitch` will be documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this crate adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
