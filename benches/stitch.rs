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

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use mdstitch::{StitchOptions, stitch};

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

fn options() -> StitchOptions {
    StitchOptions::default()
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
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            prefix,
            |b, input| b.iter(|| black_box(stitch(black_box(input), &options()))),
        );
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

criterion_group!(benches, incremental, full_doc);
criterion_main!(benches);
