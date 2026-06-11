//! Flexbox layout engine: builds a tree from commands, computes positions,
//! and renders to a [`Buffer`].

use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Color, Constraints, HeightSpec, Justify, Margin, Padding, Style,
    WidthSpec,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

mod collect;
mod command;
mod flexbox;
mod render;
mod tree;

pub(crate) use collect::{FrameData, collect_all};
pub use command::Direction;
pub(crate) use command::{BeginContainerArgs, BeginScrollableArgs, Command};
pub(crate) use flexbox::compute;
pub(crate) use render::{InspectorFocus, render, render_debug_overlay, render_inspector};
pub(crate) use tree::{LayoutNode, NodeKind, build_tree, wrap_lines, wrap_segments};

/// Test-only entry point exposing `wrap_segments` for allocation-budget tests.
///
/// Not part of the stable API. Used by `tests/v020_perf_alloc.rs`.
#[doc(hidden)]
pub fn __bench_wrap_segments(
    segments: &[(String, Style)],
    max_width: u32,
) -> Vec<Vec<(String, Style)>> {
    tree::wrap_segments(segments, max_width)
}

/// Test-only entry point exposing the modal-aware dim path.
///
/// Not part of the stable API. Used by `tests/v020_perf_alloc.rs` and the
/// `examples/v020_perf_audit` demo.
#[doc(hidden)]
pub fn __bench_dim_buffer_around(buf: &mut Buffer, modal_rect: Rect) {
    render::__bench_dim_buffer_around(buf, modal_rect);
}

#[cfg(test)]
mod tests;
