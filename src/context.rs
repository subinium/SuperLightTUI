use crate::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseKind};
use crate::layout::{Command, Direction};
use crate::rect::Rect;
use crate::style::{Align, Border, Color, Constraints, Margin, Modifiers, Padding, Style, Theme};
use crate::widgets::{
    ListState, ScrollState, SpinnerState, TableState, TabsState, TextInputState, TextareaState,
    ToastLevel, ToastState,
};
use unicode_width::UnicodeWidthStr;

/// Result of a container mouse interaction.
///
/// Returned by [`Context::col`], [`Context::row`], and [`ContainerBuilder::col`] /
/// [`ContainerBuilder::row`] so you can react to clicks and hover without a separate
/// event loop.
#[derive(Debug, Clone, Copy, Default)]
pub struct Response {
    /// Whether the container was clicked this frame.
    pub clicked: bool,
    /// Whether the mouse is over the container.
    pub hovered: bool,
}

/// Trait for creating custom widgets.
///
/// Implement this trait to build reusable, composable widgets with full access
/// to the [`Context`] API — focus, events, theming, layout, and mouse interaction.
///
/// # Examples
///
/// A simple rating widget:
///
/// ```no_run
/// use slt::{Context, Widget, Color};
///
/// struct Rating {
///     value: u8,
///     max: u8,
/// }
///
/// impl Rating {
///     fn new(value: u8, max: u8) -> Self {
///         Self { value, max }
///     }
/// }
///
/// impl Widget for Rating {
///     type Response = bool;
///
///     fn ui(&mut self, ui: &mut Context) -> bool {
///         let focused = ui.register_focusable();
///         let mut changed = false;
///
///         if focused {
///             if ui.key('+') && self.value < self.max {
///                 self.value += 1;
///                 changed = true;
///             }
///             if ui.key('-') && self.value > 0 {
///                 self.value -= 1;
///                 changed = true;
///             }
///         }
///
///         let stars: String = (0..self.max).map(|i| {
///             if i < self.value { '★' } else { '☆' }
///         }).collect();
///
///         let color = if focused { Color::Yellow } else { Color::White };
///         ui.styled(stars, slt::Style::new().fg(color));
///
///         changed
///     }
/// }
///
/// fn main() -> std::io::Result<()> {
///     let mut rating = Rating::new(3, 5);
///     slt::run(|ui| {
///         if ui.key('q') { ui.quit(); }
///         ui.text("Rate this:");
///         ui.widget(&mut rating);
///     })
/// }
/// ```
pub trait Widget {
    /// The value returned after rendering. Use `()` for widgets with no return,
    /// `bool` for widgets that report changes, or [`Response`] for click/hover.
    type Response;

    /// Render the widget into the given context.
    ///
    /// Use [`Context::register_focusable`] to participate in Tab focus cycling,
    /// [`Context::key`] / [`Context::key_code`] to handle keyboard input,
    /// and [`Context::interaction`] to detect clicks and hovers.
    fn ui(&mut self, ctx: &mut Context) -> Self::Response;
}

/// The main rendering context passed to your closure each frame.
///
/// Provides all methods for building UI: text, containers, widgets, and event
/// handling. You receive a `&mut Context` on every frame and describe what to
/// render by calling its methods. SLT collects those calls, lays them out with
/// flexbox, diffs against the previous frame, and flushes only changed cells.
///
/// # Example
///
/// ```no_run
/// slt::run(|ui: &mut slt::Context| {
///     if ui.key('q') { ui.quit(); }
///     ui.text("Hello, world!").bold();
/// });
/// ```
pub struct Context {
    pub(crate) commands: Vec<Command>,
    pub(crate) events: Vec<Event>,
    pub(crate) consumed: Vec<bool>,
    pub(crate) should_quit: bool,
    pub(crate) area_width: u32,
    pub(crate) area_height: u32,
    pub(crate) tick: u64,
    pub(crate) focus_index: usize,
    pub(crate) focus_count: usize,
    prev_focus_count: usize,
    scroll_count: usize,
    prev_scroll_infos: Vec<(u32, u32)>,
    interaction_count: usize,
    prev_hit_map: Vec<Rect>,
    mouse_pos: Option<(u32, u32)>,
    click_pos: Option<(u32, u32)>,
    last_mouse_pos: Option<(u32, u32)>,
    last_text_idx: Option<usize>,
    debug: bool,
    theme: Theme,
}

/// Fluent builder for configuring containers before calling `.col()` or `.row()`.
///
/// Obtain one via [`Context::container`] or [`Context::bordered`]. Chain the
/// configuration methods you need, then finalize with `.col(|ui| { ... })` or
/// `.row(|ui| { ... })`.
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// use slt::{Border, Color};
/// ui.container()
///     .border(Border::Rounded)
///     .pad(1)
///     .grow(1)
///     .col(|ui| {
///         ui.text("inside a bordered, padded, growing column");
///     });
/// # });
/// ```
pub struct ContainerBuilder<'a> {
    ctx: &'a mut Context,
    gap: u32,
    align: Align,
    border: Option<Border>,
    border_style: Style,
    padding: Padding,
    margin: Margin,
    constraints: Constraints,
    title: Option<(String, Style)>,
    grow: u16,
    scroll_offset: Option<u32>,
}

