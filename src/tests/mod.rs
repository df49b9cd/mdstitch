//! mdstitch unit tests, grouped by concern (split from one 2k-line file).
//! Shared helpers (`opts`, `r`, fixtures) live here; groups use `super::*`.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use proptest::prelude::*;

use super::{
    LinkMode, StitchHandler, StitchOptions, has_incomplete_code_fence, is_inside_code_block, stitch,
};

fn opts() -> StitchOptions {
    StitchOptions::default()
}

fn r(text: &str) -> Cow<'_, str> {
    stitch(text, &opts())
}

mod basic_input;
mod bold;
mod bold_italic;
mod bug_fix_katex_inside_fenced_code_blocks;
mod bug_fix_text_only_link_mode_forward_scanning;
mod code_blocks;
mod comparison_operators;
mod cow_efficiency;
mod custom_handler_support;
mod edge_cases;
mod horizontal_rules;
mod html_tags;
mod images;
mod inline_code;
mod issue_50_mid_line_fence_runs_must_not_open_code_blocks;
mod italic_double_underscores;
mod italic_single_asterisk;
mod italic_single_underscore;
mod katex_block;
mod katex_inline_opt_in;
mod link_text_only_mode;
mod links;
mod lists;
mod mixed_formatting;
mod options_disabled;
mod property_based_tests_fuzz_invariants;
mod setext_headings;
mod single_tilde;
mod streaming_simulation;
mod strikethrough;
