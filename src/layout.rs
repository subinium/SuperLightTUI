use crate::buffer::Buffer;
use crate::rect::Rect;
use crate::style::{Align, Border, Color, Constraints, Margin, Padding, Style};
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Row,
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
        border: Option<Border>,
        border_style: Style,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        grow: u16,
    },
    BeginScrollable {
        grow: u16,
        border: Option<Border>,
        border_style: Style,
        padding: Padding,
        margin: Margin,
        constraints: Constraints,
        title: Option<(String, Style)>,
        scroll_offset: u32,
    },
    EndContainer,
    Spacer {
        grow: u16,
    },
    FocusMarker(usize),
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
    wrap: bool,
    gap: u32,
    border: Option<Border>,
    border_style: Style,
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
    pub(crate) focus_id: Option<usize>,
}

#[derive(Debug, Clone)]
struct ContainerConfig {
    gap: u32,
    align: Align,
    border: Option<Border>,
    border_style: Style,
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
            wrap,
            gap: 0,
            border: None,
            border_style: Style::new(),
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
            focus_id: None,
        }
    }

    fn container(direction: Direction, config: ContainerConfig) -> Self {
        Self {
            kind: NodeKind::Container(direction),
            content: None,
            style: Style::new(),
            grow: config.grow,
            align: config.align,
            wrap: false,
            gap: config.gap,
            border: config.border,
            border_style: config.border_style,
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
            focus_id: None,
        }
    }

    fn spacer(grow: u16) -> Self {
        Self {
            kind: NodeKind::Spacer,
            content: None,
            style: Style::new(),
            grow,
            align: Align::Start,
            wrap: false,
            gap: 0,
            border: None,
            border_style: Style::new(),
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
            focus_id: None,
        }
    }

    fn border_inset(&self) -> u32 {
        if self.border.is_some() {
            1
        } else {
            0
        }
    }

    fn frame_horizontal(&self) -> u32 {
        self.padding.horizontal() + self.border_inset() * 2
    }

    fn frame_vertical(&self) -> u32 {
        self.padding.vertical() + self.border_inset() * 2
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
                let text = self.content.as_deref().unwrap_or("");
                let inner_width = available_width.saturating_sub(self.margin.horizontal());
                let lines = wrap_lines(text, inner_width).len().max(1) as u32;
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

pub(crate) fn build_tree(commands: &[Command]) -> LayoutNode {
    let mut root = LayoutNode::container(
        Direction::Column,
        ContainerConfig {
            gap: 0,
            align: Align::Start,
            border: None,
            border_style: Style::new(),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        },
    );
    build_children(&mut root, commands, &mut 0);
    root
}

fn build_children(parent: &mut LayoutNode, commands: &[Command], pos: &mut usize) {
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
            Command::BeginContainer {
                direction,
                gap,
                align,
                border,
                border_style,
                padding,
                margin,
                constraints,
                title,
                grow,
            } => {
                let mut node = LayoutNode::container(
                    *direction,
                    ContainerConfig {
                        gap: *gap,
                        align: *align,
                        border: *border,
                        border_style: *border_style,
                        padding: *padding,
                        margin: *margin,
                        constraints: *constraints,
                        title: title.clone(),
                        grow: *grow,
                    },
                );
                node.focus_id = pending_focus_id.take();
                *pos += 1;
                build_children(&mut node, commands, pos);
                parent.children.push(node);
            }
            Command::BeginScrollable {
                grow,
                border,
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
                        border: *border,
                        border_style: *border_style,
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
                build_children(&mut node, commands, pos);
                parent.children.push(node);
            }
            Command::Spacer { grow } => {
                parent.children.push(LayoutNode::spacer(*grow));
                *pos += 1;
            }
            Command::EndContainer => {
                *pos += 1;
                return;
            }
        }
    }
}