impl<'a> ContainerBuilder<'a> {
    // ── border ───────────────────────────────────────────────────────

    /// Set the border style.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Set rounded border style. Shorthand for `.border(Border::Rounded)`.
    pub fn rounded(self) -> Self {
        self.border(Border::Rounded)
    }

    /// Set the style applied to the border characters.
    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    // ── padding (Tailwind: p, px, py, pt, pr, pb, pl) ───────────────

    /// Set uniform padding on all sides. Alias for [`pad`](Self::pad).
    pub fn p(self, value: u32) -> Self {
        self.pad(value)
    }

    /// Set uniform padding on all sides.
    pub fn pad(mut self, value: u32) -> Self {
        self.padding = Padding::all(value);
        self
    }

    /// Set horizontal padding (left and right).
    pub fn px(mut self, value: u32) -> Self {
        self.padding.left = value;
        self.padding.right = value;
        self
    }

    /// Set vertical padding (top and bottom).
    pub fn py(mut self, value: u32) -> Self {
        self.padding.top = value;
        self.padding.bottom = value;
        self
    }

    /// Set top padding.
    pub fn pt(mut self, value: u32) -> Self {
        self.padding.top = value;
        self
    }

    /// Set right padding.
    pub fn pr(mut self, value: u32) -> Self {
        self.padding.right = value;
        self
    }

    /// Set bottom padding.
    pub fn pb(mut self, value: u32) -> Self {
        self.padding.bottom = value;
        self
    }

    /// Set left padding.
    pub fn pl(mut self, value: u32) -> Self {
        self.padding.left = value;
        self
    }

    /// Set per-side padding using a [`Padding`] value.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    // ── margin (Tailwind: m, mx, my, mt, mr, mb, ml) ────────────────

    /// Set uniform margin on all sides.
    pub fn m(mut self, value: u32) -> Self {
        self.margin = Margin::all(value);
        self
    }

    /// Set horizontal margin (left and right).
    pub fn mx(mut self, value: u32) -> Self {
        self.margin.left = value;
        self.margin.right = value;
        self
    }

    /// Set vertical margin (top and bottom).
    pub fn my(mut self, value: u32) -> Self {
        self.margin.top = value;
        self.margin.bottom = value;
        self
    }

    /// Set top margin.
    pub fn mt(mut self, value: u32) -> Self {
        self.margin.top = value;
        self
    }

    /// Set right margin.
    pub fn mr(mut self, value: u32) -> Self {
        self.margin.right = value;
        self
    }

    /// Set bottom margin.
    pub fn mb(mut self, value: u32) -> Self {
        self.margin.bottom = value;
        self
    }

    /// Set left margin.
    pub fn ml(mut self, value: u32) -> Self {
        self.margin.left = value;
        self
    }

    /// Set per-side margin using a [`Margin`] value.
    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    // ── sizing (Tailwind: w, h, min-w, max-w, min-h, max-h) ────────

    /// Set a fixed width (sets both min and max width).
    pub fn w(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self.constraints.max_width = Some(value);
        self
    }

    /// Set a fixed height (sets both min and max height).
    pub fn h(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self.constraints.max_height = Some(value);
        self
    }

    /// Set the minimum width constraint. Shorthand for [`min_width`](Self::min_width).
    pub fn min_w(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self
    }

    /// Set the maximum width constraint. Shorthand for [`max_width`](Self::max_width).
    pub fn max_w(mut self, value: u32) -> Self {
        self.constraints.max_width = Some(value);
        self
    }

    /// Set the minimum height constraint. Shorthand for [`min_height`](Self::min_height).
    pub fn min_h(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self
    }

    /// Set the maximum height constraint. Shorthand for [`max_height`](Self::max_height).
    pub fn max_h(mut self, value: u32) -> Self {
        self.constraints.max_height = Some(value);
        self
    }

    /// Set the minimum width constraint in cells.
    pub fn min_width(mut self, value: u32) -> Self {
        self.constraints.min_width = Some(value);
        self
    }

    /// Set the maximum width constraint in cells.
    pub fn max_width(mut self, value: u32) -> Self {
        self.constraints.max_width = Some(value);
        self
    }

    /// Set the minimum height constraint in rows.
    pub fn min_height(mut self, value: u32) -> Self {
        self.constraints.min_height = Some(value);
        self
    }

    /// Set the maximum height constraint in rows.
    pub fn max_height(mut self, value: u32) -> Self {
        self.constraints.max_height = Some(value);
        self
    }

    /// Set all size constraints at once using a [`Constraints`] value.
    pub fn constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    // ── flex ─────────────────────────────────────────────────────────

    /// Set the gap (in cells) between child elements.
    pub fn gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the flex-grow factor. `1` means the container expands to fill available space.
    pub fn grow(mut self, grow: u16) -> Self {
        self.grow = grow;
        self
    }

    // ── alignment ───────────────────────────────────────────────────

    /// Set the cross-axis alignment of child elements.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Center children on the cross axis. Shorthand for `.align(Align::Center)`.
    pub fn center(self) -> Self {
        self.align(Align::Center)
    }

    // ── title ────────────────────────────────────────────────────────

    /// Set a plain-text title rendered in the top border.
    pub fn title(self, title: impl Into<String>) -> Self {
        self.title_styled(title, Style::new())
    }

