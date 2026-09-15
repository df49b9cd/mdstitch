//! mdstitch streaming-preprocessor bench (AVO `evo-mdstitch` scope, round 0).
//!
//! `stitch()` runs on every streaming token before the pulldown-cmark parser,
//! so its per-call cost is on the hot path of every rendered markdown block.
//! This bench makes that cost measurable so the AVO operator loop can drive it
//! down: a per-token incremental delta over a growing doc (the streaming shape)
//! plus a full-doc pass (the settle / re-render shape). Run with:
//!
//!   cargo bench -p mdstitch
//!
//! Scores ride in `.vocoder/evo/population-evo-mdstitch.json`; the standard
//! tuple (bench geomean, warnings, test_count) is computed by
//! `.scripts/evo-score.sh`.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mdstitch::{StitchOptions, stitch};
use std::hint::black_box;

/// Build a ~N-byte markdown document exercising every builtin handler: bold,
/// italic, strikethrough, fenced code, links, images, katex, html tags, setext
/// headings. Ends mid-paragraph so the tail is "live" (streaming shape).
fn big_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut para = 0usize;
    while doc.len() < target_bytes {
        match para % 6 {
            0 => doc.push_str(&format!(
                "## Section {para}\n\nA paragraph with **bold**, *italic*, \
                 ~~strike~~ and `inline code` streaming token by token. \
                 Paragraph {para} of the corpus.\n\n"
            )),
            1 => doc.push_str(&format!(
                "```rust\n// fence {para}\nfn f{para}() -> u64 {{\n    let mut \
                 acc = 0u64;\n    for i in 0..{para} {{ acc += i as u64; }}\n    \
                 acc\n}}\n```\n\n"
            )),
            2 => doc.push_str(&format!(
                "- [link {para}a](https://earendil.works/{para}) and \
                 ![img {para}](https://earendil.works/{para}.png)\n- item \
                 {para}b\n- item {para}c\n\n"
            )),
            3 => doc.push_str(&format!(
                "Math block {para}: $$E = mc^{{2}}$$ plus inline \
                 $a^{{2}} + b^{{2}} = c^{{2}}$ in prose.\n\n"
            )),
            4 => doc.push_str(&format!(
                "<div class=\"{para}\">html tag {para}</div>\n\nSetext \
                 heading {para}\n==========\n\n"
            )),
            _ => doc.push_str(&format!(
                "Plain prose block {para}: the stable-prefix cache splits at \
                 the last blank line outside fences; this text is the live \
                 tail during streaming.\n\n"
            )),
        }
        para += 1;
    }
    // End mid-paragraph: the live tail a streaming delta appends to.
    doc.push_str("Streaming tail in prog");
    doc
}

/// Build a ~N-byte document of GENUINELY plain prose: only letters, digits,
/// commas, periods, semicolons, hyphens, spaces and newlines. None of the
/// builtin handler trigger characters (`* _ ` ~ $ > < [ ( = !`) appear, so
/// `stitch()` has nothing to complete — this is the shape a marker-absence
/// fast path rewards. Ends mid-paragraph (live tail). Used by the
/// `stitch_plain_*` benches so an early-out shows up in the score (the
/// marker-heavy `big_doc` corpus would hide it).
fn plain_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut para = 0usize;
    while doc.len() < target_bytes {
        doc.push_str(&format!(
            "Plain prose paragraph number {para}. It has no markdown markers \
             at all - no emphasis, no code, no math, no links, no html - just \
             sentences streaming token by token while the model generates \
             them. This is the common case for most generated text. Paragraph \
             {para} of the plain corpus ends here.\n\n"
        ));
        para += 1;
    }
    doc.push_str("Streaming tail in prog");
    doc
}

/// Build a ~N-byte code-only document: fenced blocks + plain prose, the
/// realistic shape of a "code answer" assistant reply. Only the emphasis
/// group's backtick trigger is present — html/link/math/setext triggers are
/// absent, so the per-handler presence gating (x3) shows up here. The
/// marker-heavy corpus would hide it (every group hits); the plain corpus
/// hides it the other way (whole-pipeline early-out).
fn codeonly_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut n = 0usize;
    while doc.len() < target_bytes {
        doc.push_str(&format!(
            "```rust\nfn f{n}() -> u64 {{\n    let mut acc = 0u64;\n    for i in 0..{n} {{ acc += i as u64; }}\n    acc\n}}\n```\n\nThe function f{n} computes the triangular number. It iterates and accumulates; boundary conditions are noted in the paragraph that follows so streaming prose keeps flowing between code fences.\n\n"
        ));
        n += 1;
    }
    doc.push_str("Streaming tail in prog");
    doc
}

fn options() -> StitchOptions {
    StitchOptions::default()
}

// --- Canary corpus (P1.3) --------------------------------------------------
// Worst-case inputs for the paths an optimization is most likely to shortcut.
// They ride as SEPARATE `canary_*` benches so the scorer excludes them from
// the primary `bench_geomean_ms` and emits each as its own veto metric —
// `evo-compare` then rejects any candidate that regresses a canary beyond its
// noise band, even while the primary improves. This formalizes the E7 lesson:
// the marker-absence early-out won plain prose but the marker-heavy corpus
// was the score's blind spot. A canary is the blind spot made visible.

