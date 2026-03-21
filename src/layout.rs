//! Flexbox layout engine: builds a tree from commands, computes positions,
//! and renders to a [`Buffer`].

use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Color, Constraints, Justify, Margin, Padding, Style,
};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

/// Main axis direction for a container's children.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Lay out children horizontally (left to right).
    Row,
    /// Lay out children vertically (top to bottom).
    Column,
}

#[derive(Debug, Clone)]
pub(crate) enum Command {
    Text {
        content: String,
        cursor_offset: Option<usize>,
        style: Style,
        grow: u16,
        align: Align,
        wrap: bool,
        truncate: bool,
        margin: Margin,
        constraints: Constraints,
    },
    BeginContainer {
        direction: Direction,
        gap: u32,
        align: Align,
        align_self: Option<Align>,
        justify: Justify,
        border: Option<Border>,
        border_sides: BorderSides,
        border_style: Style,
        bg_color: Option<Color>,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        grow: u16,
        group_name: Option<String>,
    },
    BeginScrollable {
        grow: u16,
        border: Option<Border>,
        border_sides: BorderSides,
        border_style: Style,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        scroll_offset: u32,
    },
    Link {
        text: String,
        url: String,
        style: Style,
        margin: Margin,
        constraints: Constraints,
    },
    RichText {
        segments: Vec<(String, Style)>,
        wrap: bool,
        align: Align,
        margin: Margin,
        constraints: Constraints,
    },
    EndContainer,
    BeginOverlay {
        modal: bool,
    },
    EndOverlay,
    Spacer {
        grow: u16,
    },
    FocusMarker(usize),
    InteractionMarker(usize),
    RawDraw {
        draw_id: usize,
        constraints: Constraints,
        grow: u16,
        margin: Margin,
    },
}