    /// Set a styled title rendered in the top border.
    pub fn title_styled(mut self, title: impl Into<String>, style: Style) -> Self {
        self.title = Some((title.into(), style));
        self
    }

    // ── internal ─────────────────────────────────────────────────────

    /// Set the vertical scroll offset in rows. Used internally by [`Context::scrollable`].
    pub fn scroll_offset(mut self, offset: u32) -> Self {
        self.scroll_offset = Some(offset);
        self
    }

    /// Finalize the builder as a vertical (column) container.
    ///
    /// The closure receives a `&mut Context` for rendering children.
    /// Returns a [`Response`] with click/hover state for this container.
    pub fn col(self, f: impl FnOnce(&mut Context)) -> Response {
        self.finish(Direction::Column, f)
    }

    /// Finalize the builder as a horizontal (row) container.
    ///
    /// The closure receives a `&mut Context` for rendering children.
    /// Returns a [`Response`] with click/hover state for this container.
    pub fn row(self, f: impl FnOnce(&mut Context)) -> Response {
        self.finish(Direction::Row, f)
    }

    fn finish(self, direction: Direction, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.ctx.interaction_count;
        self.ctx.interaction_count += 1;

        if let Some(scroll_offset) = self.scroll_offset {
            self.ctx.commands.push(Command::BeginScrollable {
                grow: self.grow,
                border: self.border,
                border_style: self.border_style,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                scroll_offset,
            });
        } else {
            self.ctx.commands.push(Command::BeginContainer {
                direction,
                gap: self.gap,
                align: self.align,
                border: self.border,
                border_style: self.border_style,
                padding: self.padding,
                margin: self.margin,
                constraints: self.constraints,
                title: self.title,
                grow: self.grow,
            });
        }
        f(self.ctx);
        self.ctx.commands.push(Command::EndContainer);
        self.ctx.last_text_idx = None;

        self.ctx.response_for(interaction_id)
    }
}

