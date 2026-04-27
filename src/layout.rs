//! Flexbox layout engine: builds a tree from commands, computes positions,
//! and renders to a [`Buffer`].

use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Color, Constraints, Justify, Margin, Padding, Style,
};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

mod collect;
mod command;
mod flexbox;
mod render;
mod tree;

pub(crate) use collect::{collect_all, FrameData};
pub use command::Direction;
pub(crate) use command::{BeginContainerArgs, BeginScrollableArgs, Command};
pub(crate) use flexbox::compute;
pub(crate) use render::{render, render_debug_overlay};
pub(crate) use tree::{build_tree, wrap_lines, wrap_segments, LayoutNode, NodeKind};

#[cfg(test)]
mod tests;