#[derive(Debug, Clone)]
struct OverlayLayer {
    node: LayoutNode,
    modal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Text,
    Container(Direction),
    Spacer,
    RawDraw(usize),
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutNode {
    kind: NodeKind,
    content: Option<String>,
    cursor_offset: Option<usize>,
    style: Style,
    pub grow: u16,
    align: Align,
    pub(crate) align_self: Option<Align>,
    justify: Justify,
    wrap: bool,
    truncate: bool,
    gap: u32,
    border: Option<Border>,
    border_sides: BorderSides,
    border_style: Style,
    bg_color: Option<Color>,
    padding: Padding,
    margin: Margin,
    constraints: Constraints,
    title: Option<(String, Style)>,
    children: Vec<LayoutNode>,
    pos: (u32, u32),
    size: (u32, u32),
    is_scrollable: bool,
    scroll_offset: u32,
    content_height: u32,
    cached_wrap_width: Option<u32>,
    cached_wrapped: Option<Vec<String>>,
    segments: Option<Vec<(String, Style)>>,
    cached_wrapped_segments: Option<Vec<Vec<(String, Style)>>>,
    pub(crate) focus_id: Option<usize>,
    pub(crate) interaction_id: Option<usize>,
    link_url: Option<String>,
    group_name: Option<String>,
    overlays: Vec<OverlayLayer>,
}

#[derive(Debug, Clone)]
struct ContainerConfig {
    gap: u32,
    align: Align,
    align_self: Option<Align>,
    justify: Justify,
    border: Option<Border>,
    border_sides: BorderSides,
    border_style: Style,
    bg_color: Option<Color>,
    padding: Padding,
    margin: Margin,
    constraints: Constraints,
    title: Option<(String, Style)>,
    grow: u16,
}

impl LayoutNode {
    fn text(
        content: String,
        style: Style,
        grow: u16,
        align: Align,
        text_meta: (Option<usize>, bool, bool),
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let (cursor_offset, wrap, truncate) = text_meta;
        let width = UnicodeWidthStr::width(content.as_str()) as u32;
        Self {
            kind: NodeKind::Text,
            content: Some(content),
            cursor_offset,
            style,
            grow,
            align,
            align_self: None,
            justify: Justify::Start,
            wrap,
            truncate,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin,
            constraints,
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (width, 1),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            cached_wrap_width: None,
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn rich_text(
        segments: Vec<(String, Style)>,
        wrap: bool,
        align: Align,
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let width: u32 = segments
            .iter()
            .map(|(s, _)| UnicodeWidthStr::width(s.as_str()) as u32)
            .sum();
        Self {
            kind: NodeKind::Text,
            content: None,
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align,
            align_self: None,
            justify: Justify::Start,
            wrap,
            truncate: false,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin,
            constraints,
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (width, 1),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            cached_wrap_width: None,
            cached_wrapped: None,
            segments: Some(segments),
            cached_wrapped_segments: None,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn container(direction: Direction, config: ContainerConfig) -> Self {
        Self {
            kind: NodeKind::Container(direction),
            content: None,
            cursor_offset: None,
            style: Style::new(),
            grow: config.grow,
            align: config.align,
            align_self: config.align_self,
            justify: config.justify,
            wrap: false,
            truncate: false,
            gap: config.gap,
            border: config.border,
            border_sides: config.border_sides,
            border_style: config.border_style,
            bg_color: config.bg_color,
            padding: config.padding,
            margin: config.margin,
            constraints: config.constraints,
            title: config.title,
            children: Vec::new(),
            pos: (0, 0),
            size: (0, 0),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            cached_wrap_width: None,
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn spacer(grow: u16) -> Self {
        Self {
            kind: NodeKind::Spacer,
            content: None,
            cursor_offset: None,
            style: Style::new(),
            grow,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            wrap: false,
            truncate: false,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            children: Vec::new(),
            pos: (0, 0),
            size: (0, 0),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            cached_wrap_width: None,
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn border_inset(&self) -> u32 {
        if self.border.is_some() {
            1
        } else {
            0
        }
    }

    fn border_left_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.left {
            1
        } else {
            0
        }
    }

    fn border_right_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.right {
            1
        } else {
            0
        }
    }

    fn border_top_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.top {
            1
        } else {
            0
        }
    }

    fn border_bottom_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.bottom {
            1
        } else {
            0
        }
    }

    fn frame_horizontal(&self) -> u32 {
        self.padding.horizontal() + self.border_left_inset() + self.border_right_inset()
    }

    fn frame_vertical(&self) -> u32 {
        self.padding.vertical() + self.border_top_inset() + self.border_bottom_inset()
    }

    fn min_width(&self) -> u32 {
        let width = match self.kind {
            NodeKind::Text => self.size.0,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => {
                let gaps = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as u32 - 1) * self.gap
                };
                let children_width: u32 = self.children.iter().map(|c| c.min_width()).sum();
                children_width + gaps + self.frame_horizontal()
            }
            NodeKind::Container(Direction::Column) => {
                self.children
                    .iter()
                    .map(|c| c.min_width())
                    .max()
                    .unwrap_or(0)
                    + self.frame_horizontal()
            }
        };

        let width = width.max(self.constraints.min_width.unwrap_or(0));
        let width = match self.constraints.max_width {
            Some(max_w) => width.min(max_w),
            None => width,
        };
        width.saturating_add(self.margin.horizontal())
    }

    fn min_height(&self) -> u32 {
        let height = match self.kind {
            NodeKind::Text => 1,
            NodeKind::Spacer | NodeKind::RawDraw(_) => 0,
            NodeKind::Container(Direction::Row) => {
                self.children
                    .iter()
                    .map(|c| c.min_height())
                    .max()
                    .unwrap_or(0)
                    + self.frame_vertical()
            }
            NodeKind::Container(Direction::Column) => {
                let gaps = if self.children.is_empty() {
                    0
                } else {
                    (self.children.len() as u32 - 1) * self.gap
                };
                let children_height: u32 = self.children.iter().map(|c| c.min_height()).sum();
                children_height + gaps + self.frame_vertical()
            }
        };

        let height = height.max(self.constraints.min_height.unwrap_or(0));
        height.saturating_add(self.margin.vertical())
    }

    fn ensure_wrapped_for_width(&mut self, available_width: u32) -> u32 {
        if self.cached_wrap_width == Some(available_width) {
            if let Some(ref segs) = self.cached_wrapped_segments {
                return segs.len().max(1) as u32;
            }
            if let Some(ref lines) = self.cached_wrapped {
                return lines.len().max(1) as u32;
            }
        }

        if let Some(ref segs) = self.segments {
            let wrapped = wrap_segments(segs, available_width);
            let line_count = wrapped.len().max(1) as u32;
            self.cached_wrap_width = Some(available_width);
            self.cached_wrapped_segments = Some(wrapped);
            self.cached_wrapped = None;
            line_count
        } else {
            let text = self.content.as_deref().unwrap_or("");
            let lines = wrap_lines(text, available_width);
            let line_count = lines.len().max(1) as u32;
            self.cached_wrap_width = Some(available_width);
            self.cached_wrapped = Some(lines);
            self.cached_wrapped_segments = None;
            line_count
        }
    }

    fn min_height_for_width(&mut self, available_width: u32) -> u32 {
        match self.kind {
            NodeKind::Text if self.wrap => {
                let inner_width = available_width.saturating_sub(self.margin.horizontal());
                let lines = self.ensure_wrapped_for_width(inner_width);
                lines.saturating_add(self.margin.vertical())
            }
            _ => self.min_height(),
        }
    }
}

fn wrap_lines(text: &str, max_width: u32) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if max_width == 0 {
        return vec![text.to_string()];
    }

    fn split_long_word(word: &str, max_width: u32) -> Vec<(String, u32)> {
        let mut chunks: Vec<(String, u32)> = Vec::new();
        let mut chunk = String::new();
        let mut chunk_width = 0_u32;

        for ch in word.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if chunk.is_empty() {
                if ch_width > max_width {
                    chunks.push((ch.to_string(), ch_width));
                } else {
                    chunk.push(ch);
                    chunk_width = ch_width;
                }
                continue;
            }

            if chunk_width + ch_width > max_width {
                chunks.push((std::mem::take(&mut chunk), chunk_width));
                if ch_width > max_width {
                    chunks.push((ch.to_string(), ch_width));
                    chunk_width = 0;
                } else {
                    chunk.push(ch);
                    chunk_width = ch_width;
                }
            } else {
                chunk.push(ch);
                chunk_width += ch_width;
            }
        }

        if !chunk.is_empty() {
            chunks.push((chunk, chunk_width));
        }

        chunks
    }

