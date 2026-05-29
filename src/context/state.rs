use super::*;

/// Internal discriminator for [`State<T>`] handles.
///
/// `Indexed` refers to a slot in `Context::hook_states` (positional, used by
/// [`Context::use_state`] / [`Context::use_memo`]). `Named` refers to a key in
/// `Context::named_states` (used by [`Context::use_state_named`]). `Keyed`
/// refers to a runtime-string key in `Context::keyed_states` (used by
/// [`Context::use_state_keyed`]).
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State<T> {
    key: StateKey,
    _marker: std::marker::PhantomData<T>,
}

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
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        match &self.key {
            StateKey::Indexed(idx) => downcast_or_panic::<T>(
                ui.hook_states[*idx].as_ref(),
                format_args!("use_state type mismatch at hook index {idx}"),
            ),
            StateKey::Named(id) => {
                let boxed = ui.named_states.get(id).unwrap_or_else(|| {
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
    pub fn get_mut<'a>(&self, ui: &'a mut Context) -> &'a mut T {
        match &self.key {
            StateKey::Indexed(idx) => downcast_or_panic_mut::<T>(
                ui.hook_states[*idx].as_mut(),
                format_args!("use_state type mismatch at hook index {idx}"),
            ),
            StateKey::Named(id) => {
                let boxed = ui.named_states.get_mut(id).unwrap_or_else(|| {
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
/// `&T`-returning form (now [`Context::use_memo_ref`]) held an immutable borrow
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memo<T> {
    idx: usize,
    _marker: std::marker::PhantomData<T>,
}

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
#[derive(Debug, Clone, Default)]
#[must_use = "Response contains interaction state — check .clicked, .hovered, or .changed"]
pub struct Response {
    /// Whether the widget was left-clicked this frame.
    pub clicked: bool,
    /// Whether the widget was right-clicked this frame.
    ///
    /// Detected when a `MouseButton::Right` `Down` event lands inside the
    /// widget's `rect`. Suppressed for non-overlay widgets while a modal is
    /// active (consistent with the existing modal-suppression behavior of
    /// `clicked` / `hovered`). Available since v0.20.0.
    pub right_clicked: bool,
    /// Whether the mouse is hovering over the widget.
    pub hovered: bool,
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
    /// The rectangle the widget occupies after layout.
    pub rect: Rect,
}

impl Response {
    /// Create a Response with all fields false/default.
    pub fn none() -> Self {
        Self::default()
    }

    /// Attach a tooltip to this widget. Renders only when the widget is
    /// currently hovered.
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
    #[must_use = "on_hover returns the Response for further chaining"]
    pub fn on_hover(self, ctx: &mut Context, text: impl Into<String>) -> Self {
        if !self.hovered || self.rect.width == 0 || self.rect.height == 0 {
            return self;
        }
        let tooltip_text = text.into();
        if tooltip_text.is_empty() {
            return self;
        }
        let lines = super::widgets_display::wrap_tooltip_text(&tooltip_text, 38);
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
    #[must_use = "on_hover_ui returns the Response for further chaining"]
    pub fn on_hover_ui(self, ctx: &mut Context, f: impl FnOnce(&mut Context)) -> Self {
        if self.hovered && self.rect.width > 0 && self.rect.height > 0 {
            f(ctx);
        }
        self
    }
}