impl Context {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        events: Vec<Event>,
        width: u32,
        height: u32,
        tick: u64,
        focus_index: usize,
        prev_focus_count: usize,
        prev_scroll_infos: Vec<(u32, u32)>,
        prev_hit_map: Vec<Rect>,
        debug: bool,
        theme: Theme,
        last_mouse_pos: Option<(u32, u32)>,
    ) -> Self {
        let consumed = vec![false; events.len()];

        let mut mouse_pos = last_mouse_pos;
        let mut click_pos = None;
        for event in &events {
            if let Event::Mouse(mouse) = event {
                mouse_pos = Some((mouse.x, mouse.y));
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    click_pos = Some((mouse.x, mouse.y));
                }
            }
        }

        Self {
            commands: Vec::new(),
            events,
            consumed,
            should_quit: false,
            area_width: width,
            area_height: height,
            tick,
            focus_index,
            focus_count: 0,
            prev_focus_count,
            scroll_count: 0,
            prev_scroll_infos,
            interaction_count: 0,
            prev_hit_map,
            mouse_pos,
            click_pos,
            last_mouse_pos,
            last_text_idx: None,
            debug,
            theme,
        }
    }

    pub(crate) fn process_focus_keys(&mut self) {
        for (i, event) in self.events.iter().enumerate() {
            if let Event::Key(key) = event {
                if key.code == KeyCode::Tab && !key.modifiers.contains(KeyModifiers::SHIFT) {
                    if self.prev_focus_count > 0 {
                        self.focus_index = (self.focus_index + 1) % self.prev_focus_count;
                    }
                    self.consumed[i] = true;
                } else if (key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT))
                    || key.code == KeyCode::BackTab
                {
                    if self.prev_focus_count > 0 {
                        self.focus_index = if self.focus_index == 0 {
                            self.prev_focus_count - 1
                        } else {
                            self.focus_index - 1
                        };
                    }
                    self.consumed[i] = true;
                }
            }
        }
    }

    /// Render a custom [`Widget`].
    ///
    /// Calls [`Widget::ui`] with this context and returns the widget's response.
    pub fn widget<W: Widget>(&mut self, w: &mut W) -> W::Response {
        w.ui(self)
    }

    /// Allocate a click/hover interaction slot and return the [`Response`].
    ///
    /// Use this in custom widgets to detect mouse clicks and hovers without
    /// wrapping content in a container. Each call reserves one slot in the
    /// hit-test map, so the call order must be stable across frames.
    pub fn interaction(&mut self) -> Response {
        let id = self.interaction_count;
        self.interaction_count += 1;
        self.response_for(id)
    }

    /// Register a widget as focusable and return whether it currently has focus.
    ///
    /// Call this in custom widgets that need keyboard focus. Each call increments
    /// the internal focus counter, so the call order must be stable across frames.
    pub fn register_focusable(&mut self) -> bool {
        let id = self.focus_count;
        self.focus_count += 1;
        if self.prev_focus_count == 0 {
            return true;
        }
        self.focus_index % self.prev_focus_count == id
    }

    // ── text ──────────────────────────────────────────────────────────

    /// Render a text element. Returns `&mut Self` for style chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Color;
    /// ui.text("hello").bold().fg(Color::Cyan);
    /// # });
    /// ```
    pub fn text(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a text element with word-boundary wrapping.
    ///
    /// Long lines are broken at word boundaries to fit the container width.
    /// Style chaining works the same as [`Context::text`].
    pub fn text_wrap(&mut self, s: impl Into<String>) -> &mut Self {
        let content = s.into();
        self.commands.push(Command::Text {
            content,
            style: Style::new(),
            grow: 0,
            align: Align::Start,
            wrap: true,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    // ── style chain (applies to last text) ───────────────────────────

    /// Apply bold to the last rendered text element.
    pub fn bold(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::BOLD);
        self
    }

    /// Apply dim styling to the last rendered text element.
    ///
    /// Also sets the foreground color to the theme's `text_dim` color if no
    /// explicit foreground has been set.
    pub fn dim(&mut self) -> &mut Self {
        let text_dim = self.theme.text_dim;
        self.modify_last_style(|s| {
            s.modifiers |= Modifiers::DIM;
            if s.fg.is_none() {
                s.fg = Some(text_dim);
            }
        });
        self
    }

    /// Apply italic to the last rendered text element.
    pub fn italic(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::ITALIC);
        self
    }

    /// Apply underline to the last rendered text element.
    pub fn underline(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::UNDERLINE);
        self
    }

    /// Apply reverse-video to the last rendered text element.
    pub fn reversed(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::REVERSED);
        self
    }

    /// Apply strikethrough to the last rendered text element.
    pub fn strikethrough(&mut self) -> &mut Self {
        self.modify_last_style(|s| s.modifiers |= Modifiers::STRIKETHROUGH);
        self
    }

    /// Set the foreground color of the last rendered text element.
    pub fn fg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.fg = Some(color));
        self
    }

    /// Set the background color of the last rendered text element.
    pub fn bg(&mut self, color: Color) -> &mut Self {
        self.modify_last_style(|s| s.bg = Some(color));
        self
    }

    /// Render a text element with an explicit [`Style`] applied immediately.
    ///
    /// Equivalent to calling `text(s)` followed by style-chain methods, but
    /// more concise when you already have a `Style` value.
    pub fn styled(&mut self, s: impl Into<String>, style: Style) -> &mut Self {
        self.commands.push(Command::Text {
            content: s.into(),
            style,
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Enable word-boundary wrapping on the last rendered text element.
    pub fn wrap(&mut self) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { wrap, .. } = &mut self.commands[idx] {
                *wrap = true;
            }
        }
        self
    }

    fn modify_last_style(&mut self, f: impl FnOnce(&mut Style)) {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { style, .. } = &mut self.commands[idx] {
                f(style);
            }
        }
    }

    // ── containers ───────────────────────────────────────────────────

    /// Create a vertical (column) container.
    ///
    /// Children are stacked top-to-bottom. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.col(|ui| {
    ///     ui.text("line one");
    ///     ui.text("line two");
    /// });
    /// # });
    /// ```
    pub fn col(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, 0, f)
    }

    /// Create a vertical (column) container with a gap between children.
    ///
    /// `gap` is the number of blank rows inserted between each child.
    pub fn col_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Column, gap, f)
    }

    /// Create a horizontal (row) container.
    ///
    /// Children are placed left-to-right. Returns a [`Response`] with
    /// click/hover state for the container area.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.row(|ui| {
    ///     ui.text("left");
    ///     ui.spacer();
    ///     ui.text("right");
    /// });
    /// # });
    /// ```
    pub fn row(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, 0, f)
    }

    /// Create a horizontal (row) container with a gap between children.
    ///
    /// `gap` is the number of blank columns inserted between each child.
    pub fn row_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, gap, f)
    }

    /// Create a container with a fluent builder.
    ///
    /// Use this for borders, padding, grow, constraints, and titles. Chain
    /// configuration methods on the returned [`ContainerBuilder`], then call
    /// `.col()` or `.row()` to finalize.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// use slt::Border;
    /// ui.container()
    ///     .border(Border::Rounded)
    ///     .pad(1)
    ///     .title("My Panel")
    ///     .col(|ui| {
    ///         ui.text("content");
    ///     });
    /// # });
    /// ```
    pub fn container(&mut self) -> ContainerBuilder<'_> {
        let border = self.theme.border;
        ContainerBuilder {
            ctx: self,
            gap: 0,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            scroll_offset: None,
        }
    }

    /// Create a scrollable container. Handles wheel scroll and drag-to-scroll automatically.
    ///
    /// Pass a [`ScrollState`] to persist scroll position across frames. The state
    /// is updated in-place with the current scroll offset and bounds.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scrollable(&mut scroll).col(|ui| {
    ///     for i in 0..100 {
    ///         ui.text(format!("Line {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scrollable(&mut self, state: &mut ScrollState) -> ContainerBuilder<'_> {
        let index = self.scroll_count;
        self.scroll_count += 1;
        if let Some(&(ch, vh)) = self.prev_scroll_infos.get(index) {
            state.set_bounds(ch, vh);
            let max = ch.saturating_sub(vh) as usize;
            state.offset = state.offset.min(max);
        }

        let next_id = self.interaction_count;
        if let Some(rect) = self.prev_hit_map.get(next_id).copied() {
            self.auto_scroll(&rect, state);
        }

        self.container().scroll_offset(state.offset as u32)
    }

    fn auto_scroll(&mut self, rect: &Rect, state: &mut ScrollState) {
        let last_y = self.last_mouse_pos.map(|(_, y)| y);
        let mut to_consume: Vec<usize> = Vec::new();

        for (i, event) in self.events.iter().enumerate() {
            if self.consumed[i] {
                continue;
            }
            if let Event::Mouse(mouse) = event {
                let in_bounds = mouse.x >= rect.x
                    && mouse.x < rect.right()
                    && mouse.y >= rect.y
                    && mouse.y < rect.bottom();
                if !in_bounds {
                    continue;
                }
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_up(1);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_down(1);
                        to_consume.push(i);
                    }
                    MouseKind::Drag(MouseButton::Left) => {
                        if let Some(prev_y) = last_y {
                            let delta = mouse.y as i32 - prev_y as i32;
                            if delta < 0 {
                                state.scroll_down((-delta) as usize);
                            } else if delta > 0 {
                                state.scroll_up(delta as usize);
                            }
                        }
                        to_consume.push(i);
                    }
                    _ => {}
                }
            }
        }

        for i in to_consume {
            self.consumed[i] = true;
        }
    }

    /// Shortcut for `container().border(border)`.
    ///
    /// Returns a [`ContainerBuilder`] pre-configured with the given border style.
    pub fn bordered(&mut self, border: Border) -> ContainerBuilder<'_> {
        self.container().border(border)
    }

    fn push_container(
        &mut self,
        direction: Direction,
        gap: u32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let border = self.theme.border;

        self.commands.push(Command::BeginContainer {
            direction,
            gap,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        f(self);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self.response_for(interaction_id)
    }

    fn response_for(&self, interaction_id: usize) -> Response {
        if let Some(rect) = self.prev_hit_map.get(interaction_id) {
            let clicked = self
                .click_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            let hovered = self
                .mouse_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            Response { clicked, hovered }
        } else {
            Response::default()
        }
    }

    /// Set the flex-grow factor of the last rendered text element.
    ///
    /// A value of `1` causes the element to expand and fill remaining space
    /// along the main axis.
    pub fn grow(&mut self, value: u16) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text { grow, .. } = &mut self.commands[idx] {
                *grow = value;
            }
        }
        self
    }

    /// Set the text alignment of the last rendered text element.
    pub fn align(&mut self, align: Align) -> &mut Self {
        if let Some(idx) = self.last_text_idx {
            if let Command::Text {
                align: text_align, ..
            } = &mut self.commands[idx]
            {
                *text_align = align;
            }
        }
        self
    }

    /// Render an invisible spacer that expands to fill available space.
    ///
    /// Useful for pushing siblings to opposite ends of a row or column.
    pub fn spacer(&mut self) -> &mut Self {
        self.commands.push(Command::Spacer { grow: 1 });
        self.last_text_idx = None;
        self
    }

    /// Render a single-line text input. Auto-handles cursor, typing, and backspace.
    ///
    /// The widget claims focus via [`Context::register_focusable`]. When focused,
    /// it consumes character, backspace, arrow, Home, and End key events.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::TextInputState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut input = TextInputState::with_placeholder("Search...");
    /// ui.text_input(&mut input);
    /// // input.value holds the current text
    /// # });
    /// ```
    pub fn text_input(&mut self, state: &mut TextInputState) -> &mut Self {
        let focused = self.register_focusable();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(ch) => {
                            let index = byte_index_for_char(&state.value, state.cursor);
                            state.value.insert(index, ch);
                            state.cursor += 1;
                            consumed_indices.push(i);
                        }
                        KeyCode::Backspace => {
                            if state.cursor > 0 {
                                let start = byte_index_for_char(&state.value, state.cursor - 1);
                                let end = byte_index_for_char(&state.value, state.cursor);
                                state.value.replace_range(start..end, "");
                                state.cursor -= 1;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            state.cursor = state.cursor.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            state.cursor = (state.cursor + 1).min(state.value.chars().count());
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.cursor = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.cursor = state.value.chars().count();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if state.value.is_empty() {
            self.styled(
                state.placeholder.clone(),
                Style::new().dim().fg(self.theme.text_dim),
            )
        } else {
            let mut rendered = String::new();
            for (idx, ch) in state.value.chars().enumerate() {
                if focused && idx == state.cursor {
                    rendered.push('▎');
                }
                rendered.push(ch);
            }
            if focused && state.cursor >= state.value.chars().count() {
                rendered.push('▎');
            }
            self.styled(rendered, Style::new().fg(self.theme.text))
        }
    }

    /// Render an animated spinner.
    ///
    /// The spinner advances one frame per tick. Use [`SpinnerState::dots`] or
    /// [`SpinnerState::line`] to create the state, then chain style methods to
    /// color it.
    pub fn spinner(&mut self, state: &SpinnerState) -> &mut Self {
        self.styled(
            state.frame(self.tick).to_string(),
            Style::new().fg(self.theme.primary),
        )
    }

    /// Render toast notifications. Calls `state.cleanup(tick)` automatically.
    ///
    /// Expired messages are removed before rendering. If there are no active
    /// messages, nothing is rendered and `self` is returned unchanged.
    pub fn toast(&mut self, state: &mut ToastState) -> &mut Self {
        state.cleanup(self.tick);
        if state.messages.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for message in state.messages.iter().rev() {
            let color = match message.level {
                ToastLevel::Info => self.theme.primary,
                ToastLevel::Success => self.theme.success,
                ToastLevel::Warning => self.theme.warning,
                ToastLevel::Error => self.theme.error,
            };
            self.styled(format!("  ● {}", message.text), Style::new().fg(color));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a multi-line text area with the given number of visible rows.
    ///
    /// When focused, handles character input, Enter (new line), Backspace,
    /// arrow keys, Home, and End. The cursor is rendered as a block character.
    pub fn textarea(&mut self, state: &mut TextareaState, visible_rows: u32) -> &mut Self {
        if state.lines.is_empty() {
            state.lines.push(String::new());
        }
        state.cursor_row = state.cursor_row.min(state.lines.len().saturating_sub(1));
        state.cursor_col = state
            .cursor_col
            .min(state.lines[state.cursor_row].chars().count());

        let focused = self.register_focusable();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(ch) => {
                            let index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            state.lines[state.cursor_row].insert(index, ch);
                            state.cursor_col += 1;
                            consumed_indices.push(i);
                        }
                        KeyCode::Enter => {
                            let split_index = byte_index_for_char(
                                &state.lines[state.cursor_row],
                                state.cursor_col,
                            );
                            let remainder = state.lines[state.cursor_row].split_off(split_index);
                            state.cursor_row += 1;
                            state.lines.insert(state.cursor_row, remainder);
                            state.cursor_col = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::Backspace => {
                            if state.cursor_col > 0 {
                                let start = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col - 1,
                                );
                                let end = byte_index_for_char(
                                    &state.lines[state.cursor_row],
                                    state.cursor_col,
                                );
                                state.lines[state.cursor_row].replace_range(start..end, "");
                                state.cursor_col -= 1;
                            } else if state.cursor_row > 0 {
                                let current = state.lines.remove(state.cursor_row);
                                state.cursor_row -= 1;
                                state.cursor_col = state.lines[state.cursor_row].chars().count();
                                state.lines[state.cursor_row].push_str(&current);
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Left => {
                            if state.cursor_col > 0 {
                                state.cursor_col -= 1;
                            } else if state.cursor_row > 0 {
                                state.cursor_row -= 1;
                                state.cursor_col = state.lines[state.cursor_row].chars().count();
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            let line_len = state.lines[state.cursor_row].chars().count();
                            if state.cursor_col < line_len {
                                state.cursor_col += 1;
                            } else if state.cursor_row + 1 < state.lines.len() {
                                state.cursor_row += 1;
                                state.cursor_col = 0;
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Up => {
                            if state.cursor_row > 0 {
                                state.cursor_row -= 1;
                                state.cursor_col = state
                                    .cursor_col
                                    .min(state.lines[state.cursor_row].chars().count());
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Down => {
                            if state.cursor_row + 1 < state.lines.len() {
                                state.cursor_row += 1;
                                state.cursor_col = state
                                    .cursor_col
                                    .min(state.lines[state.cursor_row].chars().count());
                            }
                            consumed_indices.push(i);
                        }
                        KeyCode::Home => {
                            state.cursor_col = 0;
                            consumed_indices.push(i);
                        }
                        KeyCode::End => {
                            state.cursor_col = state.lines[state.cursor_row].chars().count();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Column,
            gap: 0,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for row in 0..visible_rows as usize {
            let line = state.lines.get(row).cloned().unwrap_or_default();
            let mut rendered = line.clone();
            let mut style = if line.is_empty() {
                Style::new().fg(self.theme.text_dim)
            } else {
                Style::new().fg(self.theme.text)
            };

            if focused && row == state.cursor_row {
                rendered.clear();
                for (idx, ch) in line.chars().enumerate() {
                    if idx == state.cursor_col {
                        rendered.push('▎');
                    }
                    rendered.push(ch);
                }
                if state.cursor_col >= line.chars().count() {
                    rendered.push('▎');
                }
                style = Style::new().fg(self.theme.text);
            }

            self.styled(rendered, style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a progress bar (20 chars wide). `ratio` is clamped to `0.0..=1.0`.
    ///
    /// Uses block characters (`█` filled, `░` empty). For a custom width use
    /// [`Context::progress_bar`].
    pub fn progress(&mut self, ratio: f64) -> &mut Self {
        self.progress_bar(ratio, 20)
    }

    /// Render a progress bar with a custom character width.
    ///
    /// `ratio` is clamped to `0.0..=1.0`. `width` is the total number of
    /// characters rendered.
    pub fn progress_bar(&mut self, ratio: f64, width: u32) -> &mut Self {
        let clamped = ratio.clamp(0.0, 1.0);
        let filled = (clamped * width as f64).round() as u32;
        let empty = width.saturating_sub(filled);
        let mut bar = String::new();
        for _ in 0..filled {
            bar.push('█');
        }
        for _ in 0..empty {
            bar.push('░');
        }
        self.text(bar)
    }

    /// Render a selectable list. Handles Up/Down (and `k`/`j`) navigation when focused.
    ///
    /// The selected item is highlighted with the theme's primary color. If the
    /// list is empty, nothing is rendered.
    pub fn list(&mut self, state: &mut ListState) -> &mut Self {
        if state.items.is_empty() {
            state.selected = 0;
            return self;
        }

        let focused = self.register_focusable();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected = (state.selected + 1).min(state.items.len() - 1);
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        for (idx, item) in state.items.iter().enumerate() {
            if idx == state.selected {
                if focused {
                    self.styled(
                        format!("▸ {item}"),
                        Style::new().bold().fg(self.theme.primary),
                    );
                } else {
                    self.styled(format!("▸ {item}"), Style::new().fg(self.theme.primary));
                }
            } else {
                self.styled(format!("  {item}"), Style::new().fg(self.theme.text));
            }
        }

        self
    }

    /// Render a data table with column headers. Handles Up/Down selection when focused.
    ///
    /// Column widths are computed automatically from header and cell content.
    /// The selected row is highlighted with the theme's selection colors.
    pub fn table(&mut self, state: &mut TableState) -> &mut Self {
        if state.is_dirty() {
            state.recompute_widths();
        }

        let focused = self.register_focusable();

        if focused && !state.rows.is_empty() {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            state.selected = state.selected.saturating_sub(1);
                            consumed_indices.push(i);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            state.selected = (state.selected + 1).min(state.rows.len() - 1);
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }
            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        state.selected = state.selected.min(state.rows.len().saturating_sub(1));

        let header_line = format_table_row(&state.headers, state.column_widths(), " │ ");
        self.styled(header_line, Style::new().bold().fg(self.theme.text));

        let separator = state
            .column_widths()
            .iter()
            .map(|w| "─".repeat(*w as usize))
            .collect::<Vec<_>>()
            .join("─┼─");
        self.text(separator);

        for (idx, row) in state.rows.iter().enumerate() {
            let line = format_table_row(row, state.column_widths(), " │ ");
            if idx == state.selected {
                let mut style = Style::new()
                    .bg(self.theme.selected_bg)
                    .fg(self.theme.selected_fg);
                if focused {
                    style = style.bold();
                }
                self.styled(line, style);
            } else {
                self.styled(line, Style::new().fg(self.theme.text));
            }
        }

        self
    }

    /// Render a tab bar. Handles Left/Right navigation when focused.
    ///
    /// The active tab is rendered in the theme's primary color. If the labels
    /// list is empty, nothing is rendered.
    pub fn tabs(&mut self, state: &mut TabsState) -> &mut Self {
        if state.labels.is_empty() {
            state.selected = 0;
            return self;
        }

        state.selected = state.selected.min(state.labels.len() - 1);
        let focused = self.register_focusable();

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Left => {
                            state.selected = if state.selected == 0 {
                                state.labels.len() - 1
                            } else {
                                state.selected - 1
                            };
                            consumed_indices.push(i);
                        }
                        KeyCode::Right => {
                            state.selected = (state.selected + 1) % state.labels.len();
                            consumed_indices.push(i);
                        }
                        _ => {}
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for (idx, label) in state.labels.iter().enumerate() {
            let style = if idx == state.selected {
                let s = Style::new().fg(self.theme.primary).bold();
                if focused {
                    s.underline()
                } else {
                    s
                }
            } else {
                Style::new().fg(self.theme.text_dim)
            };
            self.styled(format!("[ {label} ]"), style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a clickable button. Returns `true` when activated via Enter, Space, or mouse click.
    ///
    /// The button is styled with the theme's primary color when focused and the
    /// accent color when hovered.
    pub fn button(&mut self, label: impl Into<String>) -> bool {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);

        let mut activated = response.clicked;
        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        activated = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        let style = if focused {
            Style::new().fg(self.theme.primary).bold()
        } else if response.hovered {
            Style::new().fg(self.theme.accent)
        } else {
            Style::new().fg(self.theme.text)
        };

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 0,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        self.styled(format!("[ {} ]", label.into()), style);
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        activated
    }

    /// Render a checkbox. Toggles the bool on Enter, Space, or click.
    ///
    /// The checked state is shown with the theme's success color. When focused,
    /// a `▸` prefix is added.
    pub fn checkbox(&mut self, label: impl Into<String>, checked: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *checked = !*checked;
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 1,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        let marker_style = if *checked {
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        let marker = if *checked { "[x]" } else { "[ ]" };
        let label_text = label.into();
        if focused {
            self.styled(format!("▸ {marker}"), marker_style.bold());
            self.styled(label_text, Style::new().fg(self.theme.text).bold());
        } else {
            self.styled(marker, marker_style);
            self.styled(label_text, Style::new().fg(self.theme.text));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render an on/off toggle switch.
    ///
    /// Toggles `on` when activated via Enter, Space, or click. The switch
    /// renders as `●━━ ON` or `━━● OFF` colored with the theme's success or
    /// dim color respectively.
    pub fn toggle(&mut self, label: impl Into<String>, on: &mut bool) -> &mut Self {
        let focused = self.register_focusable();
        let interaction_id = self.interaction_count;
        self.interaction_count += 1;
        let response = self.response_for(interaction_id);
        let mut should_toggle = response.clicked;

        if focused {
            let mut consumed_indices = Vec::new();
            for (i, event) in self.events.iter().enumerate() {
                if let Event::Key(key) = event {
                    if matches!(key.code, KeyCode::Enter | KeyCode::Char(' ')) {
                        should_toggle = true;
                        consumed_indices.push(i);
                    }
                }
            }

            for index in consumed_indices {
                self.consumed[index] = true;
            }
        }

        if should_toggle {
            *on = !*on;
        }

        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        let label_text = label.into();
        let switch = if *on { "●━━ ON" } else { "━━● OFF" };
        let switch_style = if *on {
            Style::new().fg(self.theme.success)
        } else {
            Style::new().fg(self.theme.text_dim)
        };
        if focused {
            self.styled(
                format!("▸ {label_text}"),
                Style::new().fg(self.theme.text).bold(),
            );
            self.styled(switch, switch_style.bold());
        } else {
            self.styled(label_text, Style::new().fg(self.theme.text));
            self.styled(switch, switch_style);
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    pub fn separator(&mut self) -> &mut Self {
        self.commands.push(Command::Text {
            content: "─".repeat(200),
            style: Style::new().fg(self.theme.border).dim(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.last_text_idx = Some(self.commands.len() - 1);
        self
    }

    /// Render a help bar showing keybinding hints.
    ///
    /// `bindings` is a slice of `(key, action)` pairs. Keys are rendered in the
    /// theme's primary color; actions in the dim text color. Pairs are separated
    /// by a `·` character.
    pub fn help(&mut self, bindings: &[(&str, &str)]) -> &mut Self {
        if bindings.is_empty() {
            return self;
        }

        self.interaction_count += 1;
        self.commands.push(Command::BeginContainer {
            direction: Direction::Row,
            gap: 2,
            align: Align::Start,
            border: None,
            border_style: Style::new().fg(self.theme.border),
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
        });
        for (idx, (key, action)) in bindings.iter().enumerate() {
            if idx > 0 {
                self.styled("·", Style::new().fg(self.theme.text_dim));
            }
            self.styled(*key, Style::new().bold().fg(self.theme.primary));
            self.styled(*action, Style::new().fg(self.theme.text_dim));
        }
        self.commands.push(Command::EndContainer);
        self.last_text_idx = None;

        self
    }

    // ── events ───────────────────────────────────────────────────────

    /// Check if a character key was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key(&self, c: char) -> bool {
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i] && matches!(e, Event::Key(k) if k.code == KeyCode::Char(c))
        })
    }

    /// Check if a specific key code was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_code(&self, code: KeyCode) -> bool {
        self.events
            .iter()
            .enumerate()
            .any(|(i, e)| !self.consumed[i] && matches!(e, Event::Key(k) if k.code == code))
    }

    /// Check if a character key with specific modifiers was pressed this frame.
    ///
    /// Returns `true` if the key event has not been consumed by another widget.
    pub fn key_mod(&self, c: char, modifiers: KeyModifiers) -> bool {
        self.events.iter().enumerate().any(|(i, e)| {
            !self.consumed[i]
                && matches!(e, Event::Key(k) if k.code == KeyCode::Char(c) && k.modifiers.contains(modifiers))
        })
    }

    /// Return the position of a left mouse button down event this frame, if any.
    ///
    /// Returns `None` if no unconsumed mouse-down event occurred.
    pub fn mouse_down(&self) -> Option<(u32, u32)> {
        self.events.iter().enumerate().find_map(|(i, event)| {
            if self.consumed[i] {
                return None;
            }
            if let Event::Mouse(mouse) = event {
                if matches!(mouse.kind, MouseKind::Down(MouseButton::Left)) {
                    return Some((mouse.x, mouse.y));
                }
            }
            None
        })
    }

    /// Return the current mouse cursor position, if known.
    ///
    /// The position is updated on every mouse move or click event. Returns
    /// `None` until the first mouse event is received.
    pub fn mouse_pos(&self) -> Option<(u32, u32)> {
        self.mouse_pos
    }

    /// Check if an unconsumed scroll-up event occurred this frame.
    pub fn scroll_up(&self) -> bool {
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollUp))
        })
    }

    /// Check if an unconsumed scroll-down event occurred this frame.
    pub fn scroll_down(&self) -> bool {
        self.events.iter().enumerate().any(|(i, event)| {
            !self.consumed[i]
                && matches!(event, Event::Mouse(mouse) if matches!(mouse.kind, MouseKind::ScrollDown))
        })
    }

    /// Signal the run loop to exit after this frame.
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Get the current theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Change the theme for subsequent rendering.
    ///
    /// All widgets rendered after this call will use the new theme's colors.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    // ── info ─────────────────────────────────────────────────────────

    /// Get the terminal width in cells.
    pub fn width(&self) -> u32 {
        self.area_width
    }

    /// Get the terminal height in cells.
    pub fn height(&self) -> u32 {
        self.area_height
    }

    /// Get the current tick count (increments each frame).
    ///
    /// Useful for animations and time-based logic. The tick starts at 0 and
    /// increases by 1 on every rendered frame.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Return whether the layout debugger is enabled.
    ///
    /// The debugger is toggled with F12 at runtime.
    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

fn byte_index_for_char(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_index)
        .map_or(value.len(), |(idx, _)| idx)
}

fn format_table_row(cells: &[String], widths: &[u32], separator: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, width) in widths.iter().enumerate() {
        let cell = cells.get(i).map(String::as_str).unwrap_or("");
        let cell_width = UnicodeWidthStr::width(cell) as u32;
        let padding = (*width).saturating_sub(cell_width) as usize;
        parts.push(format!("{cell}{}", " ".repeat(padding)));
    }
    parts.join(separator)
}
