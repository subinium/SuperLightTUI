use super::*;

/// Internal discriminator for [`State<T>`] handles.
///
/// `Indexed` refers to a slot in `Context::hook_states` (positional, used by
/// [`Context::use_state`] / [`Context::use_memo`]). `Named` and `Keyed` both
/// refer to a key in `Context::keyed_states`: `Named` borrows the
/// `&'static str` id of the deprecated `use_state_named` so it never
/// allocates, `Keyed` owns the runtime string of [`Context::use_state_keyed`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StateKey {
    Indexed(usize),
    Named(&'static str),
    Keyed(String),
}

/// Handle to state created by `use_state()`. Access via `.get(ui)` / `.get_mut(ui)`.
///
/// # Note on `Copy`
///
/// As of v0.20.0, `State<T>` is no longer `Copy`. The internal key may hold an
/// owned `String` (for [`Context::use_state_keyed`]), which prevents trivial
/// duplication. Existing call sites that use the handle locally (`let s =
/// ui.use_state(...); s.get(ui);`) are unaffected — the handle is moved into
/// closures or borrowed by reference. If you previously relied on implicit
/// copy semantics, call `.clone()` explicitly.
pub struct State<T> {
    key: StateKey,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for State<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl<T> PartialEq for State<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for State<T> {}

/// Downcast a stored boxed `Any` to `&T`, panicking with a uniform context
/// message on mismatch. Internal helper to keep [`State::get`] / [`State::get_mut`]
/// concise and ensure every panic site formats identically.
///
/// `ctx` should be a complete leading clause such as
/// `"use_state_named type mismatch for id \"foo\""` — the helper appends
/// `" — expected <type>"` so callers don't repeat that suffix at every site.
fn downcast_or_panic<'a, T: 'static>(
    boxed: &'a dyn std::any::Any,
    ctx: std::fmt::Arguments<'_>,
) -> &'a T {
    boxed
        .downcast_ref::<T>()
        .unwrap_or_else(|| panic!("{ctx} — expected {}", std::any::type_name::<T>()))
}

/// Mutable counterpart of [`downcast_or_panic`].
fn downcast_or_panic_mut<'a, T: 'static>(
    boxed: &'a mut dyn std::any::Any,
    ctx: std::fmt::Arguments<'_>,
) -> &'a mut T {
    boxed
        .downcast_mut::<T>()
        .unwrap_or_else(|| panic!("{ctx} — expected {}", std::any::type_name::<T>()))
}

impl<T: 'static> State<T> {
    pub(crate) fn from_idx(idx: usize) -> Self {
        Self {
            key: StateKey::Indexed(idx),
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn from_named(id: &'static str) -> Self {
        Self {
            key: StateKey::Named(id),
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn from_keyed(id: String) -> Self {
        Self {
            key: StateKey::Keyed(id),
            _marker: std::marker::PhantomData,
        }
    }

    /// Read the current value.
    ///
    /// # Panics
    ///
    /// Panics if this handle's slot or id contains a different concrete type,
    /// which means positional hooks changed order or the same named/keyed id
    /// was reused with another `T`. Named and keyed handles also panic if their
    /// backing entry was removed before access. The message identifies the
    /// hook index or id and the expected type.
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        match &self.key {
            StateKey::Indexed(idx) => downcast_or_panic::<T>(
                ui.hook_states[*idx].as_ref(),
                format_args!("use_state type mismatch at hook index {idx}"),
            ),
            StateKey::Named(id) => {
                let boxed = ui.keyed_states.get(*id).unwrap_or_else(|| {
                    panic!("use_state_named: no entry for id {id:?} — was use_state_named called?")
                });
                downcast_or_panic::<T>(
                    boxed.as_ref(),
                    format_args!("use_state_named type mismatch for id {id:?}"),
                )
            }
            StateKey::Keyed(id) => {
                let boxed = ui.keyed_states.get(id).unwrap_or_else(|| {
                    panic!("use_state_keyed: no entry for id {id:?} — was use_state_keyed called?")
                });
                downcast_or_panic::<T>(
                    boxed.as_ref(),
                    format_args!("use_state_keyed type mismatch for id {id:?}"),
                )
            }
        }
    }

    /// Mutably access the current value.
    ///
    /// # Panics
    ///
    /// Panics under the same slot, id, and type-mismatch conditions as
    /// [`get`](Self::get).
    pub fn get_mut<'a>(&self, ui: &'a mut Context) -> &'a mut T {
        match &self.key {
            StateKey::Indexed(idx) => downcast_or_panic_mut::<T>(
                ui.hook_states[*idx].as_mut(),
                format_args!("use_state type mismatch at hook index {idx}"),
            ),
            StateKey::Named(id) => {
                let boxed = ui.keyed_states.get_mut(*id).unwrap_or_else(|| {
                    panic!("use_state_named: no entry for id {id:?} — was use_state_named called?")
                });
                downcast_or_panic_mut::<T>(
                    boxed.as_mut(),
                    format_args!("use_state_named type mismatch for id {id:?}"),
                )
            }
            StateKey::Keyed(id) => {
                let boxed = ui.keyed_states.get_mut(id).unwrap_or_else(|| {
                    panic!("use_state_keyed: no entry for id {id:?} — was use_state_keyed called?")
                });
                downcast_or_panic_mut::<T>(
                    boxed.as_mut(),
                    format_args!("use_state_keyed type mismatch for id {id:?}"),
                )
            }
        }
    }
}

