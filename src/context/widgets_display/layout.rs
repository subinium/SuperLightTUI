use super::*;
use std::sync::LazyLock;

static SEP_LINE: LazyLock<String> = LazyLock::new(|| "─".repeat(200));

fn sep_line() -> &'static str {
    &SEP_LINE
}

/// Compass-rose anchor for [`Context::overlay_at`] / [`Context::modal_at`].
///
/// Each variant maps to a (cross-axis [`Align`], main-axis [`Justify`]) pair
/// that pins overlay content to the requested screen position. The `_at`
/// helpers expand to a full-screen wrapper (so flexbox has slack to push
/// against), then place the user's content per the selected anchor.
///
/// ```no_run
/// # use slt::Anchor;
/// # slt::run(|ui: &mut slt::Context| {
/// ui.overlay_at(Anchor::BottomRight, |ui| {
///     ui.text("v0.19.3").dim();
/// });
/// # });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top-left corner.
    TopLeft,
    /// Top edge, horizontally centered.
    TopCenter,
    /// Top-right corner.
    TopRight,
    /// Left edge, vertically centered.
    CenterLeft,
    /// Screen center.
    Center,
    /// Right edge, vertically centered.
    CenterRight,
    /// Bottom-left corner.
    BottomLeft,
    /// Bottom edge, horizontally centered.
    BottomCenter,
    /// Bottom-right corner.
    BottomRight,
}

/// Map [`Anchor`] to the wrapper column's (cross-axis align, main-axis justify).
///
/// The inner column is `Direction::Column`, so:
///   - `Justify` controls the vertical (main-axis) position.
///   - `Align`   controls the horizontal (cross-axis) position.
fn anchor_to_align_justify(anchor: Anchor) -> (Align, Justify) {
    match anchor {
        Anchor::TopLeft => (Align::Start, Justify::Start),
        Anchor::TopCenter => (Align::Center, Justify::Start),
        Anchor::TopRight => (Align::End, Justify::Start),
        Anchor::CenterLeft => (Align::Start, Justify::Center),
        Anchor::Center => (Align::Center, Justify::Center),
        Anchor::CenterRight => (Align::End, Justify::Center),
        Anchor::BottomLeft => (Align::Start, Justify::End),
        Anchor::BottomCenter => (Align::Center, Justify::End),
        Anchor::BottomRight => (Align::End, Justify::End),
    }
}

/// Resolve `(dx, dy)` to a [`Margin`] for the outer grow-1 anchor column,
/// given an [`Anchor`].
///
/// Sign convention: **positive `dx` / `dy` inset toward the viewport center**
/// (mirrors the CSS `inset` shorthand intuition). The margin shrinks the
/// column's slack on the side adjacent to the anchored edge, so subsequent
/// flexbox `align`/`justify` push the user's content inward by `(dx, dy)`:
///   - `BottomRight` + `(dx=2, dy=1)` → `mr=2, mb=1` (push 2 left, 1 up)
///   - `TopLeft`     + `(dx=2, dy=1)` → `ml=2, mt=1` (push 2 right, 1 down)
///   - `Center`      + `(dx=2, dy=1)` → `ml=2, mt=1` (shift 2 right, 1 down)
///   - `Center`      + `(dx=-2, dy=-1)` → `mr=2, mb=1` (shift 2 left, 1 up)
///
/// Negative values for corner / edge anchors would push the content
/// off-screen (no opposite-side slack to consume), so they are clamped to 0;
/// see [`Context::overlay_at_offset`] for the documented contract.
fn anchor_offset_to_margin(anchor: Anchor, dx: i32, dy: i32) -> Margin {
    let mut margin = Margin::default();

    // Horizontal axis: positive dx insets toward center.
    let h_anchor = match anchor {
        Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => HSide::Left,
        Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => HSide::Right,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => HSide::Center,
    };
    match h_anchor {
        HSide::Left => {
            // Anchored to left edge: positive dx pushes right via ml.
            // Negative dx would push left (offscreen) — no slack on the
            // opposite side, and `u32` margin can't represent negatives,
            // so we clamp to 0. See `Context::overlay_at_offset` doc.
            if dx > 0 {
                margin.left = dx as u32;
            }
        }
        HSide::Right => {
            // Anchored to right edge: positive dx pushes left via mr.
            if dx > 0 {
                margin.right = dx as u32;
            }
        }
        HSide::Center => {
            // Centered: positive dx shifts right (ml), negative shifts left (mr).
            if dx > 0 {
                margin.left = dx as u32;
            } else if dx < 0 {
                margin.right = dx.unsigned_abs();
            }
        }
    }

    // Vertical axis: positive dy insets toward center.
    let v_anchor = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => VSide::Top,
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => VSide::Bottom,
        Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => VSide::Center,
    };
    match v_anchor {
        VSide::Top => {
            if dy > 0 {
                margin.top = dy as u32;
            }
        }
        VSide::Bottom => {
            if dy > 0 {
                margin.bottom = dy as u32;
            }
        }
        VSide::Center => {
            if dy > 0 {
                margin.top = dy as u32;
            } else if dy < 0 {
                margin.bottom = dy.unsigned_abs();
            }
        }
    }

    margin
}

enum HSide {
    Left,
    Right,
    Center,
}

enum VSide {
    Top,
    Bottom,
    Center,
}

impl Context {
    /// Render a horizontal divider line.
    ///
    /// The line is drawn with the theme's border color and expands to fill the
    /// container width.
    ///
    /// Returns a [`Response`] so the divider's hit-test rect is available for
    /// hover detection. Prior to v0.21.0 this returned `&mut Self`, but the
    /// chained style mutators (`.bold()`, `.fg()`) were a no-op — the cached
    /// separator string is already finalized — so the chain was dropped.
    /// Statement-form callers (`ui.separator();`) compile unchanged.
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.separator();
    /// # });
    /// ```
    pub fn separator(&mut self) -> Response {
        let response = self.interaction();
        // The cached `sep_line()` is much wider than any reasonable terminal,
        // so the cross-axis (column-direction) clip in `Buffer::set_string`
        // truncates the trailing chars. Keeping `grow = 0` means a column
        // layout doesn't stretch the separator vertically, and `truncate =
        // false` avoids the ellipsis fallback which would otherwise replace
        // the last cell with `…`.
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            style: Style::new().fg(self.theme.border).dim(),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        response
    }