    fn push_word_into_line(
        lines: &mut Vec<String>,
        current_line: &mut String,
        current_width: &mut u32,
        word: &str,
        word_width: u32,
        max_width: u32,
    ) {
        if word.is_empty() {
            return;
        }

        if word_width > max_width {
            let chunks = split_long_word(word, max_width);
            for (chunk, chunk_width) in chunks {
                if current_line.is_empty() {
                    *current_line = chunk;
                    *current_width = chunk_width;
                } else if *current_width + 1 + chunk_width <= max_width {
                    current_line.push(' ');
                    current_line.push_str(&chunk);
                    *current_width += 1 + chunk_width;
                } else {
                    lines.push(std::mem::take(current_line));
                    *current_line = chunk;
                    *current_width = chunk_width;
                }
            }
            return;
        }

        if current_line.is_empty() {
            *current_line = word.to_string();
            *current_width = word_width;
        } else if *current_width + 1 + word_width <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
            *current_width += 1 + word_width;
        } else {
            lines.push(std::mem::take(current_line));
            *current_line = word.to_string();
            *current_width = word_width;
        }
    }

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width: u32 = 0;
    let mut current_word = String::new();
    let mut word_width: u32 = 0;

    for ch in text.chars() {
        if ch == ' ' {
            push_word_into_line(
                &mut lines,
                &mut current_line,
                &mut current_width,
                &current_word,
                word_width,
                max_width,
            );
            current_word.clear();
            word_width = 0;
            continue;
        }

        current_word.push(ch);
        word_width += UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
    }

    push_word_into_line(
        &mut lines,
        &mut current_line,
        &mut current_width,
        &current_word,
        word_width,
        max_width,
    );

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

fn wrap_segments(segments: &[(String, Style)], max_width: u32) -> Vec<Vec<(String, Style)>> {
    if max_width == 0 || segments.is_empty() {
        return vec![vec![]];
    }
    let mut chars: Vec<(char, Style)> = Vec::new();
    for (text, style) in segments {
        for ch in text.chars() {
            chars.push((ch, *style));
        }
    }
    if chars.is_empty() {
        return vec![vec![]];
    }

    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut line_chars: Vec<(char, Style)> = Vec::new();
        let mut line_width: u32 = 0;

        if !lines.is_empty() {
            while i < chars.len() && chars[i].0 == ' ' {
                i += 1;
            }
        }

        while i < chars.len() {
            let (ch, st) = chars[i];
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u32;
            if line_width + ch_width > max_width && line_width > 0 {
                if let Some(bp) = line_chars.iter().rposition(|(c, _)| *c == ' ') {
                    let rewind = line_chars.len() - bp - 1;
                    i -= rewind;
                    line_chars.truncate(bp);
                }
                break;
            }
            line_chars.push((ch, st));
            line_width += ch_width;
            i += 1;
        }

        let mut line_segs: Vec<(String, Style)> = Vec::new();
        let mut cur = String::new();
        let mut cur_style: Option<Style> = None;
        for (ch, st) in &line_chars {
            if cur_style == Some(*st) {
                cur.push(*ch);
            } else {
                if let Some(s) = cur_style {
                    if !cur.is_empty() {
                        line_segs.push((std::mem::take(&mut cur), s));
                    }
                }
                cur_style = Some(*st);
                cur.push(*ch);
            }
        }
        if let Some(s) = cur_style {
            if !cur.is_empty() {
                let trimmed = cur.trim_end().to_string();
                if !trimmed.is_empty() {
                    line_segs.push((trimmed, s));
                } else if !line_segs.is_empty() {
                    if let Some(last) = line_segs.last_mut() {
                        let t = last.0.trim_end().to_string();
                        if t.is_empty() {
                            line_segs.pop();
                        } else {
                            last.0 = t;
                        }
                    }
                }
            }
        }
        lines.push(line_segs);
    }
    if lines.is_empty() {
        vec![vec![]]
    } else {
        lines
    }
}

