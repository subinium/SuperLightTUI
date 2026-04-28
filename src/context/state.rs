use super::*;

/// Internal discriminator for [`State<T>`] handles.
///
/// `Indexed` refers to a slot in `Context::hook_states` (positional, used by
/// [`Context::use_state`] / [`Context::use_memo`]). `Named` refers to a key in
/// `Context::named_states` (used by [`Context::use_state_named`]).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum StateKey {
    Indexed(usize),
    Named(&'static str),
}

/// Handle to state created by `use_state()`. Access via `.get(ui)` / `.get_mut(ui)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct State<T> {
    key: StateKey,
    _marker: std::marker::PhantomData<T>,
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

    /// Read the current value.
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        match self.key {
            StateKey::Indexed(idx) => {
                ui.hook_states[idx].downcast_ref::<T>().unwrap_or_else(|| {
                    panic!(
                        "use_state type mismatch at hook index {} — expected {}",
                        idx,
                        std::any::type_name::<T>()
                    )
                })
            }
            StateKey::Named(id) => ui
                .named_states
                .get(id)
                .unwrap_or_else(|| {
                    panic!(
                        "use_state_named: no entry for id {:?} — was use_state_named called?",
                        id
                    )
                })
                .downcast_ref::<T>()
                .unwrap_or_else(|| {
                    panic!(
                        "use_state_named type mismatch for id {:?} — expected {}",
                        id,
                        std::any::type_name::<T>()
                    )
                }),
        }
    }

    /// Mutably access the current value.
    pub fn get_mut<'a>(&self, ui: &'a mut Context) -> &'a mut T {
        match self.key {
            StateKey::Indexed(idx) => {
                ui.hook_states[idx].downcast_mut::<T>().unwrap_or_else(|| {
                    panic!(
                        "use_state type mismatch at hook index {} — expected {}",
                        idx,
                        std::any::type_name::<T>()
                    )
                })
            }
            StateKey::Named(id) => ui
                .named_states
                .get_mut(id)
                .unwrap_or_else(|| {
                    panic!(
                        "use_state_named: no entry for id {:?} — was use_state_named called?",
                        id
                    )
                })
                .downcast_mut::<T>()
                .unwrap_or_else(|| {
                    panic!(
                        "use_state_named type mismatch for id {:?} — expected {}",
                        id,
                        std::any::type_name::<T>()
                    )
                }),
        }
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
    /// Whether the widget was clicked this frame.
    pub clicked: bool,
    /// Whether the mouse is hovering over the widget.
    pub hovered: bool,
    /// Whether the widget's value changed this frame.
    pub changed: bool,
    /// Whether the widget currently has keyboard focus.
    pub focused: bool,
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
