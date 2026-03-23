use super::*;

#[derive(Debug, Clone)]
pub(crate) struct OverlayLayer {
    pub(crate) node: LayoutNode,
    pub(crate) modal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Text,
    Container(Direction),
    Spacer,
    RawDraw(usize),
}

#[derive(Debug, Clone)]
pub(crate) struct LayoutNode {
    pub(crate) kind: NodeKind,
    pub(crate) content: Option<String>,
    pub(crate) cursor_offset: Option<usize>,
    pub(crate) style: Style,
    pub(crate) grow: u16,
    pub(crate) align: Align,
    pub(crate) align_self: Option<Align>,
    pub(crate) justify: Justify,
    pub(crate) wrap: bool,
    pub(crate) truncate: bool,
    pub(crate) gap: u32,
    pub(crate) border: Option<Border>,
    pub(crate) border_sides: BorderSides,
    pub(crate) border_style: Style,
    pub(crate) bg_color: Option<Color>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) children: Vec<LayoutNode>,
    pub(crate) pos: (u32, u32),
    pub(crate) size: (u32, u32),
    pub(crate) is_scrollable: bool,
    pub(crate) scroll_offset: u32,
    pub(crate) content_height: u32,
    pub(crate) cached_wrap_width: Option<u32>,
    pub(crate) cached_wrapped: Option<Vec<String>>,
    pub(crate) segments: Option<Vec<(String, Style)>>,
    pub(crate) cached_wrapped_segments: Option<Vec<Vec<(String, Style)>>>,
    pub(crate) focus_id: Option<usize>,
    pub(crate) interaction_id: Option<usize>,
    pub(crate) link_url: Option<String>,
    pub(crate) group_name: Option<String>,
    pub(crate) overlays: Vec<OverlayLayer>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerConfig {
    pub(crate) gap: u32,
    pub(crate) align: Align,
    pub(crate) align_self: Option<Align>,
    pub(crate) justify: Justify,
    pub(crate) border: Option<Border>,
    pub(crate) border_sides: BorderSides,
    pub(crate) border_style: Style,
    pub(crate) bg_color: Option<Color>,
    pub(crate) padding: Padding,
    pub(crate) margin: Margin,
    pub(crate) constraints: Constraints,
    pub(crate) title: Option<(String, Style)>,
    pub(crate) grow: u16,
}

impl LayoutNode {
    pub(crate) fn text(
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

    pub(crate) fn rich_text(
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

    pub(crate) fn container(direction: Direction, config: ContainerConfig) -> Self {
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

    pub(crate) fn spacer(grow: u16) -> Self {
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

    pub(crate) fn border_inset(&self) -> u32 {
        if self.border.is_some() {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_left_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.left {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_right_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.right {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_top_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.top {
            1
        } else {
            0
        }
    }

    pub(crate) fn border_bottom_inset(&self) -> u32 {
        if self.border.is_some() && self.border_sides.bottom {
            1
        } else {
            0
        }
    }

    pub(crate) fn frame_horizontal(&self) -> u32 {
        self.padding.horizontal() + self.border_left_inset() + self.border_right_inset()
    }

    pub(crate) fn frame_vertical(&self) -> u32 {
        self.padding.vertical() + self.border_top_inset() + self.border_bottom_inset()
    }

    pub(crate) fn min_width(&self) -> u32 {
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

    pub(crate) fn min_height(&self) -> u32 {
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

    pub(crate) fn ensure_wrapped_for_width(&mut self, available_width: u32) -> u32 {
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

    pub(crate) fn min_height_for_width(&mut self, available_width: u32) -> u32 {
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

pub(crate) fn wrap_lines(text: &str, max_width: u32) -> Vec<String> {
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

pub(crate) fn wrap_segments(
    segments: &[(String, Style)],
    max_width: u32,
) -> Vec<Vec<(String, Style)>> {
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

pub(crate) fn default_container_config() -> ContainerConfig {
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
                node.interaction_id = pending_interaction_id.take();
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
                node.interaction_id = pending_interaction_id.take();
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
