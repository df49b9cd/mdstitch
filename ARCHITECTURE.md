# Architecture

How `mdstitch` is built and why. For usage, see the [README](README.md);
for the public API, [docs.rs](https://docs.rs/mdstitch).

## Where it sits

`stitch` is a **string-to-string preprocessor** that runs on each accumulated
chunk of a token-by-token markdown stream, *before* the real parser
(pulldown-cmark in tahoe-gpui). It never parses into a document tree — it
scans raw text and closes unterminated markers so every intermediate frame
is well-formed CommonMark. The contract:

- **Total**: `stitch(x)` is always valid input for the downstream parser.
- **Idempotent**: `stitch(stitch(x)) == stitch(x)` — pinned by proptest.
- **Cheap when nothing is wrong**: returns `Cow::Borrowed` (zero allocation)
  when the text needs no repair.

Because it runs on every streaming delta, per-call cost is on the hot path
of every rendered markdown block; the design below is dominated by keeping
that cost near zero for already-complete text.

## The pipeline

One call to [`stitch`](src/lib.rs) runs four phases:

```
input ──▶ trailing-space strip ──▶ trigger scan ──▶ run_pipeline ──▶ output
                                                        │
                          ┌─────────────────────────────┤
                          │  BUILTIN_ORDER stages, each  │
                          │  gated by TriggerPresence,   │
                          │  custom handlers spliced in  │
                          │  by priority                 │
                          └─────────────────────────────┘
```

### 1. Trailing-space strip

A single trailing space is dropped (it is usually a half-typed token); two
trailing spaces are kept because CommonMark reads them as a hard line break.
This is the only unconditional rewrite.

### 2. Trigger scan — the fast path

[`scan_triggers`](src/lib.rs) asks a SIMD question first: *does the text
contain any byte that an enabled handler could act on?* It uses
[`memchr`](https://docs.rs/memchr) (SIMD) per trigger family:

| Group     | Trigger bytes                    | Gates                             |
| --------- | -------------------------------- | --------------------------------- |
| emphasis  | `*` `_` `` ` `` `~` `\`          | bold/italic/code/strike/tilde     |
| math      | `$`                              | katex, inline katex, comparison `$` |
| html      | `<` `>`                          | html_tags, comparison operators  |
| link      | `[` `]` `!` `(`                  | links / images                    |
| setext    | `-` / `=` at a line start (<4 indent cols) | setext headings         |

Two properties make this safe:

- **Option-aware** — a disabled handler's trigger bytes don't count, so
  text that could only trigger a disabled handler still takes the fast
  path.
- **Conservative** — `false` means "provably absent", never a false negative.
  Trigger sets are inclusive on purpose (e.g. `]` is a link trigger because
  the link handler rewrites the `](stitch:incomplete-link)` sentinel it
  inserts; `\` because handlers consult escape state).

When **all** groups are absent, `stitch` returns immediately — no
`CodeBlockRanges` build, no handler passes, `Cow::Borrowed` all the way
through. Plain prose streams through at SIMD scan speed. This is the
single most important optimization: most streaming deltas are already
closed, and they must cost as close to nothing as possible.

When a group is absent but others aren't, its bits still pay off downstream:
each builtin stage is *gated* on its group, so a code-only reply skips the
html/link/math/setext passes without a single extra byte scan.

### 3. `run_pipeline` — one stage table for both modes

Since 0.1.2 there is a **single execution model**. Stages are variants of the
[`Builtin`](src/lib.rs) enum, executed in the order of [`BUILTIN_ORDER`],
which is strictly ascending by priority (pinned by a unit test). Two modes
share it:

- **Builtin-only fast path** — stages consult their `TriggerPresence` gate;
  a gated-closed stage is skipped without running.
- **Custom-merged path** — when `StitchOptions::handlers` is non-empty,
  custom handlers are spliced into the builtin order at their `priority()`
  (stable sort, so equal priorities keep registration order, and a custom
  at exactly a builtin's priority runs after the builtin). All gates are
  treated as open: a custom handler's triggers are unknown, so presence
  bits from the entry scan can't prove anything anymore.

The 0.1.0 design had two hand-maintained pipelines (a dynamic-dispatch
priority sort and a hardcoded fast path). They drifted once on the
`inline_code` / `single_tilde` ordering before being unified — the lesson
is baked into the design: `BUILTIN_ORDER` is the single source of truth,
and the priority-sort test pins it.

### Idempotency

`stitch(stitch(x)) == stitch(x)` does **not** fall out of the priority
order. Two mechanisms enforce it:

1. **Self-stable appends.** Every appending handler refuses an append whose
   byte a later handler's region would hide — the content-keyed gates in
   `emphasis` (`gate_block_math_swallow` / `gate_inline_math_swallow` /
   `gate_inline_code_swallow`), which consult `CodeBlockRanges`. The same
   rule covers insertion points: `insert_closing_underscore` returns `None`
   when the `_` would land beside an existing `_` or an unescaped `\`,
   because `should_skip_underscore` skips exactly those positions, making
   the new byte invisible to the counter from the moment it is written.
2. **Bounded fixpoint.** `run_pipeline` re-runs the builtin stages until two
   consecutive passes agree (≤ 8 iterations, cycle-detected). If it never
   converges, the shortest candidate that verifies as a fixed point is
   returned, so the caller's guarantee still holds. Custom handlers skip the
   loop — their idempotency is per-handler.

### 4. Stage mechanics

Each stage threads a `Cow<str>` through [`apply_with`](src/lib.rs), which
compares the handler's returned borrow to its input by pointer:

- same pointer → unchanged, keep the original `Cow` (no copy);
- borrowed sub-slice → must own it (rare; e.g. trimmed);
- owned → take ownership.

`Cow` threading is what keeps a no-op handler pass actually free — no
re-allocation per stage, only on the first stage that rewrites.

Two special mechanisms:

- **Protocol-mode early return.** In `LinkMode::Protocol`, once the link
  handler rewrites to `[text](stitch:incomplete-link)`, the pipeline stops
  (`Stage::EarlyReturn`): downstream handlers would mangle the sentinel.
  The sentinel URL is the "pending link" signal the renderer can style.
- **`HtmlRerunAfterLinks`.** `LinkMode::TextOnly` unwraps brackets
  (`[<a](` → `<a`), which can expose an incomplete HTML tag that the first
  `html_tags` pass rejected as implausible because of the trailing `](`.
  A compensating re-run of `html_tags` (priority `LINKS + 1`) lets the
  pipeline reach its fixed point in a single `stitch` call.

## Order constraints that are load-bearing

The stage order is not arbitrary; two adjacencies exist because proptest
regressions demanded them:

1. `inline_code` (25) **before** emphasis (30–42): closing an open backtick
   first lets emphasis handlers see a real, closed code span and skip its
   contents (regression: `` "*A***`a" ``).
2. `single_tilde` (26) **after** `links` (20): TextOnly link unwrapping can
   delete a `[` and expose a lone `~` that must still be escaped
   (regression: `"a~[A"`).

Both are documented in place in [`options::priority`](src/options.rs). When
renumbering priorities, keep execution order strictly ascending — the
`builtin_order_is_priority_sorted` test enforces it.

## `CodeBlockRanges` — one scan, many queries

Handlers need to know *where code and math live* so they don't "complete"
markers inside a code fence. Naively that is one O(n) scan per handler —
~15 full-text passes per streaming delta.

[`CodeBlockRanges`](src/ranges.rs) flips that: build once in O(n) (six range
families: code blocks, complete inline code, math, complete math, link URLs,
HTML tags), then every handler query is O(log n) binary search. The cache is
managed by [`SharedRanges`](src/lib.rs):

- built lazily on first stage that needs it;
- invalidated after any handler that may rewrite mid-text (TextOnly links,
  or any custom handler — they are opaque mutators), detected by a cheap
  before/after pointer compare.

`CodeBlockRanges` is also part of the public API: custom-handler authors
can reuse it for repeated position queries instead of the convenience
predicates.

## Scanning primitives

- [`fence.rs`](src/fence.rs) — the single source of truth for CommonMark §4.5
  fence detection and the inline-code state machine. It was extracted when
  three callers turned out to be re-implementations of the same loop;
  everything else (`ranges`, `utils::is_inside_code_block`, …) is a thin
  adapter over `scan_code_regions`.
- [`bracket.rs`](src/bracket.rs) — balanced `[`/`]` matching that skips
  inline-code spans. In its own module purely to break a `utils ↔ ranges`
  import cycle.
- [`utils.rs`](src/utils.rs) — shared predicates (`is_escaped` with
  correct odd-backslash counting, `is_word_char`, list-marker and
  horizontal-rule detection). The four most useful to handler authors
  (`is_inside_code_block`, `is_within_link_or_image_url`,
  `is_within_math_block`, `is_word_char`) are re-exported at the crate root.

## Module map

| Module                    | Role                                                            |
| ------------------------- | --------------------------------------------------------------- |
| `lib.rs`                  | `stitch` entry, trigger scan, stage table, `run_pipeline`        |
| `options.rs`              | `StitchOptions`, `StitchHandler`, `LinkMode`, `priority::*`     |
| `ranges.rs`               | `CodeBlockRanges` — shared range index                           |
| `fence.rs`                | CommonMark §4.5 fence / inline-code scanner (source of truth)   |
| `bracket.rs`              | balanced bracket matcher                                         |
| `utils.rs`                | shared predicates                                                |
| `emphasis.rs`             | `**`, `***`, `__` and the three italic variants                   |
| `inline_code.rs`          | `` ` `` completion                                               |
| `strikethrough.rs`        | `~~` completion                                                  |
| `single_tilde.rs`         | lone `~` escaping                                                |
| `link_image.rs`           | `[text](url` / `![alt](url` completion                           |
| `katex.rs`                | `$$…$$` block and `$…$` inline math                              |
| `html_tags.rs`            | incomplete trailing tag stripping                                |
| `setext_heading.rs`       | dangling `===` / `---` underlines                                 |
| `comparison_operators.rs` | `>` at list-item start                                           |
| `detect_direction.rs`     | first-strong-character LTR/RTL heuristic *(not in the pipeline)*  |
| `incomplete_code.rs`      | `has_incomplete_code_fence`, `open_fence`, `has_table` *(not in pipeline)* |
| `preprocess.rs`           | custom / literal HTML tag helpers *(not in pipeline)*            |
| `tests/`                  | unit tests + proptest, one file per concern                      |

Each marker family is exactly one module, so a new marker type is: write
`handle` (+ optional `handle_with_ranges`), add a `Builtin` variant, slot it
into `BUILTIN_ORDER`, add a `priority` constant, an option flag, and a gate.

## Performance model

The costs that matter, in order:

1. **Plain prose (the common case)**: one SIMD `memchr` pass, return
   borrowed. ~26 µs/256 KiB worst case for the full-width scan.
2. **Marker-heavy text**: each present trigger byte is found early by
   memchr's early exit, then the gated stages that can act run; absent
   groups cost nothing beyond their scan.
3. **Ranges** are built at most once (rebuilt only when a mid-text rewrite
   invalidates byte offsets).
4. **Cow threading** means N stages that don't rewrite allocate zero times.

Benchmarks (`cargo bench`) measure exactly these shapes:
`stitch_incremental` (per-token delta over a growing doc) and
`stitch_plain_incremental` (plain-prose early-out on the streaming shape).

## Testing strategy

- **Per-handler unit tests** — `src/tests/` has one file per concern, plus
  dedicated regression files named for the bugs they pin (e.g.
  `issue_50_mid_line_fence_runs_must_not_open_code_blocks.rs`).
- **proptest invariants** — arbitrary UTF-8 never panics; *every streaming
  prefix* of arbitrary UTF-8 never panics (each char-boundary cut goes
  through `stitch`); idempotency across every option combination; custom
  handler order matches the priority sort. Regressions are committed under
  `proptest-regressions/tests.txt`.
- **Pipeline-shape test** — `builtin_order_is_priority_sorted` pins the
  stage table to the priority constants.

CI ([ci.yml](.github/workflows/ci.yml)) runs build, test, clippy
(`-D warnings`), fmt check, `cargo doc` (`-D warnings`), and a bench
compile smoke on every push/PR. Releases are tag-driven: push a `v*` tag
and [release.yml](.github/workflows/release.yml) publishes to crates.io
(idempotently — re-tagging an already-published version is a no-op).