pub(crate) fn build_tree(commands: Vec<Command>) -> LayoutNode {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut overlays: Vec<OverlayLayer> = Vec::new();
    let mut commands = commands.into_iter();
    build_children(&mut root, &mut commands, &mut overlays, false);
    root.overlays = overlays;
    root
}

fn default_container_config() -> ContainerConfig {
    ContainerConfig {
        gap: 0,
        align: Align::Start,
        align_self: None,
        justify: Justify::Start,
        border: None,
        border_sides: BorderSides::all(),
        border_style: Style::new(),
        bg_color: None,
        padding: Padding::default(),
        margin: Margin::default(),
        constraints: Constraints::default(),
        title: None,
        grow: 0,
    }
}

fn build_children(
    parent: &mut LayoutNode,
    commands: &mut std::vec::IntoIter<Command>,
    overlays: &mut Vec<OverlayLayer>,
    stop_on_end_overlay: bool,
) {
    let mut pending_focus_id: Option<usize> = None;
    let mut pending_interaction_id: Option<usize> = None;
    while let Some(command) = commands.next() {
        match command {
            Command::FocusMarker(id) => pending_focus_id = Some(id),
            Command::InteractionMarker(id) => pending_interaction_id = Some(id),
            Command::Text {
                content,
                cursor_offset,
                style,
                grow,
                align,
                wrap,
                truncate,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    content,
                    style,
                    grow,
                    align,
                    (cursor_offset, wrap, truncate),
                    margin,
                    constraints,
                );
                node.focus_id = pending_focus_id.take();
                parent.children.push(node);
            }
            Command::RichText {
                segments,
                wrap,
                align,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::rich_text(segments, wrap, align, margin, constraints);
                node.focus_id = pending_focus_id.take();
                parent.children.push(node);
            }
            Command::Link {
                text,
                url,
                style,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    text,
                    style,
                    0,
                    Align::Start,
                    (None, false, false),
                    margin,
                    constraints,
                );
                node.link_url = Some(url);
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                parent.children.push(node);
            }
            Command::BeginContainer {
                direction,
                gap,
                align,
                align_self,
                justify,
                border,
                border_sides,
                border_style,
                bg_color,
                padding,
                margin,
                constraints,
                title,
                grow,
                group_name,
            } => {
                let mut node = LayoutNode::container(
                    direction,
                    ContainerConfig {
                        gap,
                        align,
                        align_self,
                        justify,
                        border,
                        border_sides,
                        border_style,
                        bg_color,
                        padding,
                        margin,
                        constraints,
                        title,
                        grow,
                    },
                );
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                node.group_name = group_name;
                build_children(&mut node, commands, overlays, false);
                parent.children.push(node);
            }
            Command::BeginScrollable {
                grow,
                border,
                border_sides,
                border_style,
                padding,
                margin,
                constraints,
                title,
                scroll_offset,
            } => {
                let mut node = LayoutNode::container(
                    Direction::Column,
                    ContainerConfig {
                        gap: 0,
                        align: Align::Start,
                        align_self: None,
                        justify: Justify::Start,
                        border,
                        border_sides,
                        border_style,
                        bg_color: None,
                        padding,
                        margin,
                        constraints,
                        title,
                        grow,
                    },
                );
                node.is_scrollable = true;
                node.scroll_offset = scroll_offset;
                node.focus_id = pending_focus_id.take();
                node.interaction_id = pending_interaction_id.take();
                build_children(&mut node, commands, overlays, false);
                parent.children.push(node);
            }
            Command::BeginOverlay { modal } => {
                let mut overlay_node =
                    LayoutNode::container(Direction::Column, default_container_config());
                overlay_node.interaction_id = pending_interaction_id.take();
                build_children(&mut overlay_node, commands, overlays, true);
                overlays.push(OverlayLayer {
                    node: overlay_node,
                    modal,
                });
            }
            Command::Spacer { grow } => parent.children.push(LayoutNode::spacer(grow)),
            Command::RawDraw {
                draw_id,
                constraints,
                grow,
                margin,
            } => {
                let node = LayoutNode {
                    kind: NodeKind::RawDraw(draw_id),
                    content: None,
                    cursor_offset: None,
                    style: Style::new(),
                    grow,
                    align: Align::Start,
                    align_self: None,
                    justify: Justify::Start,
                    wrap: false,
                    truncate: false,
                    gap: 0,
                    border: None,
                    border_sides: BorderSides::all(),
                    border_style: Style::new(),
                    bg_color: None,
                    padding: Padding::default(),
                    margin,
                    constraints,
                    title: None,
                    children: Vec::new(),
                    pos: (0, 0),
                    size: (
                        constraints.min_width.unwrap_or(0),
                        constraints.min_height.unwrap_or(0),
                    ),
                    is_scrollable: false,
                    scroll_offset: 0,
                    content_height: 0,
                    cached_wrap_width: None,
                    cached_wrapped: None,
                    segments: None,
                    cached_wrapped_segments: None,
                    focus_id: pending_focus_id.take(),
                    interaction_id: None,
                    link_url: None,
                    group_name: None,
                    overlays: Vec::new(),
                };
                parent.children.push(node);
            }
            Command::EndContainer => return,
            Command::EndOverlay => {
                if stop_on_end_overlay {
                    return;
                }
            }
        }
    }
}

