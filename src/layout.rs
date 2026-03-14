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
        style: Style,
        grow: u16,
        align: Align,
        wrap: bool,
        margin: Margin,
        constraints: Constraints,
    },
    BeginContainer {
        direction: Direction,
        gap: u32,
        align: Align,
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
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutNode {
    kind: NodeKind,
    content: Option<String>,
    style: Style,
    pub grow: u16,
    align: Align,
    justify: Justify,
    wrap: bool,
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
    cached_wrapped: Option<Vec<String>>,
    segments: Option<Vec<(String, Style)>>,
    cached_wrapped_segments: Option<Vec<Vec<(String, Style)>>>,
    pub(crate) focus_id: Option<usize>,
    link_url: Option<String>,
    group_name: Option<String>,
    overlays: Vec<OverlayLayer>,
}

#[derive(Debug, Clone)]
struct ContainerConfig {
    gap: u32,
    align: Align,
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
        wrap: bool,
        margin: Margin,
        constraints: Constraints,
    ) -> Self {
        let width = UnicodeWidthStr::width(content.as_str()) as u32;
        Self {
            kind: NodeKind::Text,
            content: Some(content),
            style,
            grow,
            align,
            justify: Justify::Start,
            wrap,
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
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
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
            style: Style::new(),
            grow: 0,
            align,
            justify: Justify::Start,
            wrap,
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
            cached_wrapped: None,
            segments: Some(segments),
            cached_wrapped_segments: None,
            focus_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn container(direction: Direction, config: ContainerConfig) -> Self {
        Self {
            kind: NodeKind::Container(direction),
            content: None,
            style: Style::new(),
            grow: config.grow,
            align: config.align,
            justify: config.justify,
            wrap: false,
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
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
            link_url: None,
            group_name: None,
            overlays: Vec::new(),
        }
    }

    fn spacer(grow: u16) -> Self {
        Self {
            kind: NodeKind::Spacer,
            content: None,
            style: Style::new(),
            grow,
            align: Align::Start,
            justify: Justify::Start,
            wrap: false,
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
            cached_wrapped: None,
            segments: None,
            cached_wrapped_segments: None,
            focus_id: None,
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
            NodeKind::Spacer => 0,
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
            NodeKind::Spacer => 0,
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

    fn min_height_for_width(&self, available_width: u32) -> u32 {
        match self.kind {
            NodeKind::Text if self.wrap => {
                let inner_width = available_width.saturating_sub(self.margin.horizontal());
                let lines = if let Some(ref segs) = self.segments {
                    wrap_segments(segs, inner_width).len().max(1) as u32
                } else {
                    let text = self.content.as_deref().unwrap_or("");
                    wrap_lines(text, inner_width).len().max(1) as u32
                };
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

pub(crate) fn build_tree(commands: &[Command]) -> LayoutNode {
    let mut root = LayoutNode::container(Direction::Column, default_container_config());
    let mut overlays: Vec<OverlayLayer> = Vec::new();
    build_children(&mut root, commands, &mut 0, &mut overlays, false);
    root.overlays = overlays;
    root
}

fn default_container_config() -> ContainerConfig {
    ContainerConfig {
        gap: 0,
        align: Align::Start,
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
    commands: &[Command],
    pos: &mut usize,
    overlays: &mut Vec<OverlayLayer>,
    stop_on_end_overlay: bool,
) {
    let mut pending_focus_id: Option<usize> = None;
    while *pos < commands.len() {
        match &commands[*pos] {
            Command::FocusMarker(id) => {
                pending_focus_id = Some(*id);
                *pos += 1;
            }
            Command::Text {
                content,
                style,
                grow,
                align,
                wrap,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    content.clone(),
                    *style,
                    *grow,
                    *align,
                    *wrap,
                    *margin,
                    *constraints,
                );
                node.focus_id = pending_focus_id.take();
                parent.children.push(node);
                *pos += 1;
            }
            Command::RichText {
                segments,
                wrap,
                align,
                margin,
                constraints,
            } => {
                let mut node =
                    LayoutNode::rich_text(segments.clone(), *wrap, *align, *margin, *constraints);
                node.focus_id = pending_focus_id.take();
                parent.children.push(node);
                *pos += 1;
            }
            Command::Link {
                text,
                url,
                style,
                margin,
                constraints,
            } => {
                let mut node = LayoutNode::text(
                    text.clone(),
                    *style,
                    0,
                    Align::Start,
                    false,
                    *margin,
                    *constraints,
                );
                node.link_url = Some(url.clone());
                node.focus_id = pending_focus_id.take();
                parent.children.push(node);
                *pos += 1;
            }
            Command::BeginContainer {
                direction,
                gap,
                align,
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
                    *direction,
                    ContainerConfig {
                        gap: *gap,
                        align: *align,
                        justify: *justify,
                        border: *border,
                        border_sides: *border_sides,
                        border_style: *border_style,
                        bg_color: *bg_color,
                        padding: *padding,
                        margin: *margin,
                        constraints: *constraints,
                        title: title.clone(),
                        grow: *grow,
                    },
                );
                node.focus_id = pending_focus_id.take();
                node.group_name = group_name.clone();
                *pos += 1;
                build_children(&mut node, commands, pos, overlays, false);
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
                        justify: Justify::Start,
                        border: *border,
                        border_sides: *border_sides,
                        border_style: *border_style,
                        bg_color: None,
                        padding: *padding,
                        margin: *margin,
                        constraints: *constraints,
                        title: title.clone(),
                        grow: *grow,
                    },
                );
                node.is_scrollable = true;
                node.scroll_offset = *scroll_offset;
                node.focus_id = pending_focus_id.take();
                *pos += 1;
                build_children(&mut node, commands, pos, overlays, false);
                parent.children.push(node);
            }
            Command::BeginOverlay { modal } => {
                *pos += 1;
                let mut overlay_node =
                    LayoutNode::container(Direction::Column, default_container_config());
                build_children(&mut overlay_node, commands, pos, overlays, true);
                overlays.push(OverlayLayer {
                    node: overlay_node,
                    modal: *modal,
                });
            }
            Command::Spacer { grow } => {
                parent.children.push(LayoutNode::spacer(*grow));
                *pos += 1;
            }
            Command::EndContainer => {
                *pos += 1;
                return;
            }
            Command::EndOverlay => {
                *pos += 1;
                if stop_on_end_overlay {
                    return;
                }
            }
        }
    }
}

pub(crate) fn compute(node: &mut LayoutNode, area: Rect) {
    if let Some(pct) = node.constraints.width_pct {
        let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
        node.constraints.min_width = Some(resolved);
        node.constraints.max_width = Some(resolved);
        node.constraints.width_pct = None;
    }
    if let Some(pct) = node.constraints.height_pct {
        let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
        node.constraints.min_height = Some(resolved);
        node.constraints.max_height = Some(resolved);
        node.constraints.height_pct = None;
    }

    node.pos = (area.x, area.y);
    node.size = (
        area.width.clamp(
            node.constraints.min_width.unwrap_or(0),
            node.constraints.max_width.unwrap_or(u32::MAX),
        ),
        area.height.clamp(
            node.constraints.min_height.unwrap_or(0),
            node.constraints.max_height.unwrap_or(u32::MAX),
        ),
    );

    if matches!(node.kind, NodeKind::Text) && node.wrap {
        if let Some(ref segs) = node.segments {
            let wrapped = wrap_segments(segs, area.width);
            node.size = (area.width, wrapped.len().max(1) as u32);
            node.cached_wrapped_segments = Some(wrapped);
            node.cached_wrapped = None;
        } else {
            let lines = wrap_lines(node.content.as_deref().unwrap_or(""), area.width);
            node.size = (area.width, lines.len().max(1) as u32);
            node.cached_wrapped = Some(lines);
            node.cached_wrapped_segments = None;
        }
    } else {
        node.cached_wrapped = None;
        node.cached_wrapped_segments = None;
    }

    match node.kind {
        NodeKind::Text | NodeKind::Spacer => {}
        NodeKind::Container(Direction::Row) => {
            layout_row(
                node,
                inner_area(
                    node,
                    Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
                ),
            );
            node.content_height = 0;
        }
        NodeKind::Container(Direction::Column) => {
            let viewport_area = inner_area(
                node,
                Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
            );
            if node.is_scrollable {
                let saved_grows: Vec<u16> = node.children.iter().map(|c| c.grow).collect();
                for child in &mut node.children {
                    child.grow = 0;
                }
                let total_gaps = if node.children.is_empty() {
                    0
                } else {
                    (node.children.len() as u32 - 1) * node.gap
                };
                let natural_height: u32 = node
                    .children
                    .iter()
                    .map(|c| c.min_height_for_width(viewport_area.width))
                    .sum::<u32>()
                    + total_gaps;

                if natural_height > viewport_area.height {
                    let virtual_area = Rect::new(
                        viewport_area.x,
                        viewport_area.y,
                        viewport_area.width,
                        natural_height,
                    );
                    layout_column(node, virtual_area);
                } else {
                    for (child, &grow) in node.children.iter_mut().zip(saved_grows.iter()) {
                        child.grow = grow;
                    }
                    layout_column(node, viewport_area);
                }
                node.content_height = scroll_content_height(node, viewport_area.y);
            } else {
                layout_column(node, viewport_area);
                node.content_height = 0;
            }
        }
    }

    for overlay in &mut node.overlays {
        let width = overlay.node.min_width().min(area.width);
        let height = overlay.node.min_height_for_width(width).min(area.height);
        let x = area.x.saturating_add(area.width.saturating_sub(width) / 2);
        let y = area
            .y
            .saturating_add(area.height.saturating_sub(height) / 2);
        compute(&mut overlay.node, Rect::new(x, y, width, height));
    }
}

fn scroll_content_height(node: &LayoutNode, inner_y: u32) -> u32 {
    let Some(max_bottom) = node
        .children
        .iter()
        .map(|child| {
            child
                .pos
                .1
                .saturating_add(child.size.1)
                .saturating_add(child.margin.bottom)
        })
        .max()
    else {
        return 0;
    };

    max_bottom.saturating_sub(inner_y)
}

fn justify_offsets(justify: Justify, remaining: u32, n: u32, gap: u32) -> (u32, u32) {
    if n <= 1 {
        let start = match justify {
            Justify::Center => remaining / 2,
            Justify::End => remaining,
            _ => 0,
        };
        return (start, gap);
    }

    match justify {
        Justify::Start => (0, gap),
        Justify::Center => (remaining.saturating_sub((n - 1) * gap) / 2, gap),
        Justify::End => (remaining.saturating_sub((n - 1) * gap), gap),
        Justify::SpaceBetween => (0, remaining / (n - 1)),
        Justify::SpaceAround => {
            let slot = remaining / n;
            (slot / 2, slot)
        }
        Justify::SpaceEvenly => {
            let slot = remaining / (n + 1);
            (slot, slot)
        }
    }
}

fn inner_area(node: &LayoutNode, area: Rect) -> Rect {
    let x = area.x + node.border_left_inset() + node.padding.left;
    let y = area.y + node.border_top_inset() + node.padding.top;
    let width = area
        .width
        .saturating_sub(node.border_left_inset() + node.border_right_inset())
        .saturating_sub(node.padding.horizontal());
    let height = area
        .height
        .saturating_sub(node.border_top_inset() + node.border_bottom_inset())
        .saturating_sub(node.padding.vertical());

    Rect::new(x, y, width, height)
}

fn layout_row(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        if let Some(pct) = child.constraints.width_pct {
            let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_width = Some(resolved);
            child.constraints.max_width = Some(resolved);
            child.constraints.width_pct = None;
        }
        if let Some(pct) = child.constraints.height_pct {
            let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_height = Some(resolved);
            child.constraints.max_height = Some(resolved);
            child.constraints.height_pct = None;
        }
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.width.saturating_sub(total_gaps);
    let min_widths: Vec<u32> = node
        .children
        .iter()
        .map(|child| child.min_width())
        .collect();

    let mut total_grow: u32 = 0;
    let mut fixed_width: u32 = 0;
    for (child, &min_width) in node.children.iter().zip(min_widths.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_width += min_width;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_width);
    let mut remaining_grow = total_grow;

    let mut child_widths: Vec<u32> = Vec::with_capacity(node.children.len());
    for (i, child) in node.children.iter().enumerate() {
        let w = if child.grow > 0 && total_grow > 0 {
            let share = if remaining_grow == 0 {
                0
            } else {
                flex_space * child.grow as u32 / remaining_grow
            };
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_widths[i].min(available)
        };
        child_widths.push(w);
    }

    let total_children_width: u32 = child_widths.iter().sum();
    let remaining = area.width.saturating_sub(total_children_width);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut x = area.x + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let w = child_widths[i];
        let child_outer_h = match node.align {
            Align::Start => area.height,
            _ => child.min_height_for_width(w).min(area.height),
        };
        let child_x = x.saturating_add(child.margin.left);
        let child_y = area.y.saturating_add(child.margin.top);
        let child_w = w.saturating_sub(child.margin.horizontal());
        let child_h = child_outer_h.saturating_sub(child.margin.vertical());
        compute(child, Rect::new(child_x, child_y, child_w, child_h));
        let child_total_h = child.size.1.saturating_add(child.margin.vertical());
        let y_offset = match node.align {
            Align::Start => 0,
            Align::Center => area.height.saturating_sub(child_total_h) / 2,
            Align::End => area.height.saturating_sub(child_total_h),
        };
        child.pos.1 = child.pos.1.saturating_add(y_offset);
        x += w + inter_gap;
    }
}

fn layout_column(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    for child in &mut node.children {
        if let Some(pct) = child.constraints.width_pct {
            let resolved = (area.width as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_width = Some(resolved);
            child.constraints.max_width = Some(resolved);
            child.constraints.width_pct = None;
        }
        if let Some(pct) = child.constraints.height_pct {
            let resolved = (area.height as u64 * pct.min(100) as u64 / 100) as u32;
            child.constraints.min_height = Some(resolved);
            child.constraints.max_height = Some(resolved);
            child.constraints.height_pct = None;
        }
    }

    let n = node.children.len() as u32;
    let total_gaps = (n - 1) * node.gap;
    let available = area.height.saturating_sub(total_gaps);
    let min_heights: Vec<u32> = node
        .children
        .iter()
        .map(|child| child.min_height_for_width(area.width))
        .collect();

    let mut total_grow: u32 = 0;
    let mut fixed_height: u32 = 0;
    for (child, &min_height) in node.children.iter().zip(min_heights.iter()) {
        if child.grow > 0 {
            total_grow += child.grow as u32;
        } else {
            fixed_height += min_height;
        }
    }

    let mut flex_space = available.saturating_sub(fixed_height);
    let mut remaining_grow = total_grow;

    let mut child_heights: Vec<u32> = Vec::with_capacity(node.children.len());
    for (i, child) in node.children.iter().enumerate() {
        let h = if child.grow > 0 && total_grow > 0 {
            let share = if remaining_grow == 0 {
                0
            } else {
                flex_space * child.grow as u32 / remaining_grow
            };
            flex_space = flex_space.saturating_sub(share);
            remaining_grow = remaining_grow.saturating_sub(child.grow as u32);
            share
        } else {
            min_heights[i].min(available)
        };
        child_heights.push(h);
    }

    let total_children_height: u32 = child_heights.iter().sum();
    let remaining = area.height.saturating_sub(total_children_height);
    let (start_offset, inter_gap) = justify_offsets(node.justify, remaining, n, node.gap);

    let mut y = area.y + start_offset;
    for (i, child) in node.children.iter_mut().enumerate() {
        let h = child_heights[i];
        let child_outer_w = match node.align {
            Align::Start => area.width,
            _ => child.min_width().min(area.width),
        };
        let child_x = area.x.saturating_add(child.margin.left);
        let child_y = y.saturating_add(child.margin.top);
        let child_w = child_outer_w.saturating_sub(child.margin.horizontal());
        let child_h = h.saturating_sub(child.margin.vertical());
        compute(child, Rect::new(child_x, child_y, child_w, child_h));
        let child_total_w = child.size.0.saturating_add(child.margin.horizontal());
        let x_offset = match node.align {
            Align::Start => 0,
            Align::Center => area.width.saturating_sub(child_total_w) / 2,
            Align::End => area.width.saturating_sub(child_total_w),
        };
        child.pos.0 = child.pos.0.saturating_add(x_offset);
        y += h + inter_gap;
    }
}

pub(crate) fn render(node: &LayoutNode, buf: &mut Buffer) {
    render_inner(node, buf, 0, None);
    buf.clip_stack.clear();
    for overlay in &node.overlays {
        if overlay.modal {
            dim_entire_buffer(buf);
        }
        render_inner(&overlay.node, buf, 0, None);
    }
}

fn dim_entire_buffer(buf: &mut Buffer) {
    for y in buf.area.y..buf.area.bottom() {
        for x in buf.area.x..buf.area.right() {
            let cell = buf.get_mut(x, y);
            cell.style.modifiers |= crate::style::Modifiers::DIM;
        }
    }
}

pub(crate) fn render_debug_overlay(
    node: &LayoutNode,
    buf: &mut Buffer,
    frame_time_us: u64,
    fps: f32,
) {
    for child in &node.children {
        render_debug_overlay_inner(child, buf, 0, 0);
    }
    render_debug_status_bar(node, buf, frame_time_us, fps);
}

fn render_debug_status_bar(node: &LayoutNode, buf: &mut Buffer, frame_time_us: u64, fps: f32) {
    if buf.area.height == 0 || buf.area.width == 0 {
        return;
    }

    let widgets: u32 = node.children.iter().map(count_leaf_widgets).sum();
    let width = buf.area.width;
    let height = buf.area.height;
    let y = buf.area.bottom() - 1;
    let style = Style::new().fg(Color::Black).bg(Color::Yellow).bold();

    let status = format!(
        "[SLT Debug] {}x{} | {} widgets | {:.1}ms | {:.0}fps",
        width,
        height,
        widgets,
        frame_time_us as f64 / 1_000.0,
        fps.max(0.0)
    );

    let row_fill = " ".repeat(width as usize);
    buf.set_string(buf.area.x, y, &row_fill, style);
    buf.set_string(buf.area.x, y, &status, style);
}

fn count_leaf_widgets(node: &LayoutNode) -> u32 {
    let mut total = if node.children.is_empty() {
        match node.kind {
            NodeKind::Spacer => 0,
            _ => 1,
        }
    } else {
        node.children.iter().map(count_leaf_widgets).sum()
    };

    for overlay in &node.overlays {
        total = total.saturating_add(count_leaf_widgets(&overlay.node));
    }

    total
}

fn render_debug_overlay_inner(node: &LayoutNode, buf: &mut Buffer, depth: u32, y_offset: u32) {
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };

    if let NodeKind::Container(_) = node.kind {
        let sy = screen_y(node.pos.1, y_offset);
        if sy + node.size.1 as i64 > 0 {
            let color = debug_color_for_depth(depth);
            let style = Style::new().fg(color);
            let clamped_y = sy.max(0) as u32;
            draw_debug_border(node.pos.0, clamped_y, node.size.0, node.size.1, buf, style);
            if sy >= 0 {
                buf.set_string(node.pos.0, clamped_y, &depth.to_string(), style);
            }
        }
    }

    if node.is_scrollable {
        if let Some(area) = visible_area(node, y_offset) {
            let inner = inner_area(node, area);
            buf.push_clip(inner);
            for child in &node.children {
                render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset);
            }
            buf.pop_clip();
        }
    } else {
        for child in &node.children {
            render_debug_overlay_inner(child, buf, depth.saturating_add(1), child_offset);
        }
    }
}

fn debug_color_for_depth(depth: u32) -> Color {
    match depth {
        0 => Color::Cyan,
        1 => Color::Yellow,
        2 => Color::Magenta,
        _ => Color::Red,
    }
}

fn draw_debug_border(x: u32, y: u32, w: u32, h: u32, buf: &mut Buffer, style: Style) {
    if w == 0 || h == 0 {
        return;
    }
    let right = x + w - 1;
    let bottom = y + h - 1;

    if w == 1 && h == 1 {
        buf.set_char(x, y, '┼', style);
        return;
    }
    if h == 1 {
        for xx in x..=right {
            buf.set_char(xx, y, '─', style);
        }
        return;
    }
    if w == 1 {
        for yy in y..=bottom {
            buf.set_char(x, yy, '│', style);
        }
        return;
    }

    buf.set_char(x, y, '┌', style);
    buf.set_char(right, y, '┐', style);
    buf.set_char(x, bottom, '└', style);
    buf.set_char(right, bottom, '┘', style);

    for xx in (x + 1)..right {
        buf.set_char(xx, y, '─', style);
        buf.set_char(xx, bottom, '─', style);
    }
    for yy in (y + 1)..bottom {
        buf.set_char(x, yy, '│', style);
        buf.set_char(right, yy, '│', style);
    }
}

#[allow(dead_code)]
fn draw_debug_padding_markers(node: &LayoutNode, y_offset: u32, buf: &mut Buffer, style: Style) {
    if node.size.0 == 0 || node.size.1 == 0 {
        return;
    }

    if node.padding == Padding::default() {
        return;
    }

    let Some(area) = visible_area(node, y_offset) else {
        return;
    };
    let inner = inner_area(node, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let right = inner.right() - 1;
    let bottom = inner.bottom() - 1;
    buf.set_char(inner.x, inner.y, 'p', style);
    buf.set_char(right, inner.y, 'p', style);
    buf.set_char(inner.x, bottom, 'p', style);
    buf.set_char(right, bottom, 'p', style);
}

#[allow(dead_code)]
fn draw_debug_margin_markers(node: &LayoutNode, y_offset: u32, buf: &mut Buffer, style: Style) {
    if node.margin == Margin::default() {
        return;
    }

    let margin_y_i = node.pos.1 as i64 - node.margin.top as i64 - y_offset as i64;
    let w = node
        .size
        .0
        .saturating_add(node.margin.horizontal())
        .max(node.margin.horizontal());
    let h = node
        .size
        .1
        .saturating_add(node.margin.vertical())
        .max(node.margin.vertical());

    if w == 0 || h == 0 || margin_y_i + h as i64 <= 0 {
        return;
    }

    let x = node.pos.0.saturating_sub(node.margin.left);
    let y = margin_y_i.max(0) as u32;
    let bottom_i = margin_y_i + h as i64 - 1;
    if bottom_i < 0 {
        return;
    }
    let right = x + w - 1;
    let bottom = bottom_i as u32;
    if margin_y_i >= 0 {
        buf.set_char(x, y, 'm', style);
        buf.set_char(right, y, 'm', style);
    }
    buf.set_char(x, bottom, 'm', style);
    buf.set_char(right, bottom, 'm', style);
}

fn screen_y(layout_y: u32, y_offset: u32) -> i64 {
    layout_y as i64 - y_offset as i64
}

fn visible_area(node: &LayoutNode, y_offset: u32) -> Option<Rect> {
    let sy = screen_y(node.pos.1, y_offset);
    let bottom = sy + node.size.1 as i64;
    if bottom <= 0 || node.size.0 == 0 || node.size.1 == 0 {
        return None;
    }
    let clamped_y = sy.max(0) as u32;
    let clamped_h = (bottom as u32).saturating_sub(clamped_y);
    Some(Rect::new(node.pos.0, clamped_y, node.size.0, clamped_h))
}

fn render_inner(node: &LayoutNode, buf: &mut Buffer, y_offset: u32, parent_bg: Option<Color>) {
    if node.size.0 == 0 || node.size.1 == 0 {
        return;
    }

    let sy = screen_y(node.pos.1, y_offset);
    let sx = i64::from(node.pos.0);
    let ex = sx.saturating_add(i64::from(node.size.0));
    let ey = sy.saturating_add(i64::from(node.size.1));
    let viewport_left = i64::from(buf.area.x);
    let viewport_top = i64::from(buf.area.y);
    let viewport_right = viewport_left.saturating_add(i64::from(buf.area.width));
    let viewport_bottom = viewport_top.saturating_add(i64::from(buf.area.height));

    if ex <= viewport_left || ey <= viewport_top || sx >= viewport_right || sy >= viewport_bottom {
        return;
    }

    match node.kind {
        NodeKind::Text => {
            if let Some(ref segs) = node.segments {
                if node.wrap {
                    let fallback;
                    let wrapped = if let Some(cached) = &node.cached_wrapped_segments {
                        cached.as_slice()
                    } else {
                        fallback = wrap_segments(segs, node.size.0);
                        &fallback
                    };
                    for (i, line_segs) in wrapped.iter().enumerate() {
                        let line_y = sy + i as i64;
                        if line_y < 0 {
                            continue;
                        }
                        let mut x = node.pos.0;
                        for (text, style) in line_segs {
                            let mut s = *style;
                            if s.bg.is_none() {
                                s.bg = parent_bg;
                            }
                            buf.set_string(x, line_y as u32, text, s);
                            x += UnicodeWidthStr::width(text.as_str()) as u32;
                        }
                    }
                } else {
                    if sy < 0 {
                        return;
                    }
                    let mut x = node.pos.0;
                    for (text, style) in segs {
                        let mut s = *style;
                        if s.bg.is_none() {
                            s.bg = parent_bg;
                        }
                        buf.set_string(x, sy as u32, text, s);
                        x += UnicodeWidthStr::width(text.as_str()) as u32;
                    }
                }
            } else if let Some(ref text) = node.content {
                let mut style = node.style;
                if style.bg.is_none() {
                    style.bg = parent_bg;
                }
                if node.wrap {
                    let fallback;
                    let lines = if let Some(cached) = &node.cached_wrapped {
                        cached.as_slice()
                    } else {
                        fallback = wrap_lines(text, node.size.0);
                        fallback.as_slice()
                    };
                    for (i, line) in lines.iter().enumerate() {
                        let line_y = sy + i as i64;
                        if line_y < 0 {
                            continue;
                        }
                        let text_width = UnicodeWidthStr::width(line.as_str()) as u32;
                        let x_offset = if text_width < node.size.0 {
                            match node.align {
                                Align::Start => 0,
                                Align::Center => (node.size.0 - text_width) / 2,
                                Align::End => node.size.0 - text_width,
                            }
                        } else {
                            0
                        };
                        buf.set_string(
                            node.pos.0.saturating_add(x_offset),
                            line_y as u32,
                            line,
                            style,
                        );
                    }
                } else {
                    if sy < 0 {
                        return;
                    }
                    let text_width = UnicodeWidthStr::width(text.as_str()) as u32;
                    let x_offset = if text_width < node.size.0 {
                        match node.align {
                            Align::Start => 0,
                            Align::Center => (node.size.0 - text_width) / 2,
                            Align::End => node.size.0 - text_width,
                        }
                    } else {
                        0
                    };
                    let draw_x = node.pos.0.saturating_add(x_offset);
                    if let Some(ref url) = node.link_url {
                        buf.set_string_linked(draw_x, sy as u32, text, style, url);
                    } else {
                        buf.set_string(draw_x, sy as u32, text, style);
                    }
                }
            }
        }
        NodeKind::Spacer => {}
        NodeKind::Container(_) => {
            if let Some(color) = node.bg_color {
                if let Some(area) = visible_area(node, y_offset) {
                    let fill_style = Style::new().bg(color);
                    for y in area.y..area.bottom() {
                        for x in area.x..area.right() {
                            buf.set_string(x, y, " ", fill_style);
                        }
                    }
                }
            }
            let child_bg = node.bg_color.or(parent_bg);
            render_container_border(node, buf, y_offset, child_bg);
            if node.is_scrollable {
                let Some(area) = visible_area(node, y_offset) else {
                    return;
                };
                let inner = inner_area(node, area);
                let child_offset = y_offset.saturating_add(node.scroll_offset);
                let render_y_start = inner.y as i64;
                let render_y_end = inner.bottom() as i64;
                buf.push_clip(inner);
                for child in &node.children {
                    let child_top = child.pos.1 as i64 - child_offset as i64;
                    let child_bottom = child_top + child.size.1 as i64;
                    if child_bottom <= render_y_start || child_top >= render_y_end {
                        continue;
                    }
                    render_inner(child, buf, child_offset, child_bg);
                }
                buf.pop_clip();
                render_scroll_indicators(node, inner, buf, child_bg);
            } else {
                let Some(area) = visible_area(node, y_offset) else {
                    return;
                };
                let clip = inner_area(node, area);
                buf.push_clip(clip);
                for child in &node.children {
                    render_inner(child, buf, y_offset, child_bg);
                }
                buf.pop_clip();
            }
        }
    }
}

fn render_container_border(
    node: &LayoutNode,
    buf: &mut Buffer,
    y_offset: u32,
    inherit_bg: Option<Color>,
) {
    if node.border_inset() == 0 {
        return;
    }
    let Some(border) = node.border else {
        return;
    };
    let sides = node.border_sides;
    let chars = border.chars();
    let x = node.pos.0;
    let w = node.size.0;
    let h = node.size.1;
    if w == 0 || h == 0 {
        return;
    }

    let mut style = node.border_style;
    if style.bg.is_none() {
        style.bg = inherit_bg;
    }

    let top_i = screen_y(node.pos.1, y_offset);
    let bottom_i = top_i + h as i64 - 1;
    if bottom_i < 0 {
        return;
    }
    let right = x + w - 1;

    if sides.top && top_i >= 0 {
        let y = top_i as u32;
        for xx in x..=right {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.bottom {
        let y = bottom_i as u32;
        for xx in x..=right {
            buf.set_char(xx, y, chars.h, style);
        }
    }
    if sides.left {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            buf.set_char(x, yy, chars.v, style);
        }
    }
    if sides.right {
        let vert_start = top_i.max(0) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            buf.set_char(right, yy, chars.v, style);
        }
    }

    if top_i >= 0 {
        let y = top_i as u32;
        let tl = match (sides.top, sides.left) {
            (true, true) => Some(chars.tl),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = tl {
            buf.set_char(x, y, ch, style);
        }

        let tr = match (sides.top, sides.right) {
            (true, true) => Some(chars.tr),
            (true, false) => Some(chars.h),
            (false, true) => Some(chars.v),
            (false, false) => None,
        };
        if let Some(ch) = tr {
            buf.set_char(right, y, ch, style);
        }
    }

    let y = bottom_i as u32;
    let bl = match (sides.bottom, sides.left) {
        (true, true) => Some(chars.bl),
        (true, false) => Some(chars.h),
        (false, true) => Some(chars.v),
        (false, false) => None,
    };
    if let Some(ch) = bl {
        buf.set_char(x, y, ch, style);
    }

    let br = match (sides.bottom, sides.right) {
        (true, true) => Some(chars.br),
        (true, false) => Some(chars.h),
        (false, true) => Some(chars.v),
        (false, false) => None,
    };
    if let Some(ch) = br {
        buf.set_char(right, y, ch, style);
    }

    if sides.top && top_i >= 0 {
        if let Some((title, title_style)) = &node.title {
            let mut ts = *title_style;
            if ts.bg.is_none() {
                ts.bg = inherit_bg;
            }
            let y = top_i as u32;
            let title_x = x.saturating_add(2);
            if title_x <= right {
                let max_width = (right - title_x + 1) as usize;
                let trimmed: String = title.chars().take(max_width).collect();
                buf.set_string(title_x, y, &trimmed, ts);
            }
        }
    }
}

fn render_scroll_indicators(
    node: &LayoutNode,
    inner: Rect,
    buf: &mut Buffer,
    inherit_bg: Option<Color>,
) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let mut style = node.border_style;
    if style.bg.is_none() {
        style.bg = inherit_bg;
    }

    let indicator_x = inner.right() - 1;
    if node.scroll_offset > 0 {
        buf.set_char(indicator_x, inner.y, '▲', style);
    }
    if node.scroll_offset.saturating_add(inner.height) < node.content_height {
        buf.set_char(indicator_x, inner.bottom() - 1, '▼', style);
    }
}

pub(crate) fn collect_scroll_infos(node: &LayoutNode) -> Vec<(u32, u32)> {
    let mut infos = Vec::new();
    collect_scroll_infos_inner(node, &mut infos);
    for overlay in &node.overlays {
        collect_scroll_infos_inner(&overlay.node, &mut infos);
    }
    infos
}

pub(crate) fn collect_scroll_rects(node: &LayoutNode) -> Vec<Rect> {
    let mut rects = Vec::new();
    collect_scroll_rects_inner(node, &mut rects, 0);
    for overlay in &node.overlays {
        collect_scroll_rects_inner(&overlay.node, &mut rects, 0);
    }
    rects
}

fn collect_scroll_rects_inner(node: &LayoutNode, rects: &mut Vec<Rect>, y_offset: u32) {
    if node.is_scrollable {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        rects.push(Rect::new(node.pos.0, adj_y, node.size.0, node.size.1));
    }
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    for child in &node.children {
        collect_scroll_rects_inner(child, rects, child_offset);
    }
}

pub(crate) fn collect_hit_areas(node: &LayoutNode) -> Vec<Rect> {
    let mut areas = Vec::new();
    for child in &node.children {
        collect_hit_areas_inner(child, &mut areas, 0);
    }
    for overlay in &node.overlays {
        collect_hit_areas_inner(&overlay.node, &mut areas, 0);
    }
    areas
}

pub(crate) fn collect_group_rects(node: &LayoutNode) -> Vec<(String, Rect)> {
    let mut rects = Vec::new();
    for child in &node.children {
        collect_group_rects_inner(child, &mut rects, 0);
    }
    for overlay in &node.overlays {
        collect_group_rects_inner(&overlay.node, &mut rects, 0);
    }
    rects
}

fn collect_scroll_infos_inner(node: &LayoutNode, infos: &mut Vec<(u32, u32)>) {
    if node.is_scrollable {
        let viewport_h = node.size.1.saturating_sub(node.frame_vertical());
        infos.push((node.content_height, viewport_h));
    }
    for child in &node.children {
        collect_scroll_infos_inner(child, infos);
    }
}

fn collect_hit_areas_inner(node: &LayoutNode, areas: &mut Vec<Rect>, y_offset: u32) {
    if matches!(node.kind, NodeKind::Container(_)) || node.link_url.is_some() {
        if node.pos.1 + node.size.1 > y_offset {
            areas.push(Rect::new(
                node.pos.0,
                node.pos.1.saturating_sub(y_offset),
                node.size.0,
                node.size.1,
            ));
        } else {
            areas.push(Rect::new(0, 0, 0, 0));
        }
    }
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    for child in &node.children {
        collect_hit_areas_inner(child, areas, child_offset);
    }
}

fn collect_group_rects_inner(node: &LayoutNode, rects: &mut Vec<(String, Rect)>, y_offset: u32) {
    if let Some(name) = &node.group_name {
        if node.pos.1 + node.size.1 > y_offset {
            rects.push((
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
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    for child in &node.children {
        collect_group_rects_inner(child, rects, child_offset);
    }
}

pub(crate) fn collect_content_areas(node: &LayoutNode) -> Vec<(Rect, Rect)> {
    let mut areas = Vec::new();
    for child in &node.children {
        collect_content_areas_inner(child, &mut areas, 0);
    }
    for overlay in &node.overlays {
        collect_content_areas_inner(&overlay.node, &mut areas, 0);
    }
    areas
}

fn collect_content_areas_inner(node: &LayoutNode, areas: &mut Vec<(Rect, Rect)>, y_offset: u32) {
    if matches!(node.kind, NodeKind::Container(_)) {
        let adj_y = node.pos.1.saturating_sub(y_offset);
        let full = Rect::new(node.pos.0, adj_y, node.size.0, node.size.1);
        let inset_x = node.padding.left + node.border_left_inset();
        let inset_y = node.padding.top + node.border_top_inset();
        let inner_w = node.size.0.saturating_sub(node.frame_horizontal());
        let inner_h = node.size.1.saturating_sub(node.frame_vertical());
        let content = Rect::new(node.pos.0 + inset_x, adj_y + inset_y, inner_w, inner_h);
        areas.push((full, content));
    }
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    for child in &node.children {
        collect_content_areas_inner(child, areas, child_offset);
    }
}

pub(crate) fn collect_focus_rects(node: &LayoutNode) -> Vec<(usize, Rect)> {
    let mut rects = Vec::new();
    collect_focus_rects_inner(node, &mut rects, 0);
    for overlay in &node.overlays {
        collect_focus_rects_inner(&overlay.node, &mut rects, 0);
    }
    rects
}

pub(crate) fn collect_focus_groups(node: &LayoutNode) -> Vec<Option<String>> {
    let mut groups = Vec::new();
    for child in &node.children {
        collect_focus_groups_inner(child, &mut groups, None);
    }
    for overlay in &node.overlays {
        collect_focus_groups_inner(&overlay.node, &mut groups, None);
    }
    groups
}

fn collect_focus_rects_inner(node: &LayoutNode, rects: &mut Vec<(usize, Rect)>, y_offset: u32) {
    if let Some(id) = node.focus_id {
        if node.pos.1 + node.size.1 > y_offset {
            rects.push((
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
    let child_offset = if node.is_scrollable {
        y_offset.saturating_add(node.scroll_offset)
    } else {
        y_offset
    };
    for child in &node.children {
        collect_focus_rects_inner(child, rects, child_offset);
    }
}

fn collect_focus_groups_inner(
    node: &LayoutNode,
    groups: &mut Vec<Option<String>>,
    active_group: Option<&str>,
) {
    let current_group = node.group_name.as_deref().or(active_group);
    if let Some(id) = node.focus_id {
        if id >= groups.len() {
            groups.resize(id + 1, None);
        }
        groups[id] = current_group.map(ToString::to_string);
    }
    for child in &node.children {
        collect_focus_groups_inner(child, groups, current_group);
    }
}

#[cfg(test)]
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
            false,
            Margin::default(),
            Constraints::default(),
        ));

        // Separator 1
        outer_container.children.push(LayoutNode::text(
            "separator".to_string(),
            Style::new(),
            0,
            Align::Start,
            false,
            Margin::default(),
            Constraints::default(),
        ));

        // Inner scrollable-like container with grow:1
        let mut inner_container = LayoutNode::container(
            Direction::Column,
            ContainerConfig {
                gap: 0,
                align: Align::Start,
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
            false,
            Margin::default(),
            Constraints::default(),
        ));
        inner_container.children.push(LayoutNode::text(
            "content2".to_string(),
            Style::new(),
            0,
            Align::Start,
            false,
            Margin::default(),
            Constraints::default(),
        ));
        inner_container.children.push(LayoutNode::text(
            "content3".to_string(),
            Style::new(),
            0,
            Align::Start,
            false,
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
            false,
            Margin::default(),
            Constraints::default(),
        ));

        // Footer
        outer_container.children.push(LayoutNode::text(
            "footer".to_string(),
            Style::new(),
            0,
            Align::Start,
            false,
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
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
            Command::FocusMarker(1),
            Command::Text {
                content: "input2".into(),
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
        ];

        let mut tree = build_tree(&commands);
        let area = crate::rect::Rect::new(0, 0, 40, 10);
        compute(&mut tree, area);

        let rects = collect_focus_rects(&tree);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].0, 0);
        assert_eq!(rects[1].0, 1);
        assert!(rects[0].1.width > 0);
        assert!(rects[1].1.width > 0);
        assert_ne!(rects[0].1.y, rects[1].1.y);
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
                style: Style::new(),
                grow: 0,
                align: Align::Start,
                wrap: false,
                margin: Default::default(),
                constraints: Default::default(),
            },
            Command::EndContainer,
        ];

        let mut tree = build_tree(&commands);
        let area = crate::rect::Rect::new(0, 0, 40, 10);
        compute(&mut tree, area);

        let rects = collect_focus_rects(&tree);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, 0);
        assert!(rects[0].1.width >= 8);
        assert!(rects[0].1.height >= 3);
    }
}