pub(crate) fn compute(node: &mut LayoutNode, area: Rect) {
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
        let lines = wrap_lines(node.content.as_deref().unwrap_or(""), area.width);
        node.size = (area.width, lines.len().max(1) as u32);
        node.cached_wrapped = Some(lines);
    } else {
        node.cached_wrapped = None;
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

fn inner_area(node: &LayoutNode, area: Rect) -> Rect {
    let inset = node.border_inset();
    let x = area.x + inset + node.padding.left;
    let y = area.y + inset + node.padding.top;
    let width = area
        .width
        .saturating_sub(inset * 2)
        .saturating_sub(node.padding.horizontal());
    let height = area
        .height
        .saturating_sub(inset * 2)
        .saturating_sub(node.padding.vertical());

    Rect::new(x, y, width, height)
}

fn layout_row(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    let total_gaps = (node.children.len() as u32 - 1) * node.gap;
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
    let mut x = area.x;

    for (i, child) in node.children.iter_mut().enumerate() {
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
        x += w + node.gap;
    }
}

fn layout_column(node: &mut LayoutNode, area: Rect) {
    if node.children.is_empty() {
        return;
    }

    let total_gaps = (node.children.len() as u32 - 1) * node.gap;
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
    let mut y = area.y;

    for (i, child) in node.children.iter_mut().enumerate() {
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
        y += h + node.gap;
    }
}

pub(crate) fn render(node: &LayoutNode, buf: &mut Buffer) {
    render_inner(node, buf, 0);
}

pub(crate) fn render_debug_overlay(node: &LayoutNode, buf: &mut Buffer) {
    for child in &node.children {
        render_debug_overlay_inner(child, buf, 0, 0);
    }
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

fn render_inner(node: &LayoutNode, buf: &mut Buffer, y_offset: u32) {
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
            if let Some(ref text) = node.content {
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
                            node.style,
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
                    buf.set_string(
                        node.pos.0.saturating_add(x_offset),
                        sy as u32,
                        text,
                        node.style,
                    );
                }
            }
        }
        NodeKind::Spacer => {}
        NodeKind::Container(_) => {
            render_container_border(node, buf, y_offset);
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
                    render_inner(child, buf, child_offset);
                }
                buf.pop_clip();
                render_scroll_indicators(node, inner, buf);
            } else {
                let Some(area) = visible_area(node, y_offset) else {
                    return;
                };
                let clip = inner_area(node, area);
                buf.push_clip(clip);
                for child in &node.children {
                    render_inner(child, buf, y_offset);
                }
                buf.pop_clip();
            }
        }
    }
}

fn render_container_border(node: &LayoutNode, buf: &mut Buffer, y_offset: u32) {
    let Some(border) = node.border else {
        return;
    };
    let chars = border.chars();
    let x = node.pos.0;
    let w = node.size.0;
    let h = node.size.1;
    if w == 0 || h == 0 {
        return;
    }

    let top_i = screen_y(node.pos.1, y_offset);
    let bottom_i = top_i + h as i64 - 1;
    if bottom_i < 0 {
        return;
    }
    let right = x + w - 1;

    if w == 1 && h == 1 {
        if top_i >= 0 {
            buf.set_char(x, top_i as u32, chars.tl, node.border_style);
        }
    } else if h == 1 {
        if top_i >= 0 {
            let y = top_i as u32;
            for xx in x..=right {
                buf.set_char(xx, y, chars.h, node.border_style);
            }
        }
    } else if w == 1 {
        let vert_start = (top_i.max(0)) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..=vert_end {
            buf.set_char(x, yy, chars.v, node.border_style);
        }
    } else {
        if top_i >= 0 {
            let y = top_i as u32;
            buf.set_char(x, y, chars.tl, node.border_style);
            buf.set_char(right, y, chars.tr, node.border_style);
            for xx in (x + 1)..right {
                buf.set_char(xx, y, chars.h, node.border_style);
            }
        }

        let bot = bottom_i as u32;
        buf.set_char(x, bot, chars.bl, node.border_style);
        buf.set_char(right, bot, chars.br, node.border_style);
        for xx in (x + 1)..right {
            buf.set_char(xx, bot, chars.h, node.border_style);
        }

        let vert_start = ((top_i + 1).max(0)) as u32;
        let vert_end = bottom_i as u32;
        for yy in vert_start..vert_end {
            buf.set_char(x, yy, chars.v, node.border_style);
            buf.set_char(right, yy, chars.v, node.border_style);
        }
    }

    if top_i >= 0 {
        if let Some((title, title_style)) = &node.title {
            let y = top_i as u32;
            let title_x = x.saturating_add(2);
            if title_x <= right {
                let max_width = (right - title_x + 1) as usize;
                let trimmed: String = title.chars().take(max_width).collect();
                buf.set_string(title_x, y, &trimmed, *title_style);
            }
        }
    }
}