mod flexbox;
mod render;

pub(crate) use flexbox::compute;
pub(crate) use render::{render, render_debug_overlay};

#[derive(Default)]
pub(crate) struct FrameData {
    pub scroll_infos: Vec<(u32, u32)>,
    pub scroll_rects: Vec<Rect>,
    pub hit_areas: Vec<Rect>,
    pub group_rects: Vec<(String, Rect)>,
    pub content_areas: Vec<(Rect, Rect)>,
    pub focus_rects: Vec<(usize, Rect)>,
    pub focus_groups: Vec<Option<String>>,
    pub raw_draw_rects: Vec<RawDrawRect>,
}

/// Collect all per-frame data from a laid-out tree in a single DFS pass.
///
/// Replaces the 7 individual `collect_*` functions that each traversed the
/// tree independently, reducing per-frame traversals from 7× to 1×.
pub(crate) fn collect_all(node: &LayoutNode) -> FrameData {
    let mut data = FrameData::default();

    // scroll_infos, scroll_rects, focus_rects process the root node itself.
    // group_rects, content_areas, focus_groups skip the root.
    if node.is_scrollable {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        data.scroll_infos.push((node.content_height, viewport_h));
        data.scroll_rects
            .push(Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1));
    }
    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > 0 {
            data.focus_rects.push((
                id,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            ));
        }
    }
    if let Some(id) = node.interaction_id {
        let rect = if node.pos.1 + node.size.1 > 0 {
            Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1)
        } else {
            Rect::new(0, 0, 0, 0)
        };
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = rect;
    }

    let child_offset = if node.is_scrollable {
        node.scroll_offset
    } else {
        0
    };
    for child in &node.children {
        collect_all_inner(child, &mut data, child_offset, None, None);
    }

    for overlay in &node.overlays {
        collect_all_inner(&overlay.node, &mut data, 0, None, None);
    }

    data
}

fn collect_all_inner(
    node: &LayoutNode,
    data: &mut FrameData,
    y_offset: u32,
    active_group: Option<&str>,
    viewport: Option<Rect>,
) {
    // --- scroll_infos (no y_offset dependency) ---
    if node.is_scrollable {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        data.scroll_infos.push((node.content_height, viewport_h));
    }

    // --- scroll_rects (uses y_offset) ---
    if node.is_scrollable {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        data.scroll_rects
            .push(Rect::new(node.pos.0, adj_y, node.size.0, node.size.1));
    }

    // --- hit_areas (indexed by interaction_id) ---
    if let Some(id) = node.interaction_id {
        let rect = if node.pos.1 + node.size.1 > y_offset {
            Rect::new(
                node.pos.0,
                node.pos.1.saturating_sub(y_offset),
                node.size.0,
                node.size.1,
            )
        } else {
            Rect::new(0, 0, 0, 0)
        };
        if id >= data.hit_areas.len() {
            data.hit_areas.resize(id + 1, Rect::new(0, 0, 0, 0));
        }
        data.hit_areas[id] = rect;
    }

    if let NodeKind::RawDraw(draw_id) = node.kind {
        let node_x = node.pos.0;
        let node_w = node.size.0;
        let node_h = node.size.1;
        let screen_y = node.pos.1 as i64 - y_offset as i64;

        if let Some(vp) = viewport {
            let img_top = screen_y;
            let img_bottom = screen_y + node_h as i64;
            let vp_top = vp.y as i64;
            let vp_bottom = vp.bottom() as i64;

            if img_bottom > vp_top && img_top < vp_bottom {
                let visible_top = img_top.max(vp_top) as u32;
                let visible_bottom = img_bottom.min(vp_bottom) as u32;
                let visible_height = visible_bottom.saturating_sub(visible_top);
                let top_clip_rows = (vp_top - img_top).max(0) as u32;

                data.raw_draw_rects.push(RawDrawRect {
                    draw_id,
                    rect: Rect::new(node_x, visible_top, node_w, visible_height),
                    top_clip_rows,
                    original_height: node_h,
                });
            }
        } else {
            data.raw_draw_rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, screen_y.max(0) as u32, node_w, node_h),
                top_clip_rows: 0,
                original_height: node_h,
            });
        }
    }

    // --- group_rects ---
    if let Some(name) = &node.group_name {
        if node.pos.1 + node.size.1 > y_offset {
            data.group_rects.push((
                name.clone(),
                Rect::new(
                    node.pos.0,
                    node.pos.1.saturating_sub(y_offset),
                    node.size.0,
                    node.size.1,
                ),
            ));
        }
    }

    // --- content_areas ---
    if matches!(node.kind, NodeKind::Container(_)) {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        let full = Rect::new(node.pos.0, adj_y, node.size.0, node.size.1);
        let inset_x = node.padding.left + node.border_left_inset();
        let inset_y = node.padding.top + node.border_top_inset();
        let inner_w = node.size.0.saturating_sub(node.frame_horizontal());
        let inner_h = node.size.1.saturating_sub(node.frame_vertical());
        let content = Rect::new(node.pos.0 + inset_x, adj_y + inset_y, inner_w, inner_h);
        data.content_areas.push((full, content));
    }

    // --- focus_rects ---
    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > y_offset {
            data.focus_rects.push((
                id,
                Rect::new(
                    node.pos.0,
                    node.pos.1.saturating_sub(y_offset),
                    node.size.0,
                    node.size.1,
                ),
            ));
        }
    }

    // --- focus_groups ---
    let current_group = node.group_name.as_deref().or(active_group);
    if let Some(id) = node.focus_id {
        if id >= data.focus_groups.len() {
            data.focus_groups.resize(id + 1, None);
        }
        data.focus_groups[id] = current_group.map(ToString::to_string);
    }

    // --- Recurse into children ---
    let (child_offset, child_viewport) = if node.is_scrollable {
        let screen_y = node.pos.1.saturating_sub(y_offset);
        let area = Rect::new(node.pos.0, screen_y, node.size.0, node.size.1);
        let inner = flexbox::inner_area(node, area);
        (y_offset.saturating_add(node.scroll_offset), Some(inner))
    } else {
        (y_offset, viewport)
    };
    for child in &node.children {
        collect_all_inner(child, data, child_offset, current_group, child_viewport);
    }
}