    /// Render a horizontal separator line with a custom color.
    ///
    /// Returns a [`Response`] for hover detection; see [`Context::separator`]
    /// for the v0.21.0 return-shape change. Statement-form callers compile
    /// unchanged.
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.separator_colored(Color::Cyan);
    /// # });
    /// ```
    pub fn separator_colored(&mut self, color: Color) -> Response {
        let response = self.interaction();
        self.commands.push(Command::Text {
            content: sep_line().to_owned(),
            cursor_offset: None,
            style: Style::new().fg(color),
            grow: 0,
            align: Align::Start,
            wrap: false,
            truncate: false,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = Some(self.commands.len() - 1);
        response
    }

    /// Conditionally render content when the named screen is active.
    ///
    /// Each screen gets an isolated hook segment — `use_state` / `use_memo`
    /// calls inside one screen do not interfere with another screen's hooks,
    /// even when you switch between screens across frames.
    ///
    /// Focus state is saved and restored per screen automatically.
    /// Navigation requested inside a screen mutates `ScreenState` when that
    /// closure returns, while the frame finishes rendering the screen that was
    /// active at its start. The destination renders on the next frame, so
    /// source and destination content are never composed together.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut screens = slt::ScreenState::new("main");
    /// # slt::run(|ui| {
    /// ui.screen("main", &mut screens, |ui| {
    ///     ui.text("Main screen");
    /// });
    /// # });
    /// ```
    pub fn screen(&mut self, name: &str, screens: &mut ScreenState, f: impl FnOnce(&mut Context)) {
        // Look up (or create) this screen's reserved hook segment.
        //
        // Cache-hit path is the steady state — every frame after the first.
        // Avoid the unconditional `name.to_string()` `entry()` allocation by
        // checking first via `&str` lookup. Only the first frame for a
        // given screen pays the `to_string()` cost. Closes #134 (Option B).
        let (seg_start, seg_count) = if let Some(&v) = self.screen_hook_map.get(name) {
            v
        } else {
            let v = (self.hook_states.len(), 0);
            self.screen_hook_map.insert(name.to_string(), v);
            v
        };

        let screen_key = screens as *mut ScreenState as usize;
        let is_active = self
            .screen_nav_render_origins
            .get(&screen_key)
            .map_or_else(|| screens.current() == name, |origin| origin == name);

        if is_active {
            // Save outer focus, restore this screen's focus
            let outer_focus_index = self.focus_index;
            let (saved_focus_idx, _saved_focus_count) = screens.restore_focus(name);
            self.focus_index = saved_focus_idx;

            // Set hook cursor to this screen's segment start
            self.rollback.hook_cursor = seg_start;
            let focus_count_before = self.rollback.focus_count;

            // Scope deferred navigation to this exact screen closure. Nested
            // screens get their own range and cannot drain an outer request.
            let nav_scope_start = self.pending_screen_nav.len();
            self.screen_nav_depth += 1;

            // Execute the screen's closure
            f(self);

            self.screen_nav_depth = self
                .screen_nav_depth
                .checked_sub(1)
                .expect("active screen must own a navigation scope");

            // Record the hook count for this screen.
            //
            // The first-frame path above already inserted an owned `String`
            // key for this screen; subsequent frames reuse it. Locate that
            // existing slot via `&str` and overwrite the value in place,
            // avoiding a second `to_string()` allocation per active frame.
            let hooks_used = self.rollback.hook_cursor - seg_start;
            if let Some(slot) = self.screen_hook_map.get_mut(name) {
                *slot = (seg_start, hooks_used);
            } else {
                self.screen_hook_map
                    .insert(name.to_string(), (seg_start, hooks_used));
            }

            // Save this screen's focus state
            let screen_focus_count = self.rollback.focus_count - focus_count_before;
            screens.save_focus(name, self.focus_index, screen_focus_count);

            // Restore outer focus
            self.focus_index = outer_focus_index;

            // Issue #279: apply navigation requested from inside the closure
            // now that the closure's `&mut Context` borrow has ended. We still
            // hold `&mut screens` here, so there is no double mutable borrow —
            // app code can call `ui.push_screen(...)` / `ui.pop_screen()` from
            // within the closure without the borrow conflict from the issue.
            if self.pending_screen_nav.len() > nav_scope_start {
                // Preserve the screen that rendered at the start of this
                // transition. ScreenState changes immediately, but later
                // declarations wait until the next frame to render the new
                // destination, avoiding a one-frame source/destination blend.
                self.screen_nav_render_origins
                    .entry(screen_key)
                    .or_insert_with(|| screens.current().to_owned());
                for nav in self.pending_screen_nav.drain(nav_scope_start..) {
                    screens.apply_nav(nav);
                }
            }
        } else {
            // Skip: advance hook cursor past the reserved segment
            if seg_count > 0 && seg_start >= self.rollback.hook_cursor {
                self.rollback.hook_cursor = seg_start + seg_count;
            }
        }
    }

    /// Request pushing a new screen onto the active [`ScreenState`] stack.
    ///
    /// Call this from inside a [`Context::screen`] closure to navigate forward.
    /// The push is deferred and applied to your `ScreenState` the moment the
    /// closure returns, so it does not conflict with the `&mut ScreenState`
    /// already borrowed by `screen(...)` (issue #279). Rendering stays on the
    /// source screen for that transition frame; the destination appears on the
    /// next frame.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut screens = slt::ScreenState::new("home");
    /// # slt::run(|ui| {
    /// ui.screen("home", &mut screens, |ui| {
    ///     if ui.button("Settings").clicked {
    ///         ui.push_screen("settings");
    ///     }
    /// });
    /// # });
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::push`] directly when navigating outside a screen.
    #[track_caller]
    pub fn push_screen(&mut self, name: impl Into<String>) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Push(name.into()));
    }

    /// Request popping the current screen off the active [`ScreenState`] stack
    /// (the root screen is preserved).
    ///
    /// Like [`Self::push_screen`], the pop is deferred and applied when the
    /// enclosing [`Context::screen`] closure returns (issue #279).
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::pop`] directly when navigating outside a screen.
    #[track_caller]
    pub fn pop_screen(&mut self) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Pop);
    }

    /// Request resetting the active [`ScreenState`] stack to just its root
    /// screen.
    ///
    /// Deferred and applied when the enclosing [`Context::screen`] closure
    /// returns (issue #279).
    ///
    /// # Panics
    ///
    /// Panics when called outside an active [`Context::screen`] closure. Use
    /// [`ScreenState::reset`] directly when navigating outside a screen.
    #[track_caller]
    pub fn reset_screen(&mut self) {
        self.assert_screen_nav_scope();
        self.pending_screen_nav.push(ScreenNav::Reset);
    }

    #[track_caller]
    fn assert_screen_nav_scope(&self) {
        assert!(
            self.screen_nav_depth > 0,
            "screen navigation helpers can only be called inside an active Context::screen closure; mutate ScreenState directly outside a screen"
        );
    }

    /// Remove retained hook/focus state for an inactive screen.
    ///
    /// Returns `false` when `name` is still present in `screens`' stack. Call
    /// this after popping runtime-generated detail screens to release their
    /// isolated hook segment and saved focus entry.
    pub fn remove_screen_state(&mut self, screens: &mut ScreenState, name: &str) -> bool {
        if screens.contains(name) {
            return false;
        }
        let focus_removed = screens.remove_inactive(name);
        let hooks_removed = self.remove_screen_hooks(name);
        focus_removed || hooks_removed
    }

    /// Retain inactive screen states accepted by `keep`.
    ///
    /// Screens currently present in `screens`' stack are always kept. Returns
    /// the number of screen names removed from either the hook map or the
    /// saved-focus map.
    pub fn retain_screen_state(
        &mut self,
        screens: &mut ScreenState,
        mut keep: impl FnMut(&str) -> bool,
    ) -> usize {
        let remove_names: Vec<String> = self
            .screen_hook_map
            .keys()
            .filter(|name| !screens.contains(name) && !keep(name))
            .cloned()
            .collect();

        let mut removed = 0;
        for name in remove_names {
            if self.remove_screen_state(screens, &name) {
                removed += 1;
            }
        }
        removed + screens.retain_inactive(keep)
    }

    /// Number of retained screen hook segments.
    ///
    /// Diagnostic helper for apps that generate screen names from runtime ids.
    pub fn screen_state_count(&self) -> usize {
        self.screen_hook_map.len()
    }

    fn remove_screen_hooks(&mut self, name: &str) -> bool {
        let Some((seg_start, seg_count)) = self.screen_hook_map.remove(name) else {
            return false;
        };

        if seg_count == 0 {
            return true;
        }

        let end = seg_start
            .saturating_add(seg_count)
            .min(self.hook_states.len());
        if seg_start >= end {
            return true;
        }

        let removed = end - seg_start;
        self.hook_states.drain(seg_start..end);
        for (other_start, _) in self.screen_hook_map.values_mut() {
            if *other_start > seg_start {
                *other_start = other_start.saturating_sub(removed);
            }
        }
        if self.rollback.hook_cursor > seg_start {
            self.rollback.hook_cursor = self.rollback.hook_cursor.saturating_sub(removed);
        }
        true
    }

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
    ///
    /// **Deprecated since 0.20.1**: the name collides with
    /// [`ContainerBuilder::col_gap`], which sets the *row-finalize* main-axis
    /// gap (Tailwind `gap-x` axis convention) and so means the opposite thing.
    /// Use `ui.container().gap(n).col(f)` instead — same output, no collision.
    #[deprecated(
        since = "0.20.1",
        note = "Use `ui.container().gap(n).col(f)` instead — same output, no name collision with `ContainerBuilder::col_gap`."
    )]
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
    ///
    /// **Deprecated since 0.20.1**: the name collides with
    /// [`ContainerBuilder::row_gap`], which sets the *column-finalize*
    /// main-axis gap (Tailwind `gap-y` axis convention) and so means the
    /// opposite thing. Use `ui.container().gap(n).row(f)` instead — same
    /// output, no collision.
    #[deprecated(
        since = "0.20.1",
        note = "Use `ui.container().gap(n).row(f)` instead — same output, no name collision with `ContainerBuilder::row_gap`."
    )]
    pub fn row_gap(&mut self, gap: u32, f: impl FnOnce(&mut Context)) -> Response {
        self.push_container(Direction::Row, gap, f)
    }

    /// Render inline text with mixed styles on a single line.
    ///
    /// Unlike [`row`](Context::row), `line()` is designed for rich text —
    /// children are rendered as continuous inline text without gaps.
    ///
    /// It intentionally returns `&mut Self` instead of [`Response`] so you can
    /// keep chaining display-oriented modifiers after composing the inline run.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Color;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line(|ui| {
    ///     ui.text("Status: ");
    ///     ui.text("Online").bold().fg(Color::Green);
    /// });
    /// # });
    /// ```
    pub fn line(&mut self, f: impl FnOnce(&mut Context)) -> &mut Self {
        let _ = self.push_container(Direction::Row, 0, f);
        self
    }

    /// Render inline text with mixed styles, wrapping at word boundaries.
    ///
    /// Like [`line`](Context::line), but when the combined text exceeds
    /// the container width it wraps across multiple lines while
    /// preserving per-segment styles.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{Color, Style};
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.line_wrap(|ui| {
    ///     ui.text("This is a long ");
    ///     ui.text("important").bold().fg(Color::Red);
    ///     ui.text(" message that wraps across lines");
    /// });
    /// # });
    /// ```
    pub fn line_wrap(&mut self, f: impl FnOnce(&mut Context)) -> &mut Self {
        let start = self.commands.len();
        f(self);
        let has_link = self.commands[start..]
            .iter()
            .any(|cmd| matches!(cmd, Command::Link { .. }));

        if has_link {
            self.commands.insert(
                start,
                Command::BeginContainer(Box::new(BeginContainerArgs {
                    direction: Direction::Row,
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
                    group_name: None,
                })),
            );
            self.commands.push(Command::EndContainer);
            self.rollback.last_text_idx = None;
            return self;
        }

        let mut segments: Vec<(String, Style)> = Vec::new();
        for cmd in self.commands.drain(start..) {
            match cmd {
                Command::Text { content, style, .. } => {
                    segments.push((content, style));
                }
                Command::Link { text, style, .. } => {
                    // Preserve link text with underline styling (URL lost in RichText,
                    // but text is visible and wraps correctly)
                    segments.push((text, style));
                }
                _ => {}
            }
        }
        self.commands.push(Command::RichText {
            segments,
            wrap: true,
            align: Align::Start,
            margin: Margin::default(),
            constraints: Constraints::default(),
        });
        self.rollback.last_text_idx = None;
        self
    }

    /// Render content in a modal overlay with dimmed background.
    ///
    /// ```no_run
    /// # let mut show = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// if show {
    ///     ui.modal(|ui| {
    ///         ui.text("Are you sure?");
    ///         if ui.button("OK").clicked { show = false; }
    ///     });
    /// }
    /// # });
    /// ```
    pub fn modal(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        // Default `modal()` preserves legacy behavior (tab_trap = false).
        // `modal_with(ModalOptions::default(), ...)` opts into the WCAG 2.1
        // SC 2.4.3 focus-trap default. This split keeps existing callers
        // bit-identical until they migrate.
        self.modal_with(ModalOptions { tab_trap: false }, f)
    }

    /// Render content in a modal overlay with configurable options.
    ///
    /// Like [`modal`](Self::modal), but accepts a [`ModalOptions`] struct.
    /// Use this to opt into focus trapping (`tab_trap: true`) or future
    /// modal flags without breaking the bare `modal()` API.
    ///
    /// When `opts.tab_trap` is `true`, focus cannot escape the modal's
    /// focusable range — Tab/Shift+Tab keep cycling within the modal even
    /// if [`Context::set_focus_index`] or a mouse click moved focus to a
    /// background widget. WCAG 2.1 SC 2.4.3 (Focus Order) recommends
    /// trapping focus inside modal dialogs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # let mut show = true;
    /// # slt::run(|ui: &mut slt::Context| {
    /// if show {
    ///     ui.modal_with(slt::context::ModalOptions { tab_trap: true }, |ui| {
    ///         ui.text("Are you sure?");
    ///         if ui.button("OK").clicked { show = false; }
    ///     });
    /// }
    /// # });
    /// ```
    pub fn modal_with(&mut self, opts: ModalOptions, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: true });
        self.rollback.overlay_depth += 1;
        self.rollback.modal_active = true;
        let modal_focus_start = self.rollback.focus_count;
        self.rollback.modal_focus_start = modal_focus_start;

        f(self);
        let modal_focus_count = self.rollback.focus_count.saturating_sub(modal_focus_start);
        self.rollback.modal_focus_count = modal_focus_count;

        // Tab trap: when enabled, ensure `focus_index` lies in this frame's
        // modal range `[start, start + count)`. If `set_focus_index` from a
        // previous frame (or a stale state) left focus pointing at a
        // background widget, clamp it to the first modal focusable so the
        // next [`process_focus_keys`] tick cycles cleanly within the modal.
        //
        // WCAG 2.1 SC 2.4.3 (Focus Order) requirement: the user must not be
        // able to navigate to content outside an active modal dialog.
        if opts.tab_trap && modal_focus_count > 0 {
            let lo = modal_focus_start;
            let hi = lo.saturating_add(modal_focus_count);
            if self.focus_index < lo || self.focus_index >= hi {
                self.focus_index = lo;
            }
        }

        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content without dimming the background.
    pub fn overlay(&mut self, f: impl FnOnce(&mut Context)) -> Response {
        let interaction_id = self.next_interaction_id();
        self.commands.push(Command::BeginOverlay { modal: false });
        self.rollback.overlay_depth += 1;
        f(self);
        self.rollback.overlay_depth = self.rollback.overlay_depth.saturating_sub(1);
        self.commands.push(Command::EndOverlay);
        self.rollback.last_text_idx = None;
        self.response_for(interaction_id)
    }

    /// Render floating content anchored to one of the 9 compass positions.
    ///
    /// Wraps [`overlay`](Self::overlay) with a full-area column that pins the
    /// content to the requested anchor via flexbox `align`/`justify`. The
    /// inner column gets `grow(1)` so the wrapper consumes the screen, giving
    /// `align`/`justify` room to push the content to the corner.
    ///
    /// ```no_run
    /// # use slt::Anchor;
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.overlay_at(Anchor::TopRight, |ui| {
    ///     ui.text("0:42").bold();
    /// });
    /// # });
    /// ```
    pub fn overlay_at(&mut self, anchor: Anchor, f: impl FnOnce(&mut Context)) -> Response {
        self.overlay(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let _ = ui.container().grow(1).align(align).justify(justify).col(f);
        })
    }

    /// Render a modal overlay anchored to one of the 9 compass positions.
    ///
    /// Like [`modal`](Self::modal) but pinned to a corner / edge / center via
    /// the same anchor wrapping as [`overlay_at`](Self::overlay_at).
    pub fn modal_at(&mut self, anchor: Anchor, f: impl FnOnce(&mut Context)) -> Response {
        self.modal(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let _ = ui.container().grow(1).align(align).justify(justify).col(f);
        })
    }

    /// Render `f` at `anchor` with cell offset `(dx, dy)` from the anchored edge.
    ///
    /// This is the SLT analog of CSS `position: absolute; top/right/bottom/left`,
    /// or Flutter's `Positioned(top:, right:, ...)`. The 9-cell [`Anchor`]
    /// chooses which edge to anchor to; `(dx, dy)` insets toward the center.
    ///
    /// # Sign convention
    /// Positive `dx` / `dy` always inset toward the viewport center. So
    /// `overlay_at_offset(Anchor::BottomRight, 2, 1, ...)` places the widget
    /// 2 cells left and 1 cell up from the bottom-right corner.
    ///
    /// For [`Anchor::Center`] (and other centered axes) negative values shift
    /// in the opposite direction — `(dx=-2, dy=-1)` shifts 2 cells left and 1
    /// cell up. For corner / edge anchors, negative values would push the
    /// content off-screen, so they are clamped to 0; use a different anchor
    /// instead of negative offsets to escape an edge.
    ///
    /// # CSS analogy
    /// ```text
    /// CSS:    place-self: end end; bottom: 1px; right: 2px;
    /// SLT:    overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| { ... })
    /// ```
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::Anchor;
    /// # slt::run(|ui: &mut slt::Context| {
    /// // Inset corner badge — 2 cells from the right, 1 row from the bottom.
    /// ui.overlay_at_offset(Anchor::BottomRight, 2, 1, |ui| {
    ///     ui.text("v0.19.3").dim();
    /// });
    /// # });
    /// ```
    pub fn overlay_at_offset(
        &mut self,
        anchor: Anchor,
        dx: i32,
        dy: i32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.overlay(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let margin = anchor_offset_to_margin(anchor, dx, dy);
            // Apply margin on the outer (grow=1) column so flexbox's parent
            // (the synthetic overlay root) shrinks the column's area before
            // align/justify pick a position. This avoids a wrapper container
            // around `f`, which would expose a flexbox limitation where
            // `Align::End` shifts the immediate child's `pos` but does not
            // propagate the shift down to grandchildren.
            let _ = ui
                .container()
                .grow(1)
                .align(align)
                .justify(justify)
                .margin(margin)
                .col(f);
        })
    }

    /// Modal variant of [`overlay_at_offset`](Self::overlay_at_offset).
    ///
    /// Like [`modal_at`](Self::modal_at) but with a `(dx, dy)` cell inset
    /// from the anchored edge. Positive values inset toward the center —
    /// see [`overlay_at_offset`](Self::overlay_at_offset) for the full sign
    /// convention.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::{Anchor, Border};
    /// # slt::run(|ui: &mut slt::Context| {
    /// ui.modal_at_offset(Anchor::TopRight, 2, 1, |ui| {
    ///     ui.bordered(Border::Rounded).p(1).col(|ui| {
    ///         ui.text("Saved!");
    ///     });
    /// });
    /// # });
    /// ```
    pub fn modal_at_offset(
        &mut self,
        anchor: Anchor,
        dx: i32,
        dy: i32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.modal(|ui| {
            let (align, justify) = anchor_to_align_justify(anchor);
            let margin = anchor_offset_to_margin(anchor, dx, dy);
            // See `overlay_at_offset` for why margin lives on the outer
            // grow-1 column rather than a wrapper around `f`.
            let _ = ui
                .container()
                .grow(1)
                .align(align)
                .justify(justify)
                .margin(margin)
                .col(f);
        })
    }

    /// Render a hover tooltip for the previously rendered interactive widget.
    ///
    /// Call this right after a widget or container response:
    /// ```ignore
    /// if ui.button("Save").clicked { save(); }
    /// ui.tooltip("Save the current document to disk");
    /// ```
    pub fn tooltip(&mut self, text: impl Into<String>) {
        let tooltip_text = text.into();
        if tooltip_text.is_empty() {
            return;
        }
        let last_interaction_id = self.rollback.interaction_count.saturating_sub(1);
        let last_response = self.response_for(last_interaction_id);
        if !last_response.hovered || last_response.rect.width == 0 || last_response.rect.height == 0
        {
            return;
        }
        let lines = wrap_tooltip_text(&tooltip_text, 38);
        self.pending_tooltips.push(PendingTooltip {
            anchor_rect: last_response.rect,
            lines,
        });
    }

    pub(crate) fn emit_pending_tooltips(&mut self) {
        let tooltips = std::mem::take(&mut self.pending_tooltips);
        if tooltips.is_empty() {
            return;
        }
        let area_w = self.area_width;
        let area_h = self.area_height;
        let surface = self.theme.surface;
        let border_color = self.theme.border;
        let text_color = self.theme.surface_text;

        for tooltip in tooltips {
            let content_w = tooltip
                .lines
                .iter()
                .map(|l| UnicodeWidthStr::width(l.as_str()) as u32)
                .max()
                .unwrap_or(0);
            let box_w = content_w.saturating_add(4).min(area_w);
            let box_h = (tooltip.lines.len() as u32).saturating_add(4).min(area_h);

            let tooltip_x = tooltip.anchor_rect.x.min(area_w.saturating_sub(box_w));
            let below_y = tooltip.anchor_rect.bottom();
            let tooltip_y = if below_y.saturating_add(box_h) <= area_h {
                below_y
            } else {
                tooltip.anchor_rect.y.saturating_sub(box_h)
            };

            let lines = tooltip.lines;
            let pad = self.theme.spacing.xs();
            let _ = self.overlay(|ui| {
                let _ = ui.container().w(area_w).h(area_h).col(|ui| {
                    let _ = ui
                        .container()
                        .ml(tooltip_x)
                        .mt(tooltip_y)
                        .max_w(box_w)
                        .border(Border::Rounded)
                        .border_fg(border_color)
                        .bg(surface)
                        .p(pad)
                        .col(|ui| {
                            for line in &lines {
                                ui.text(line.as_str()).fg(text_color);
                            }
                        });
                });
            });
        }
    }

    /// Create a named group container for shared hover/focus styling.
    ///
    /// ```ignore
    /// ui.group("card").border(Border::Rounded)
    ///     .group_hover_bg(Color::Indexed(238))
    ///     .col(|ui| { ui.text("Hover anywhere"); });
    /// ```
    pub fn group(&mut self, name: &str) -> ContainerBuilder<'_> {
        // Materialize the name once; subsequent uses are cheap `Arc::clone`
        // pointer bumps. Closes #145 (double `to_string` allocation) and
        // completes the `Arc<str>` migration tracked by #139.
        self.rollback.group_count = self.rollback.group_count.saturating_add(1);
        let name_arc: std::sync::Arc<str> = std::sync::Arc::from(name);
        self.rollback
            .group_stack
            .push(std::sync::Arc::clone(&name_arc));
        self.container().group_name_arc(name_arc)
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
    ///     .p(1)
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
            row_gap: None,
            col_gap: None,
            align: Align::Start,
            align_self_value: None,
            justify: Justify::Start,
            border: None,
            border_sides: BorderSides::all(),
            border_style: Style::new().fg(border),
            bg: None,
            text_color: None,
            dark_bg: None,
            dark_border_style: None,
            group_hover_bg: None,
            group_hover_border_style: None,
            group_name: None,
            padding: Padding::default(),
            margin: Margin::default(),
            constraints: Constraints::default(),
            title: None,
            grow: 0,
            shrink_flag: false,
            wrap_flag: false,
            basis: None,
            scroll_offset: None,
            scroll_offset_x: None,
            theme_override: None,
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
        let index = self.rollback.scroll_count;
        self.rollback.scroll_count += 1;
        // #247: the previous frame recorded the scroll axis (`is_horizontal`)
        // because this binding runs before `.row()` / `.col()` is known. Bind
        // the matching axis so a horizontal scrollable updates `offset_x` while
        // a vertical one keeps the byte-identical `offset` path.
        let mut is_horizontal = false;
        if let Some(&(content, viewport, horizontal)) = self.prev_scroll_infos.get(index) {
            is_horizontal = horizontal;
            let max = content.saturating_sub(viewport) as usize;
            if horizontal {
                state.set_bounds_x(content, viewport);
                state.offset_x = state.offset_x.min(max);
            } else {
                state.set_bounds(content, viewport);
                state.offset = state.offset.min(max);
            }
        }

        let next_id = self.rollback.interaction_count;
        if let Some(rect) = self.prev_hit_map.get(next_id).copied() {
            let inner_rects: Vec<Rect> = self
                .prev_scroll_rects
                .iter()
                .enumerate()
                .filter(|&(j, sr)| {
                    j != index
                        && sr.width > 0
                        && sr.height > 0
                        && sr.x >= rect.x
                        && sr.right() <= rect.right()
                        && sr.y >= rect.y
                        && sr.bottom() <= rect.bottom()
                })
                .map(|(_, sr)| *sr)
                .collect();
            self.auto_scroll_nested(&rect, state, &inner_rects, is_horizontal);
        }

        // Carry both axis offsets; the tree builder applies the one matching
        // the finalizing `.row()` / `.col()` direction (#247).
        let mut builder = self.container().scroll_offset(state.offset as u32);
        builder.scroll_offset_x = Some(state.offset_x as u32);
        builder
    }

    /// Scrollable column container — shortcut for
    /// `scrollable(state).grow(1).col(f)`.
    ///
    /// This is the form used by nearly every scrollable view: a vertical
    /// list that fills its parent and wheels through its own content. Use
    /// the explicit [`Context::scrollable`] builder when you need custom
    /// `grow`, borders, padding, or a scrollbar alongside.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scroll_col(&mut scroll, |ui| {
    ///     for i in 0..100 {
    ///         ui.text(format!("Line {i}"));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_col(
        &mut self,
        state: &mut ScrollState,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.scrollable(state).grow(1).col(f)
    }

    /// Scrollable row container — shortcut for
    /// `scrollable(state).grow(1).row(f)`.
    ///
    /// Lays children out left-to-right and scrolls **horizontally** when their
    /// combined width exceeds the viewport: useful for timelines, kanban
    /// boards, wide tables, Gantt strips, and long single-line log entries
    /// (#247). The horizontal axis is driven by
    /// [`ScrollState::scroll_left`] / [`ScrollState::scroll_right`], native
    /// horizontal mouse wheel, and shift+wheel. Nest a `scroll_row` inside a
    /// [`scroll_col`](Self::scroll_col) to scroll both axes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.scroll_row(&mut scroll, |ui| {
    ///     for i in 0..40 {
    ///         ui.text(format!("col-{i:02}  "));
    ///     }
    /// });
    /// # });
    /// ```
    pub fn scroll_row(
        &mut self,
        state: &mut ScrollState,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        self.scrollable(state).grow(1).row(f)
    }

    /// Render a scrollbar track for a [`ScrollState`].
    ///
    /// Displays a track (`│`) with a proportional thumb (`█`). The thumb size
    /// and position are calculated from the scroll state's content height,
    /// viewport height, and current offset.
    ///
    /// Typically placed beside a `scrollable()` container in a `row()`:
    /// ```no_run
    /// # use slt::widgets::ScrollState;
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut scroll = ScrollState::new();
    /// ui.row(|ui| {
    ///     ui.scrollable(&mut scroll).grow(1).col(|ui| {
    ///         for i in 0..100 { ui.text(format!("Line {i}")); }
    ///     });
    ///     ui.scrollbar(&mut scroll);
    /// });
    /// # });
    /// ```
    ///
    /// # Interaction (since 0.21.0)
    ///
    /// The bar is a real input surface, mirroring `split_pane`'s drag handle:
    ///
    /// - **Click-to-jump on the track:** a left mouse-down inside the track but
    ///   outside the thumb jumps `state.offset` so the clicked row maps
    ///   proportionally to the content (top cell → offset 0, bottom cell →
    ///   `max_offset`).
    /// - **Drag-to-scroll on the thumb:** a left mouse-down on the thumb sets
    ///   [`ScrollState::dragging`]; subsequent drag events scroll proportionally
    ///   to the cursor's y within the track (even when the cursor leaves the
    ///   track on the x-axis); mouse-up clears `dragging`.
    ///
    /// Only the mouse events the bar acts on are consumed, so wheel scrolling
    /// over a sibling [`scrollable`](Self::scrollable) keeps working unchanged.
    /// Like every mouse handler the bar is inert while a modal is active and
    /// the bar is not inside it.
    ///
    /// # Returns
    ///
    /// A [`Response`] whose hit-test rect covers the scrollbar track — it is
    /// the track container's own interaction response, so `.clicked`,
    /// `.hovered`, and `.rect` are populated for the track region. `.changed`
    /// is `true` on a frame where a scrollbar interaction moved the offset.
    /// When the content fits the viewport nothing is rendered and
    /// [`Response::none()`] is returned. Prior to v0.21.0 the receiver was
    /// `&ScrollState`; pass `&mut scroll` instead.
    pub fn scrollbar(&mut self, state: &mut ScrollState) -> Response {
        let vh = state.viewport_height();
        let ch = state.content_height();
        if vh == 0 || ch <= vh {
            // No overflow: render nothing, consume nothing, leave drag state
            // untouched. Matches the pre-interaction behavior exactly.
            return Response::none();
        }

        let track_height = vh;
        let thumb_height = ((vh as f64 * vh as f64 / ch as f64).ceil() as u32).max(1);
        let max_offset = ch.saturating_sub(vh);

        // The upcoming `self.container()…col()` allocates the next interaction
        // slot, so its id is the current `interaction_count`. We hit-test
        // against THAT slot's rect from the previous frame, exactly as
        // `scrollable()` and `consume_split_pane_drag` do.
        let track_id = self.rollback.interaction_count;
        let thumb_pos =
            Self::scrollbar_thumb_pos(state.offset, max_offset, track_height, thumb_height);
        let changed = if let Some(rect) = self.prev_hit_map.get(track_id).copied() {
            self.handle_scrollbar_drag(rect, state, thumb_pos, thumb_height, max_offset)
        } else {
            false
        };

        // Recompute the thumb position AFTER handling so the same frame's draw
        // reflects an offset moved by a click/drag this frame.
        let thumb_pos =
            Self::scrollbar_thumb_pos(state.offset, max_offset, track_height, thumb_height);

        let theme = self.theme;
        const THUMB: &str = "█";
        const TRACK: &str = "│";

        // The track container carries its own interaction slot (every
        // `col`/`row` reserves one), so its `Response` is the hit-test rect
        // for click-to-jump — no separate `interaction()` call is needed.
        let mut response = self.container().w(1).h(track_height).col(|ui| {
            for i in 0..track_height {
                if i >= thumb_pos && i < thumb_pos + thumb_height {
                    ui.styled(THUMB, Style::new().fg(theme.primary));
                } else {
                    ui.styled(TRACK, Style::new().fg(theme.text_dim).dim());
                }
            }
        });
        response.changed = changed;
        response
    }

    /// Map a scroll `offset` to the thumb's top row within the track.
    ///
    /// Pure helper shared by the render path and the interaction path so both
    /// agree on where the thumb sits.
    fn scrollbar_thumb_pos(
        offset: usize,
        max_offset: u32,
        track_height: u32,
        thumb_height: u32,
    ) -> u32 {
        if max_offset == 0 {
            0
        } else {
            let travel = track_height.saturating_sub(thumb_height);
            ((offset as f64 / max_offset as f64) * travel as f64).round() as u32
        }
    }

    /// Map a cursor row `y` (absolute) to a clamped scroll offset for the
    /// track rect at `track_y` with height `track_h`.
    ///
    /// The thumb is centered on the cursor: the cursor row relative to the
    /// track maps to the thumb top (minus half the thumb), which then maps
    /// linearly onto `[0, max_offset]`. The result is always in
    /// `[0, max_offset]` and monotonically non-decreasing in `y`. Extracted
    /// as an associated function so it is `proptest`-able without driving a
    /// full frame.
    pub(crate) fn scrollbar_offset_for_y(
        y: u32,
        track_y: u32,
        track_h: u32,
        thumb_height: u32,
        max_offset: u32,
    ) -> usize {
        let travel = track_h.saturating_sub(thumb_height);
        if travel == 0 {
            return 0;
        }
        let rel = y.saturating_sub(track_y).min(track_h.saturating_sub(1));
        let thumb_top = rel.saturating_sub(thumb_height / 2).min(travel);
        ((thumb_top as f64 / travel as f64) * max_offset as f64).round() as usize
    }

    /// Hit-test the previous-frame track `rect` against this frame's mouse
    /// events and apply click-to-jump / thumb-drag to `state`.
    ///
    /// Returns `true` if the offset moved. Mirrors `consume_split_pane_drag`:
    /// snapshots the unconsumed mouse events, mutates `state`, then consumes
    /// only the events it acted on so wheel scroll on a sibling container is
    /// never double-counted.
    fn handle_scrollbar_drag(
        &mut self,
        rect: Rect,
        state: &mut ScrollState,
        thumb_pos: u32,
        thumb_height: u32,
        max_offset: u32,
    ) -> bool {
        // Modal suppression: while a modal is active and the bar is not inside
        // an overlay, the bar is inert — consistent with `mouse_down`'s guard.
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return false;
        }
        if rect.width == 0 || rect.height == 0 {
            return false;
        }

        // Snapshot so `consume_indices` (mutable borrow) can run after the loop.
        // `MouseKind` is not `Copy`, so clone it (mirrors `consume_split_pane_drag`).
        let events: Vec<(usize, MouseKind, u32, u32)> = self
            .events
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Event::Mouse(m) if !self.consumed[i] => Some((i, m.kind.clone(), m.x, m.y)),
                _ => None,
            })
            .collect();

        let track_y = rect.y;
        let track_h = rect.height;
        let thumb_top = track_y + thumb_pos;
        let thumb_bottom = thumb_top + thumb_height;

        let mut consumed: Vec<usize> = Vec::new();
        let mut changed = false;
        for (i, kind, mx, my) in events {
            let in_track = mx >= rect.x && mx < rect.right() && my >= track_y && my < rect.bottom();
            match kind {
                MouseKind::Down(MouseButton::Left) if in_track => {
                    let on_thumb = my >= thumb_top && my < thumb_bottom;
                    if on_thumb {
                        // Grab the thumb; offset only moves on subsequent drags.
                        state.dragging = true;
                    } else {
                        // Click-to-jump on the track.
                        let before = state.offset;
                        state.set_offset(Self::scrollbar_offset_for_y(
                            my,
                            track_y,
                            track_h,
                            thumb_height,
                            max_offset,
                        ));
                        changed |= state.offset != before;
                    }
                    consumed.push(i);
                }
                MouseKind::Drag(MouseButton::Left) if state.dragging => {
                    // Drag tracks the cursor's y even outside the track on x.
                    let before = state.offset;
                    state.set_offset(Self::scrollbar_offset_for_y(
                        my,
                        track_y,
                        track_h,
                        thumb_height,
                        max_offset,
                    ));
                    changed |= state.offset != before;
                    consumed.push(i);
                }
                MouseKind::Up(MouseButton::Left) if state.dragging => {
                    state.dragging = false;
                    consumed.push(i);
                }
                _ => {}
            }
        }
        self.consume_indices(consumed);
        changed
    }

    fn auto_scroll_nested(
        &mut self,
        rect: &Rect,
        state: &mut ScrollState,
        inner_scroll_rects: &[Rect],
        is_horizontal: bool,
    ) {
        let mut to_consume = Vec::new();
        let shift = crate::event::KeyModifiers::SHIFT;
        for (i, mouse) in self.mouse_events_in_rect(*rect) {
            let in_inner = inner_scroll_rects.iter().any(|sr| {
                mouse.x >= sr.x && mouse.x < sr.right() && mouse.y >= sr.y && mouse.y < sr.bottom()
            });
            if in_inner {
                continue;
            }

            let delta = self.scroll_lines_per_event as usize;
            if is_horizontal {
                // #247: a horizontal scrollable consumes native horizontal wheel
                // events (`ScrollLeft` / `ScrollRight`) and shift+vertical-wheel
                // (the common terminal convention for sideways scroll on a
                // mouse with only a vertical wheel).
                let shifted = mouse.modifiers.contains(shift);
                match mouse.kind {
                    MouseKind::ScrollLeft => {
                        state.scroll_left(delta);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollRight => {
                        state.scroll_right(delta);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollUp if shifted => {
                        state.scroll_left(delta);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollDown if shifted => {
                        state.scroll_right(delta);
                        to_consume.push(i);
                    }
                    _ => {}
                }
            } else {
                match mouse.kind {
                    MouseKind::ScrollUp => {
                        state.scroll_up(delta);
                        to_consume.push(i);
                    }
                    MouseKind::ScrollDown => {
                        state.scroll_down(delta);
                        to_consume.push(i);
                    }
                    MouseKind::Drag(MouseButton::Left) => {}
                    _ => {}
                }
            }
        }
        self.consume_indices(to_consume);
    }

    /// Shortcut for `container().border(border)`.
    ///
    /// Returns a [`ContainerBuilder`] pre-configured with the given border style.
    pub fn bordered(&mut self, border: Border) -> ContainerBuilder<'_> {
        self.container()
            .border(border)
            .border_sides(BorderSides::all())
    }

    fn push_container(
        &mut self,
        direction: Direction,
        gap: u32,
        f: impl FnOnce(&mut Context),
    ) -> Response {
        let interaction_id = self.next_interaction_id();
        let border = self.theme.border;

        self.commands
            .push(Command::BeginContainer(Box::new(BeginContainerArgs {
                direction,
                // `BeginContainerArgs::gap` is signed since #222; this helper's
                // public `u32` callers (`row`/`col_gap`/…) never overlap.
                gap: gap as i32,
                align: Align::Start,
                align_self: None,
                justify: Justify::Start,
                border: None,
                border_sides: BorderSides::all(),
                border_style: Style::new().fg(border),
                bg_color: None,
                padding: Padding::default(),
                margin: Margin::default(),
                constraints: Constraints::default(),
                title: None,
                grow: 0,
                group_name: None,
            })));
        self.rollback.text_color_stack.push(None);
        f(self);
        self.rollback.text_color_stack.pop();
        self.commands.push(Command::EndContainer);
        self.rollback.last_text_idx = None;

        self.response_for(interaction_id)
    }

    pub(crate) fn response_for(&self, interaction_id: usize) -> Response {
        if (self.rollback.modal_active || self.prev_modal_active)
            && self.rollback.overlay_depth == 0
        {
            return Response::none();
        }
        if let Some(rect) = self.prev_hit_map.get(interaction_id) {
            let clicked = self
                .click_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            // Issue #208: right-click hit-test uses the same rect as the
            // existing left-click logic. Keeps modal suppression (the early
            // return above) consistent for both buttons.
            let right_clicked = self
                .right_click_pos
                .map(|(mx, my)| {
                    mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom()
                })
                .unwrap_or(false);
            // v0.21.1: double-click hit-test mirrors the left-click logic. The
            // second click of a double also reports `clicked`, so callers that
            // only check `clicked` are unaffected.
            let double_clicked = self
                .double_click_pos
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
            // v0.21.1: per-widget wheel delta is hover-gated — only the widget
            // under the cursor when the wheel moved sees a non-zero delta.
            let scroll_delta = self
                .scroll_pos
                .map(|(mx, my)| {
                    if mx >= rect.x && mx < rect.right() && my >= rect.y && my < rect.bottom() {
                        self.scroll_delta_frame
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            Response {
                clicked,
                right_clicked,
                double_clicked,
                hovered,
                changed: false,
                focused: false,
                gained_focus: false,
                lost_focus: false,
                submitted: false,
                scroll_delta,
                rect: *rect,
            }
        } else {
            Response::none()
        }
    }

    /// Returns true if the named group is currently hovered by the mouse.
    ///
    /// Uses the per-frame `hovered_groups` `HashSet` populated by
    /// `Context::build_hovered_groups()`; turns the previous O(n) scan over
    /// `prev_group_rects` into an O(1) lookup. Closes the cache half of
    /// #136 / #139.
    pub fn is_group_hovered(&self, name: &str) -> bool {
        if self.mouse_pos.is_none() {
            return false;
        }
        // `HashSet<Arc<str>>::contains` accepts `&str` via `Borrow<str>`, so
        // there is no allocation on the hot path.
        self.hovered_groups.contains(name)
    }

    /// Returns true if the named group contains the currently focused widget.
    pub fn is_group_focused(&self, name: &str) -> bool {
        if self.prev_focus_count == 0 {
            return false;
        }
        let focused_index = self.focus_index % self.prev_focus_count;
        self.prev_focus_groups
            .get(focused_index)
            .and_then(|group| group.as_deref())
            .map(|group| group == name)
            .unwrap_or(false)
    }

    /// Render a form that groups input fields vertically.
    ///
    /// Wraps the fields in a column container and forwards the form state
    /// to the closure. Use [`Context::form_field`] inside the closure to
    /// render each field with label + input + error display.
    ///
    /// Submission is driven by [`Context::form_submit`]. Per-field validators
    /// attached via [`FormField::validate`](crate::widgets::FormField::validate)
    /// run automatically inside [`Context::form_field`]; aggregate validity is
    /// read via [`FormState::is_valid`](crate::widgets::FormState::is_valid).
    pub fn form(
        &mut self,
        state: &mut FormState,
        f: impl FnOnce(&mut Context, &mut FormState),
    ) -> &mut Self {
        let _ = self.col(|ui| {
            f(ui, state);
        });
        self
    }

    /// Render a single form field with label and input, running its validators.
    ///
    /// The field's own validators (attached via
    /// [`FormField::validate`](crate::widgets::FormField::validate)) run
    /// automatically according to its
    /// [`trigger`](crate::widgets::FormField::trigger):
    /// [`OnChange`](crate::widgets::ValidateTrigger::OnChange) re-validates on
    /// each keystroke, [`OnBlur`](crate::widgets::ValidateTrigger::OnBlur)
    /// (the default) re-validates when focus leaves the field, and
    /// [`Manual`](crate::widgets::ValidateTrigger::Manual) never auto-validates.
    /// The resulting [`error`](crate::widgets::FormField::error) is shown below
    /// the input.
    ///
    /// With the `async` feature, any in-flight
    /// [`validate_async`](crate::widgets::FormField::validate_async) check is
    /// polled each frame and its result surfaced as the field error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use slt::widgets::{FormField, validators};
    /// # slt::run(|ui: &mut slt::Context| {
    /// let mut field = FormField::new("Email")
    ///     .validate(validators::email()); // OnBlur by default
    /// ui.form_field(&mut field);
    /// # });
    /// ```
    pub fn form_field(&mut self, field: &mut FormField) -> &mut Self {
        #[cfg(feature = "async")]
        let async_resolved = field.poll_async();
        let mut resp = Response::none();
        let _ = self.col(|ui| {
            ui.styled(field.label.as_str(), Style::new().bold().fg(ui.theme.text));
            resp = ui.text_input(&mut field.input);
            if let Some(error) = field.error.as_deref() {
                ui.styled(error, Style::new().dim().fg(ui.theme.error));
            }
        });
        #[cfg(feature = "async")]
        let _ = async_resolved;
        // `text_input` reports `.focused` reliably but does not yet populate
        // `.lost_focus` on its container-assembled response, so blur is derived
        // from the focus edge tracked on the field itself.
        let lost_focus = field.observe_focus(resp.focused);
        match field.trigger {
            ValidateTrigger::OnChange if resp.changed => {
                field.run_validators();
            }
            ValidateTrigger::OnBlur if lost_focus => {
                field.run_validators();
            }
            _ => {}
        }
        self
    }

    /// Render a primary-styled submit button.
    ///
    /// Distinguishes the submit affordance from incidental buttons in the
    /// same form by rendering in the theme's primary color (via
    /// [`ButtonVariant::Primary`]). Returns `true` in `.clicked` when the
    /// user clicks it, presses Enter while focused, or activates it with
    /// Space. Pair with
    /// [`FormState::validate_all`](crate::widgets::FormState::validate_all) /
    /// [`FormState::is_valid`](crate::widgets::FormState::is_valid) to gate
    /// submission on all fields being valid.
    pub fn form_submit(&mut self, label: impl Into<String>) -> Response {
        self.button_with(label, ButtonVariant::Primary)
    }
}

#[cfg(test)]
mod scrollbar_tests {
    use super::*;

    // ── #249: scrollbar() pixel ↔ offset mapping (pure helpers) ──────────

    #[test]
    fn offset_for_y_top_cell_maps_to_zero() {
        // Track at y=0..20, thumb 4 tall → travel 16, max_offset 80.
        let off = Context::scrollbar_offset_for_y(0, 0, 20, 4, 80);
        assert_eq!(off, 0);
    }

    #[test]
    fn offset_for_y_bottom_cell_maps_to_max() {
        // Clicking the last track cell jumps to the bottom of the content.
        let off = Context::scrollbar_offset_for_y(19, 0, 20, 4, 80);
        assert_eq!(off, 80);
    }

    #[test]
    fn offset_for_y_middle_is_near_half_max() {
        // Vertical midpoint → ~max_offset / 2 (within a few rows of slop).
        let off = Context::scrollbar_offset_for_y(10, 0, 20, 4, 80) as i64;
        assert!((off - 40).abs() <= 5, "midpoint offset {off} not near 40");
    }

    #[test]
    fn offset_for_y_respects_track_origin() {
        // Track offset by track_y=3; the top cell of that track yields 0.
        let off = Context::scrollbar_offset_for_y(3, 3, 20, 4, 80);
        assert_eq!(off, 0);
    }

    #[test]
    fn offset_for_y_zero_travel_is_zero() {
        // Thumb fills the whole track → nowhere to move → always 0.
        let off = Context::scrollbar_offset_for_y(7, 0, 5, 5, 0);
        assert_eq!(off, 0);
    }

    #[test]
    fn thumb_pos_endpoints() {
        // offset 0 → thumb at top; offset == max → thumb at travel.
        assert_eq!(Context::scrollbar_thumb_pos(0, 80, 20, 4), 0);
        assert_eq!(Context::scrollbar_thumb_pos(80, 80, 20, 4), 16);
    }

    proptest::proptest! {
        /// `scrollbar_offset_for_y` is always in `[0, max_offset]` and
        /// monotonically non-decreasing in the cursor row.
        #[test]
        fn offset_for_y_is_clamped_and_monotonic(
            content_height in 2u32..500,
            viewport_height in 1u32..200,
            y in 0u32..600,
        ) {
            // Derive the same track / thumb geometry the widget uses.
            proptest::prop_assume!(content_height > viewport_height);
            let track_h = viewport_height;
            let thumb_height = ((viewport_height as f64 * viewport_height as f64
                / content_height as f64)
                .ceil() as u32)
                .max(1);
            let max_offset = content_height.saturating_sub(viewport_height);

            let off = Context::scrollbar_offset_for_y(y, 0, track_h, thumb_height, max_offset);
            proptest::prop_assert!(off <= max_offset as usize);

            // Monotonic: a strictly lower cursor row never yields a smaller offset.
            let off_lower = Context::scrollbar_offset_for_y(
                y.saturating_add(1),
                0,
                track_h,
                thumb_height,
                max_offset,
            );
            proptest::prop_assert!(off_lower >= off);
        }
    }
}
