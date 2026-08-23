//! `Context` — the per-frame handle threaded into every render closure.
//!
//! This file defines the [`Context`] struct itself plus the dispatching
//! facade that imports widget impl blocks from the `widgets_display`,
//! `widgets_input`, `widgets_interactive`, and `widgets_viz` sub-modules.
//! See [`crate`] root for the entry-point [`crate::run`] / [`crate::frame`]
//! functions and `docs/ARCHITECTURE.md` for the 5-layer model that
//! organizes which method lives where.

use crate::FrameState;
use crate::chart::{Candle, ChartBuilder, HistogramBuilder, build_histogram_config, render_chart};
use crate::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseKind};
use crate::halfblock::HalfBlockImage;
use crate::layout::{BeginContainerArgs, BeginScrollableArgs, Command, Direction};
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Breakpoint, Color, Constraints, ContainerStyle, Justify, Margin,
    Modifiers, Padding, Spacing, Style, Theme, ThemeColor, WidgetColors, WidgetTheme,
};
use crate::widgets::{
    ApprovalAction, BreadcrumbResponse, ButtonVariant, CalDate, CalendarSelect, CalendarState,
    ColorPickerState, CommandPaletteState, ContextItem, FilePickerState, FormField, FormState,
    GaugeResponse, GridColumn, GutterResponse, HighlightRange, ListState, MultiSelectState,
    NumberInputState, PaginatorState, PaginatorStyle, PickerMode, RadioState, SchedKind,
    SchedulerSlot, SchedulerState, ScreenNav, ScreenState, ScrollState, SelectState, SpinnerState,
    SplitPaneResponse, SplitPaneState, StreamingTextState, TableState, TabsState, TextInputState,
    TextareaState, ToastLevel, ToastState, ToolApprovalState, TreeState, ValidateTrigger,
    color_hex_label, parse_hex_color,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[allow(dead_code)]
fn slt_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("[SLT] {msg}");
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code, clippy::print_stderr)]
fn slt_warn(msg: &str) {
    eprintln!("\x1b[33m[SLT warning]\x1b[0m {msg}");
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn slt_warn(_msg: &str) {}

mod widgets_display;
mod widgets_input;
mod widgets_interactive;
mod widgets_viz;
pub use widgets_display::{Anchor, Breadcrumb, CodeBlock, Gauge, GutterOpts, LineGauge};
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

/// Issue #234: in-frame async task API (`spawn`/`poll`), gated behind `async`.
#[cfg(feature = "async")]
mod async_tasks;
#[cfg(feature = "async")]
pub(crate) use async_tasks::AsyncTasks;
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub use async_tasks::{TaskHandle, TaskOutcome};

mod helpers;
pub(crate) use helpers::*;

#[cfg(all(test, feature = "async"))]
mod async_tasks_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod scheduler_tests;