/// Information about a raw-draw node's visible screen rect.
pub(crate) struct RawDrawRect {
    pub draw_id: usize,
    /// The visible portion of the node on screen (clipped to viewport).
    pub rect: Rect,
    /// How many cell rows are clipped from the top (for pixel crop).
    pub top_clip_rows: u32,
    /// The original unclipped height in cell rows.
    pub original_height: u32,
}

pub(crate) fn collect_raw_draw_rects(node: &LayoutNode) -> Vec<RawDrawRect> {
    let mut rects = Vec::new();
    collect_raw_draw_rects_inner(node, &mut rects, 0, None);
    for overlay in &node.overlays {
        collect_raw_draw_rects_inner(&overlay.node, &mut rects, 0, None);
    }
    rects
}

fn collect_raw_draw_rects_inner(
    node: &LayoutNode,
    rects: &mut Vec<RawDrawRect>,
    y_offset: u32,
    viewport: Option<Rect>,
) {
    if let NodeKind::RawDraw(draw_id) = node.kind {
        let node_x = node.pos.0;
        let node_w = node.size.0;
        let node_h = node.size.1;

        // Use signed math for Y to correctly handle scrolled-above-viewport images
        let screen_y = node.pos.1 as i64 - y_offset as i64;

        if let Some(vp) = viewport {
            let img_top = screen_y;
            let img_bottom = screen_y + node_h as i64;
            let vp_top = vp.y as i64;
            let vp_bottom = vp.bottom() as i64;

            // Fully outside viewport — cull entirely
            if img_bottom <= vp_top || img_top >= vp_bottom {
                return;
            }

            // Compute visible rect (intersection with viewport)
            let visible_top = img_top.max(vp_top) as u32;
            let visible_bottom = (img_bottom.min(vp_bottom)) as u32;
            let visible_height = visible_bottom.saturating_sub(visible_top);
            let top_clip_rows = (vp_top - img_top).max(0) as u32;

            rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, visible_top, node_w, visible_height),
                top_clip_rows,
                original_height: node_h,
            });
        } else {
            // No scrollable parent — render at screen position (clamp to 0)
            let screen_y_clamped = screen_y.max(0) as u32;
            rects.push(RawDrawRect {
                draw_id,
                rect: Rect::new(node_x, screen_y_clamped, node_w, node_h),
                top_clip_rows: 0,
                original_height: node_h,
            });
        }
    }

    let (child_offset, child_viewport) = if node.is_scrollable {
        // Compute this scrollable container's inner viewport in screen coords
        let screen_y = node.pos.1.saturating_sub(y_offset);
        let area = Rect::new(node.pos.0, screen_y, node.size.0, node.size.1);
        let inner = flexbox::inner_area(node, area);
        (y_offset.saturating_add(node.scroll_offset), Some(inner))
    } else {
        (y_offset, viewport)
    };

    for child in &node.children {
        collect_raw_draw_rects_inner(child, rects, child_offset, child_viewport);
    }
}