/// Internal storage shape for a value created by [`Context::use_memo`].
///
/// The previous-frame dependencies are kept type-erased (`Box<dyn Any>`) so the
/// read path ([`Memo::get`]) can downcast the slot to `MemoSlot<T>` without
/// knowing `D`. [`Context::use_memo`] downcasts `deps` back to `&D` when
/// comparing against the new dependencies to decide whether to recompute.
///
/// Kept `pub(crate)` — never part of the public API. The `T` in its type name
/// appears in the hook-ordering mismatch panic message, mirroring the historic
/// `(D, T)` message shape.
pub(crate) struct MemoSlot<T> {
    pub(crate) deps: Box<dyn std::any::Any>,
    pub(crate) value: T,
}

/// Handle to a memoized value created by [`Context::use_memo`].
///
/// Like [`State<T>`], this is an *index handle*, not a live borrow — it stores
/// only the hook slot index and does **not** keep [`Context`] borrowed. That is
/// the whole point: the handle composes with later `ui.*` calls, where the old
/// `&T`-returning form (`use_memo_ref`, removed in 0.26) held an immutable borrow
/// of `ui` that conflicted with any subsequent mutation.
///
/// Read the value with [`get`](Self::get) (`&T`) or [`copied`](Self::copied)
/// (`T: Copy`).
///
/// # Example
///
/// ```no_run
/// # slt::run(|ui: &mut slt::Context| {
/// let count = ui.use_state(|| 0i32);
/// let count_val = *count.get(ui);
/// // Handle releases the `&mut ui` borrow immediately...
/// let doubled = ui.use_memo(&count_val, |c| c * 2);
/// // ...so an intervening `ui.*` call composes cleanly.
/// ui.text("computed:");
/// ui.text(format!("{}", doubled.copied(ui)));
/// # });
/// ```
pub struct Memo<T> {
    idx: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memo")
            .field("idx", &self.idx)
            .finish_non_exhaustive()
    }
}

impl<T> PartialEq for Memo<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Eq for Memo<T> {}

impl<T: 'static> Memo<T> {
    pub(crate) fn from_idx(idx: usize) -> Self {
        Self {
            idx,
            _marker: std::marker::PhantomData,
        }
    }

    /// Read the memoized value.
    ///
    /// # Panics
    ///
    /// Panics with the slot index and expected type name if the hook at this
    /// index does not hold a `MemoSlot<T>` — i.e. the rules-of-hooks contract
    /// was broken (hooks called in a different order than the frame that created
    /// the slot). The message matches [`Context::use_memo`]'s own mismatch
    /// panic.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let m = ui.use_memo(&3i32, |d| d * 2);
    /// ui.text(format!("{}", m.get(ui)));
    /// # });
    /// ```
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        match ui.hook_states[self.idx].downcast_ref::<MemoSlot<T>>() {
            Some(slot) => &slot.value,
            None => panic!(
                "Hook type mismatch at index {}: expected {}. Hooks must be called in the same order every frame.",
                self.idx,
                std::any::type_name::<MemoSlot<T>>()
            ),
        }
    }

    /// Read a `Copy` of the memoized value.
    ///
    /// Convenience for `*memo.get(ui)`. Panics under the same conditions as
    /// [`get`](Self::get).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # slt::run(|ui: &mut slt::Context| {
    /// let doubled = ui.use_memo(&21i32, |d| d * 2).copied(ui);
    /// ui.text(format!("{doubled}"));
    /// # });
    /// ```
    pub fn copied(&self, ui: &Context) -> T
    where
        T: Copy,
    {
        *self.get(ui)
    }
}

