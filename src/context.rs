//! `Context` — the per-frame handle threaded into every render closure.
//!
//! This file defines the [`Context`] struct itself plus the dispatching
//! facade that imports widget impl blocks from the `widgets_display`,
//! `widgets_input`, `widgets_interactive`, and `widgets_viz` sub-modules.
//! See [`crate`] root for the entry-point [`crate::run`] / [`crate::frame`]
//! functions and `docs/ARCHITECTURE.md` for the 5-layer model that
//! organizes which method lives where.

use crate::chart::{build_histogram_config, render_chart, Candle, ChartBuilder, HistogramBuilder};
use crate::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseKind};
use crate::halfblock::HalfBlockImage;
use crate::layout::{BeginContainerArgs, BeginScrollableArgs, Command, Direction};
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Breakpoint, Color, Constraints, ContainerStyle, Justify, Margin,
    Modifiers, Padding, Spacing, Style, Theme, ThemeColor, WidgetColors, WidgetTheme,
};
use crate::widgets::{
    ApprovalAction, BreadcrumbResponse, ButtonVariant, CalendarState, CommandPaletteState,
    ContextItem, FilePickerState, FormField, FormState, GaugeResponse, GridColumn, GutterResponse,
    HighlightRange, ListState, MultiSelectState, RadioState, SchedKind, SchedulerSlot,
    SchedulerState, ScreenState, ScrollState, SelectState, SpinnerState, SplitPaneResponse,
    SplitPaneState, StreamingTextState, TableState, TabsState, TextInputState, TextareaState,
    ToastLevel, ToastState, ToolApprovalState, TreeState,
};
use crate::FrameState;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[allow(dead_code)]
fn slt_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("[SLT] {}", msg);
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code, clippy::print_stderr)]
fn slt_warn(msg: &str) {
    eprintln!("\x1b[33m[SLT warning]\x1b[0m {}", msg);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn slt_warn(_msg: &str) {}

mod widgets_display;
mod widgets_input;
mod widgets_interactive;
mod widgets_viz;
pub use widgets_display::{Anchor, Breadcrumb, Gauge, GutterOpts, LineGauge};
pub use widgets_viz::TreemapItem;

mod state;
pub use state::*;

mod bars;
pub use bars::*;

mod widget;
pub use widget::*;

mod core;
pub use core::*;

mod container;
pub use container::*;

mod runtime;

mod helpers;
pub(crate) use helpers::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod scheduler_tests;