#[cfg(test)]
#[allow(clippy::print_stderr)]
mod tests {
    use super::*;

    #[test]
    fn wrap_empty() {
        assert_eq!(wrap_lines("", 10), vec![""]);
    }

    #[test]
    fn wrap_fits() {
        assert_eq!(wrap_lines("hello", 10), vec!["hello"]);
    }

    #[test]
    fn wrap_word_boundary() {
        assert_eq!(wrap_lines("hello world", 7), vec!["hello", "world"]);
    }

    #[test]
    fn wrap_multiple_words() {
        assert_eq!(
            wrap_lines("one two three four", 9),
            vec!["one two", "three", "four"]
        );
    }

    #[test]
    fn wrap_long_word() {
        assert_eq!(wrap_lines("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn wrap_zero_width() {
        assert_eq!(wrap_lines("hello", 0), vec!["hello"]);
    }

    #[test]
    fn diagnostic_demo_layout() {
        use super::{compute, ContainerConfig, Direction, LayoutNode};
        use crate::rect::Rect;
        use crate::style::{Align, Border, Constraints, Justify, Margin, Padding, Style};

        // Build the tree structure matching demo.rs:
        // Root (Column, grow:0)
        //   └─ Container (Column, grow:1, border:Rounded, padding:all(1))
        //        ├─ Text "header" (grow:0)
        //        ├─ Text "separator" (grow:0)
        //        ├─ Container (Column, grow:1)  ← simulates scrollable
        //        │    ├─ Text "content1" (grow:0)
        //        │    ├─ Text "content2" (grow:0)
        //        │    └─ Text "content3" (grow:0)
        //        ├─ Text "separator2" (grow:0)
        //        └─ Text "footer" (grow:0)

        let mut root = LayoutNode::container(
            Direction::Column,
            ContainerConfig {
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
            },
        );

        // Outer bordered container with grow:1
        let mut outer_container = LayoutNode::container(
            Direction::Column,
            ContainerConfig {
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Rounded),
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::all(1),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 1,
            },
        );

        // Header text
        outer_container.children.push(LayoutNode::text(
            "header".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));

        // Separator 1
        outer_container.children.push(LayoutNode::text(
            "separator".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));

        // Inner scrollable-like container with grow:1
        let mut inner_container = LayoutNode::container(
            Direction::Column,
            ContainerConfig {
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 1,
            },
        );

        // Content items
        inner_container.children.push(LayoutNode::text(
            "content1".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));
        inner_container.children.push(LayoutNode::text(
            "content2".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));
        inner_container.children.push(LayoutNode::text(
            "content3".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));

        outer_container.children.push(inner_container);

        // Separator 2
        outer_container.children.push(LayoutNode::text(
            "separator2".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));

        // Footer
        outer_container.children.push(LayoutNode::text(
            "footer".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, false, false),
            Margin::default(),
            Constraints::default(),
        ));

        root.children.push(outer_container);

        // Compute layout with 80x50 terminal
        compute(&mut root, Rect::new(0, 0, 80, 50));

        // Debug output
        eprintln!("\n=== DIAGNOSTIC LAYOUT TEST ===");
        eprintln!("Root node:");
        eprintln!("  pos: {:?}, size: {:?}", root.pos, root.size);

        let outer = &root.children[0];
        eprintln!("\nOuter bordered container (grow:1):");
        eprintln!("  pos: {:?}, size: {:?}", outer.pos, outer.size);

        let inner = &outer.children[2];
        eprintln!("\nInner container (grow:1, simulates scrollable):");
        eprintln!("  pos: {:?}, size: {:?}", inner.pos, inner.size);

        eprintln!("\nAll children of outer container:");
        for (i, child) in outer.children.iter().enumerate() {
            eprintln!("  [{}] pos: {:?}, size: {:?}", i, child.pos, child.size);
        }

        // Assertions
        // Root should fill the entire 80x50 area
        assert_eq!(
            root.size,
            (80, 50),
            "Root node should fill entire terminal (80x50)"
        );

        // Outer container should also be 80x50 (full height due to grow:1)
        assert_eq!(
            outer.size,
            (80, 50),
            "Outer bordered container should fill entire terminal (80x50)"
        );

        // Calculate expected inner container height:
        // Available height = 50 (total)
        // Border inset = 1 (top) + 1 (bottom) = 2
        // Padding = 1 (top) + 1 (bottom) = 2
        // Fixed children heights: header(1) + sep(1) + sep2(1) + footer(1) = 4
        // Expected inner height = 50 - 2 - 2 - 4 = 42
        let expected_inner_height = 50 - 2 - 2 - 4;
        assert_eq!(
            inner.size.1, expected_inner_height as u32,
            "Inner container height should be {} (50 - border(2) - padding(2) - fixed(4))",
            expected_inner_height
        );

        // Inner container should start at y = border(1) + padding(1) + header(1) + sep(1) = 4
        let expected_inner_y = 1 + 1 + 1 + 1;
        assert_eq!(
            inner.pos.1, expected_inner_y as u32,
            "Inner container should start at y={} (border+padding+header+sep)",
            expected_inner_y
        );

        eprintln!("\n✓ All assertions passed!");
        eprintln!("  Root size: {:?}", root.size);
        eprintln!("  Outer container size: {:?}", outer.size);
        eprintln!("  Inner container size: {:?}", inner.size);
        eprintln!("  Inner container pos: {:?}", inner.pos);
    }