/// Interaction response returned by all widgets.
///
/// Container methods return a [`Response`]. Check `.clicked`, `.changed`, etc.
/// to react to user interactions.
/// `rect` is meaningful after the widget has participated in layout.
/// Container responses describe the container's own interaction area, not
/// automatically the focus state of every child widget.
///
/// # Ignoring a response
///
/// Since v0.26 `Response` is not `#[must_use]` as a type: layout and display
/// calls (`ui.col(..)`, `ui.row(..)`, `ui.separator()`, `ui.badge(..)`, ...)
/// are plain statements, with no `let _ =`. The warning is kept only where
/// dropping the result is almost always a bug: the stateless clickable
/// widget [`button`](Context::button) (its [`Button`](crate::Button) builder
/// type is `#[must_use]`), whose `clicked` flag is its only output.
///
/// ```
/// #![deny(unused_must_use)]
/// # use slt::*;
/// # TestBackend::new(80, 24).render(|ui| {
/// ui.col(|ui| {
///     ui.text("Name");
/// });
/// ui.row(|ui| {
///     ui.text("a");
/// });
/// ui.separator();
/// ui.badge("new");
/// ui.button("Save").on_click(ui, |_| {});
/// # });
/// ```
///
/// A bare `ui.button(..)` statement still warns:
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// # use slt::*;
/// # TestBackend::new(80, 24).render(|ui| {
/// ui.button("Save");
/// # });
/// ```
///
/// # Examples
///
/// ```
/// # use slt::*;
/// # TestBackend::new(80, 24).render(|ui| {
/// let r = ui.row(|ui| {
///     ui.text("Save");
/// });
/// if r.clicked {
///     // handle save
/// }
/// # });
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Response {
    /// Whether the widget was left-clicked (or activated) this frame.
    ///
    /// A left press activates only the **topmost, innermost** interactive
    /// slot under the pointer (since v0.26): a button in an overlay does not
    /// also click the container drawn behind it, and a clickable row inside a
    /// clickable card sets `clicked` on the row only. An ancestor opts back in
    /// with [`ContainerBuilder::bubble_clicks`]; [`clicked_any`](Self::clicked_any)
    /// keeps the raw "press landed inside my rect" meaning. No press is
    /// dropped: when several arrive in one input batch they are dispatched one
    /// per frame, like sequential DOM events, so each is hit-tested against
    /// the layout the previous press produced.
    ///
    /// `clicked` fires on the mouse **press** by default. Widgets that act on
    /// a press (selection, toggles) feel immediate this way, and it matches
    /// SLT's pre-0.26 behavior. With
    /// [`RunConfig::click_on_release(true)`](crate::RunConfig::click_on_release)
    /// it fires instead on the frame the button is released inside the same
    /// slot that was pressed, so dragging off the widget before releasing
    /// cancels the click (the browser `click` event).
    ///
    /// Keyboard-activatable widgets such as [`button`](Context::button) also
    /// set `clicked` on Enter / Space while focused, regardless of the mouse.
    pub clicked: bool,
    /// Whether a left press landed anywhere inside the widget's rect this
    /// frame, even when a nested or overlaying slot won the click.
    ///
    /// This is the pre-0.26 meaning of `clicked`. Use it for "click inside
    /// this region" logic that must not depend on which descendant was hit.
    /// Available since v0.26.0.
    pub clicked_any: bool,
    /// Whether the widget was right-clicked this frame.
    ///
    /// Detected when a `MouseButton::Right` `Down` event resolves to this
    /// widget under the same topmost/innermost rule as
    /// [`clicked`](Self::clicked). Suppressed for non-overlay widgets while a
    /// modal is active. Available since v0.20.0.
    pub right_clicked: bool,
    /// Whether the pointer is over the widget's rect.
    ///
    /// Containment only: an outer card and the button inside it are both
    /// `hovered` while the pointer is over the button, which is what hover
    /// styling usually wants. Use [`hovered_top`](Self::hovered_top) for the
    /// single slot that would receive a click there. Needs pointer motion
    /// reporting ([`MouseMode::Hover`](crate::MouseMode::Hover)) to track
    /// moves without a button held.
    pub hovered: bool,
    /// Whether the widget is the topmost, innermost slot under the pointer:
    /// the one a press at the current position would activate.
    ///
    /// Available since v0.26.0.
    pub hovered_top: bool,
    /// Whether a left press landed on this widget (as the topmost, innermost
    /// target) this frame. Unlike `clicked`, this never waits for the release
    /// and never comes from the keyboard. The press also *captures* the
    /// pointer for this widget until the release. Capture follows the
    /// widget's stable identity when it has one (a keyed or named focusable,
    /// a `text_input` / `textarea`, or a [`group`](crate::Context::group)
    /// container), so UI that appears before it mid-drag does not move the
    /// capture; an anonymous container is captured by its slot position. For a custom drag, mark the
    /// container [`draggable`](crate::ContainerBuilder::draggable) so the
    /// gesture does not also start the built-in text selection. Available
    /// since v0.26.0.
    pub pressed: bool,
    /// Whether the button that pressed this widget was released this frame,
    /// wherever the pointer is. Fires once per gesture; not set when the
    /// gesture is cancelled by terminal focus loss or a resize. Available
    /// since v0.26.0.
    pub released: bool,
    /// Whether the pointer started moving with the button held after
    /// pressing this widget, on this frame. Available since v0.26.0.
    pub drag_started: bool,
    /// Whether this widget holds the pointer capture and the pointer has moved
    /// since the press (a drag in progress at the end of this frame). Cleared
    /// on release, terminal focus loss, resize, and when motion arrives
    /// without a held button (only reported in
    /// [`MouseMode::Hover`](crate::MouseMode::Hover)). Available since v0.26.0.
    pub dragging: bool,
    /// Pointer motion in cells `(dx, dy)` this frame while this widget holds
    /// the capture. Sum the deltas across frames for the total drag distance.
    /// Drag motion needs [`MouseMode::Drag`](crate::MouseMode::Drag) (the
    /// local default) or [`MouseMode::Hover`](crate::MouseMode::Hover).
    /// Available since v0.26.0.
    pub drag_delta: (i32, i32),
    /// Whether the middle button was pressed on this widget this frame
    /// (topmost, innermost target). Available since v0.26.0.
    pub middle_clicked: bool,
    /// Modifier keys held during the press that targeted this widget this
    /// frame; [`KeyModifiers::NONE`] otherwise. Available since v0.26.0.
    pub modifiers: KeyModifiers,
    /// Whether the widget's value changed this frame.
    pub changed: bool,
    /// Whether the widget currently has keyboard focus.
    pub focused: bool,
    /// Whether the widget *just* received keyboard focus this frame.
    ///
    /// `true` only on the first frame after focus moved to this widget;
    /// `false` thereafter (until focus moves away and returns). Mutually
    /// exclusive with [`lost_focus`](Self::lost_focus). Available since
    /// v0.20.0.
    pub gained_focus: bool,
    /// Whether the widget *just* lost keyboard focus this frame.
    ///
    /// `true` only on the first frame after focus moved away from this widget;
    /// `false` on subsequent frames. Mutually exclusive with
    /// [`gained_focus`](Self::gained_focus). Available since v0.20.0.
    pub lost_focus: bool,
    /// Whether the widget was double-clicked this frame.
    ///
    /// Detected when two `MouseButton::Left` `Down` events land on the same
    /// terminal cell within the double-click window (~400ms). When `true`,
    /// `clicked` is also `true` for the same frame (the second click is still a
    /// click). This is the standard open/activate gesture for file pickers,
    /// lists, tables, and trees. Follows the same topmost/innermost winner rule
    /// as `clicked` and is suppressed for non-overlay widgets while a modal is
    /// active. Available since v0.21.1.
    pub double_clicked: bool,
    /// Whether the widget submitted its value this frame.
    ///
    /// Set by widgets that have an explicit submit gesture — e.g. pressing
    /// `Enter` in a focused single-line [`text_input`](Context::text_input).
    /// Always `false` for widgets with no submit semantics. Available since
    /// v0.21.1.
    pub submitted: bool,
    /// Net vertical scroll-wheel delta over this widget this frame.
    ///
    /// Positive = wheel scrolled up, negative = down, `0` when the wheel did
    /// not move while the cursor was over the widget's `rect`. Hover-gated, so
    /// each widget consumes only the wheel motion that occurred above it — a
    /// chart, canvas, or custom viewport can scroll/zoom locally without a
    /// frame-global scroll handler. Available since v0.21.1.
    pub scroll_delta: i32,
    /// The rectangle the widget occupies after layout.
    pub rect: Rect,
    /// Whether the widget rendered inside [`Context::disabled`]`(true, ..)`.
    /// A disabled widget reports no clicks, hover, focus or changes; only
    /// `rect` is filled in. Available since v0.26.0.
    pub disabled: bool,
}