/// Fence-parity worst case: many small fenced code blocks, so the doc is
/// saturated with ``` markers and `CodeBlockRanges::new` (the dominant
/// mdstitch cost, ~42% of stitch() per E7 profiling) must scan for parity
/// across the whole document. A change that skips or mis-scans fences shows
/// up here first.
fn canary_fence_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut n = 0usize;
    while doc.len() < target_bytes {
        doc.push_str(&format!(
            "```rust\n// fence {n}\nlet x{n} = {n};\n```\n\nprose between \
             fences {n}.\n\n"
        ));
        n += 1;
    }
    doc.push_str("Streaming tail in prog");
    doc
}

/// Trigger-dense worst case: prose saturated with every builtin handler's
/// trigger byte (`* _ ~ ` $ < [ (`), all as valid markdown. The memchr /
/// trigger-scan fast path finds a trigger on nearly every line, so an
/// early-out keyed on "few markers" cannot fire here — guarding against a
/// change that trades the many-marker case for the plain-prose win.
fn canary_emphasis_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut n = 0usize;
    while doc.len() < target_bytes {
        doc.push_str(&format!(
            "Row {n}: **bold{n}** and *it{n}* with ~~st{n}~~ plus `code{n}` \
             and $x^{{2}}_{n}$ then [l{n}](https://e.x/{n}) and \
             ![i{n}](https://e.x/{n}.png) and <b>t{n}</b>.\n\n"
        ));
        n += 1;
    }
    doc.push_str("Streaming tail in prog");
    doc
}

/// Setext / line-start worst case: many lines that begin with `-` or `=`
/// (setext underlines and list items), the precise scalar line-start scan
/// path kept out of the SIMD fast path in E7 x2. Guards that path's cost and
/// correctness under density.
fn canary_setext_doc(target_bytes: usize) -> String {
    let mut doc = String::with_capacity(target_bytes + 4096);
    let mut n = 0usize;
    while doc.len() < target_bytes {
        doc.push_str(&format!(
            "Heading {n}\n==========\n\n- item {n}a\n- item {n}b\n\n\
             Sub {n}\n----------\n\n"
        ));
        n += 1;
    }
    doc.push_str("Streaming tail in prog");
    doc
}

/// Canary benches. Named `canary_*` so the scorer splits them out of the
/// primary geomean and reports each as a veto metric (see evo-score.sh).
fn canaries(c: &mut Criterion) {
    let fence = canary_fence_doc(256 * 1024);
    let emph = canary_emphasis_doc(256 * 1024);
    let setext = canary_setext_doc(256 * 1024);
    c.bench_function("canary_fence_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&fence), &options())))
    });
    c.bench_function("canary_emphasis_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&emph), &options())))
    });
    c.bench_function("canary_setext_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&setext), &options())))
    });
}

/// Per-token incremental cost: stitch() each prefix of the doc as it grows,
/// mirroring token-by-token streaming. This is the shape that runs on every
/// input delta; driving its geomean down is the scope's primary score.
fn incremental(c: &mut Criterion) {
    let doc = big_doc(64 * 1024);
    // Sample a spread of prefix lengths so the geomean reflects mixed
    // small-token + large-tail cost, not just the settled full doc.
    let lens: &[usize] = &[256, 1024, 4096, 16_384, 65_536];
    let mut group = c.benchmark_group("stitch_incremental");
    for &n in lens {
        let prefix = &doc[..n.min(doc.len())];
        group.bench_with_input(BenchmarkId::from_parameter(n), prefix, |b, input| {
            b.iter(|| black_box(stitch(black_box(input), &options())))
        });
    }
    group.finish();
}

/// Full-doc pass cost: stitch() the whole document in one call (the settle /
/// re-render shape). Separate group so it can be compared against the
/// incremental path.
fn full_doc(c: &mut Criterion) {
    let doc = big_doc(256 * 1024);
    c.bench_function("stitch_full_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&doc), &options())))
    });
}

/// Full-doc pass over PLAIN prose (no markers). A marker-absence fast path in
/// `stitch()` shows up here: pre-optimization this runs the full pipeline +
/// `CodeBlockRanges::new` for nothing; post-optimization it returns
/// `Cow::Borrowed` immediately. Kept at the same 256 KiB size as
/// `stitch_full_256KiB` for a direct before/after comparison.
fn plain_full(c: &mut Criterion) {
    let doc = plain_doc(256 * 1024);
    c.bench_function("stitch_plain_full_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&doc), &options())))
    });
}

/// Code-only corpus (fences + plain prose): only the emphasis group triggers.
/// x3's per-handler gating skips the html/link/math/setext passes here; the
/// other corpora cannot show that win (all-markers hits every group, plain
/// takes the whole-pipeline early-out).
fn codeonly(c: &mut Criterion) {
    let doc = codeonly_doc(256 * 1024);
    c.bench_function("stitch_codeonly_full_256KiB", |b| {
        b.iter(|| black_box(stitch(black_box(&doc), &options())))
    });
    let prefix = &doc[..16_384.min(doc.len())];
    c.bench_function("stitch_codeonly_incremental_16384", |b| {
        b.iter(|| black_box(stitch(black_box(prefix), &options())))
    });
}

criterion_group!(
    benches,
    incremental,
    full_doc,
    plain_full,
    codeonly,
    canaries
);
criterion_main!(benches);