fn render_scroll_indicators(node: &LayoutNode, inner: Rect, buf: &mut Buffer) {
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let indicator_x = inner.right() - 1;
    if node.scroll_offset > 0 {
        buf.set_char(indicator_x, inner.y, '▲', node.border_style);
    }
    if node.scroll_offset.saturating_add(inner.height) < node.content_height {
        buf.set_char(indicator_x, inner.bottom() - 1, '▼', node.border_style);
    }
}

pub(crate) fn collect_scroll_infos(node: &LayoutNode) -> Vec<(u32, u32)> {
    let mut infos = Vec::new();
    collect_scroll_infos_inner(node, &mut infos);
    infos
}

pub(crate) fn collect_hit_areas(node: &LayoutNode) -> Vec<Rect> {
    let mut areas = Vec::new();
    for child in &node.children {
        collect_hit_areas_inner(child, &mut areas);
    }
    areas
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

fn collect_hit_areas_inner(node: &LayoutNode, areas: &mut Vec<Rect>) {
    if matches!(node.kind, NodeKind::Container(_)) {
        areas.push(Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1));
    }
    for child in &node.children {
        collect_hit_areas_inner(child, areas);
    }
}

pub(crate) fn collect_content_areas(node: &LayoutNode) -> Vec<(Rect, Rect)> {
    let mut areas = Vec::new();
    for child in &node.children {
        collect_content_areas_inner(child, &mut areas);
    }
    areas
}

fn collect_content_areas_inner(node: &LayoutNode, areas: &mut Vec<(Rect, Rect)>) {
    if matches!(node.kind, NodeKind::Container(_)) {
        let full = Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1);
        let inset_x = node.padding.left + node.border_inset();
        let inset_y = node.padding.top + node.border_inset();
        let inner_w = node.size.0.saturating_sub(node.frame_horizontal());
        let inner_h = node.size.1.saturating_sub(node.frame_vertical());
        let content = Rect::new(node.pos.0 + inset_x, node.pos.1 + inset_y, inner_w, inner_h);
        areas.push((full, content));
    }
    for child in &node.children {
        collect_content_areas_inner(child, areas);
    }
}

pub(crate) fn collect_focus_rects(node: &LayoutNode) -> Vec<(usize, Rect)> {
    let mut rects = Vec::new();
    collect_focus_rects_inner(node, &mut rects);
    rects
}

fn collect_focus_rects_inner(node: &LayoutNode, rects: &mut Vec<(usize, Rect)>) {
    if let Some(id) = node.focus_id {
        rects.push((
            id,
            Rect::new(node.pos.0, node.pos.1, node.size.0, node.size.1),
        ));
    }
    for child in &node.children {
        collect_focus_rects_inner(child, rects);
    }
}

#[cfg(test)]
mod tests {
    use super::wrap_lines;

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
        use crate::style::{Align, Border, Constraints, Margin, Padding, Style};

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
                border: None,
                border_style: Style::new(),
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
                border: Some(Border::Rounded),
                border_style: Style::new(),
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
                border: None,
                border_style: Style::new(),
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
                border: Some(Border::Single),
                border_style: Style::new(),
                padding: Padding::default(),
                margin: Default::default(),
                constraints: Default::default(),
                title: None,
                grow: 0,
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
