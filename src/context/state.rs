use super::*;

/// Handle to state created by `use_state()`. Access via `.get(ui)` / `.get_mut(ui)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct State<T> {
    idx: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> State<T> {
    pub(crate) fn from_idx(idx: usize) -> Self {
        Self {
            idx,
            _marker: std::marker::PhantomData,
        }
    }

    /// Read the current value.
    pub fn get<'a>(&self, ui: &'a Context) -> &'a T {
        ui.hook_states[self.idx]
            .downcast_ref::<T>()
            .unwrap_or_else(|| {
                panic!(
                    "use_state type mismatch at hook index {} — expected {}",
                    self.idx,
                    std::any::type_name::<T>()
                )
            })
    }

    /// Mutably access the current value.
    pub fn get_mut<'a>(&self, ui: &'a mut Context) -> &'a mut T {
        ui.hook_states[self.idx]
            .downcast_mut::<T>()
            .unwrap_or_else(|| {
                panic!(
                    "use_state type mismatch at hook index {} — expected {}",
                    self.idx,
                    std::any::type_name::<T>()
                )
            })
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
}