    #[test]
    fn collect_focus_rects_from_markers() {
        use super::*;
        use crate::style::Style;

        let commands = vec![
            Command::FocusMarker(0),
            Command::Text {
                content: "input1".into(),
                cursor_offset: None,
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                truncate: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
            Command::FocusMarker(1),
            Command::Text {
                content: "input2".into(),
                cursor_offset: None,
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                truncate: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
        ];

        let mut tree = build_tree(commands);
        let area = crate::rect::Rect::new(0, 0, 40, 10);
        compute(&mut tree, area);

        let fd = collect_all(&tree);
        assert_eq!(fd.focus_rects.len(), 2);
        assert_eq!(fd.focus_rects[0].0, 0);
        assert_eq!(fd.focus_rects[1].0, 1);
        assert!(fd.focus_rects[0].1.width > 0);
        assert!(fd.focus_rects[1].1.width > 0);
        assert_ne!(fd.focus_rects[0].1.y, fd.focus_rects[1].1.y);
    }

    #[test]
    fn focus_marker_tags_container() {
        use super::*;
        use crate::style::{Border, Style};

        let commands = vec![
            Command::FocusMarker(0),
            Command::BeginContainer {
                direction: Direction::Column,
                gap: 0,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: Some(Border::Single),
                border_sides: BorderSides::all(),
                border_style: Style::new(),
                bg_color: None,
                padding: Padding::default(),
                margin: Default::default(),
                constraints: Default::default(),
                title: None,
                grow: 0,
                group_name: None,
            },
            Command::Text {
                content: "inside".into(),
                cursor_offset: None,
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                truncate: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
            Command::EndContainer,
        ];

        let mut tree = build_tree(commands);
        let area = crate::rect::Rect::new(0, 0, 40, 10);
        compute(&mut tree, area);

        let fd = collect_all(&tree);
        assert_eq!(fd.focus_rects.len(), 1);
        assert_eq!(fd.focus_rects[0].0, 0);
        assert!(fd.focus_rects[0].1.width >= 8);
        assert!(fd.focus_rects[0].1.height >= 3);
    }

    #[test]
    fn wrapped_text_cache_reused_for_same_width() {
        let mut node = LayoutNode::text(
            "alpha beta gamma".to_string(),
            Style::new(),
            0,
            Align::Start,
            (None, true, false),
            Margin::default(),
            Constraints::default(),
        );

        let height_a = node.min_height_for_width(6);
        let first_ptr = node.cached_wrapped.as_ref().map(Vec::as_ptr).unwrap();
        let height_b = node.min_height_for_width(6);
        let second_ptr = node.cached_wrapped.as_ref().map(Vec::as_ptr).unwrap();

        assert_eq!(height_a, height_b);
        assert_eq!(first_ptr, second_ptr);
        assert_eq!(node.cached_wrap_width, Some(6));
    }

    #[test]
    fn collect_all_matches_raw_draw_collection() {
        let mut root = LayoutNode::container(Direction::Column, default_container_config());
        let mut scroll = LayoutNode::container(Direction::Column, default_container_config());
        scroll.is_scrollable = true;
        scroll.pos = (0, 0);
        scroll.size = (20, 4);
        scroll.scroll_offset = 2;
        scroll.children.push(LayoutNode {
            kind: NodeKind::RawDraw(7),
            content: None,
            cursor_offset: None,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            align_self: None,
            justify: Justify::Start,
            wrap: false,
            truncate: false,
            gap: 0,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new(),
            bg_color: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            children: Vec::new(),
            pos: (1, 3),
            size: (6, 3),
            is_scrollable: false,
            scroll_offset: 0,
            content_height: 0,
            cached_wrap_width: None,
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
            interaction_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        });
        root.children.push(scroll);

        let via_collect_all = collect_all(&root)
            .raw_draw_rects
            .into_iter()
            .map(|r| (r.draw_id, r.rect, r.top_clip_rows, r.original_height))
            .collect::<Vec<_>>();
        let via_legacy = collect_raw_draw_rects(&root)
            .into_iter()
            .map(|r| (r.draw_id, r.rect, r.top_clip_rows, r.original_height))
            .collect::<Vec<_>>();

        assert_eq!(via_collect_all, via_legacy);
    }
}