impl Response {
    /// Create a Response with all fields false/default.
    pub fn none() -> Self {
        Self::default()
    }

    /// Attach a tooltip to this widget. Renders only when the widget is
    /// currently hovered and the pointer has rested for
    /// [`RunConfig::tooltip_delay`](crate::RunConfig::tooltip_delay).
    ///
    /// Equivalent to calling [`Context::tooltip`] immediately after the
    /// widget, but composes cleanly with the chained `Response` style:
    ///
    /// ```ignore
    /// if ui.button("Save").on_hover(ui, "Saves the file").clicked {
    ///     save();
    /// }
    /// ```
    ///
    /// `text` is wrapped at 38 columns and rendered in an overlay panel
    /// anchored under (or above, if no room below) the widget's rect.
    /// Empty strings, zero-area rects, and non-hovered responses are
    /// silently skipped — no allocation in the cold path.
    ///
    /// Unlike [`Context::tooltip`], the binding is not order-sensitive:
    /// the tooltip is attached to *this* response specifically, so
    /// chaining further widgets afterward does not strip it.
    pub fn on_hover(self, ctx: &mut Context, text: impl Into<String>) -> Self {
        if !self.hovered || self.rect.width == 0 || self.rect.height == 0 || !ctx.tooltip_ready() {
            return self;
        }
        let tooltip_text = text.into();
        if tooltip_text.is_empty() {
            return self;
        }
        let lines = super::widgets::wrap_tooltip_text(&tooltip_text, 38);
        ctx.pending_tooltips.push(PendingTooltip {
            anchor_rect: self.rect,
            lines,
        });
        self
    }

    /// Run a closure to render arbitrary tooltip content when the widget is
    /// hovered.
    ///
    /// The closure receives the same `&mut Context` and runs immediately
    /// (in-place — not deferred). This means the closure can issue any UI
    /// commands; positioning is the caller's responsibility (use
    /// [`Context::overlay`] / [`Context::overlay_at`] inside the closure
    /// for floating panels).
    ///
    /// For simple text tooltips, prefer [`Response::on_hover`] which
    /// auto-positions the tooltip under the widget.
    ///
    /// ```ignore
    /// ui.button("Help").on_hover_ui(ui, |ui| {
    ///     let _ = ui.overlay(|ui| {
    ///         ui.text("Custom tooltip body");
    ///     });
    /// });
    /// ```
    pub fn on_hover_ui(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.hovered && self.rect.width > 0 && self.rect.height > 0 {
            f(ctx);
        }
        self
    }

    /// Run `f` if the widget was clicked this frame, then return the Response
    /// for further chaining.
    ///
    /// The closure receives the same `&mut Context` so it can issue UI commands
    /// (e.g. queue a toast); ignore the argument with `|_|` if you only need to
    /// mutate application state.
    ///
    /// ```ignore
    /// ui.button("Save").on_click(ui, |_| save());
    /// ```
    pub fn on_click(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.clicked {
            f(ctx);
        }
        self
    }

    /// Run `f` if the widget's value changed this frame, then return the
    /// Response for chaining. See [`on_click`](Self::on_click) for the closure
    /// argument convention. Available since v0.21.1.
    pub fn on_changed(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.changed {
            f(ctx);
        }
        self
    }

    /// Run `f` on the frame the widget *gained* keyboard focus, then return the
    /// Response for chaining. Fires once per focus acquisition (mirrors
    /// [`gained_focus`](Self::gained_focus)). Available since v0.21.1.
    pub fn on_focus(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.gained_focus {
            f(ctx);
        }
        self
    }

    /// Run `f` if the widget submitted this frame (e.g. `Enter` in a focused
    /// single-line text input), then return the Response for chaining. Mirrors
    /// [`submitted`](Self::submitted). Available since v0.21.1.
    pub fn on_submit(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.submitted {
            f(ctx);
        }
        self
    }

    /// Run `f` if the widget was double-clicked this frame, then return the
    /// Response for chaining. Mirrors [`double_clicked`](Self::double_clicked).
    /// Available since v0.21.1.
    pub fn on_double_click(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.double_clicked {
            f(ctx);
        }
        self
    }
}
