//! `Context` — the per-frame handle threaded into every render closure.
//!
//! This file defines the [`Context`] struct itself plus the dispatching
//! facade that imports widget impl blocks from the `widgets` sub-module,
//! which groups them by widget family (`widgets/<family>.rs`, paired with
//! the state file of the same name in [`crate::widgets`]).
//! See [`crate`] root for the entry-point [`crate::run`] / [`crate::frame`]
//! functions and `docs/internals/architecture.md` for the 5-layer model that
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

fn slt_assert(condition: bool, msg: &str) {
    if !condition {
        panic!("[SLT] {msg}");
    }
}

#[cfg(debug_assertions)]
#[expect(
    clippy::print_stderr,
    reason = "debug-build developer diagnostics go to stderr by design"
)]
fn slt_warn(msg: &str) {
    eprintln!("\x1b[33m[SLT warning]\x1b[0m {msg}");
}

#[cfg(not(debug_assertions))]
fn slt_warn(_msg: &str) {}

mod widgets;
pub use widgets::CodeEditor;
#[expect(
    deprecated,
    reason = "re-exports the deprecated ModalOptions alias for one release (#462)"
)]
pub use widgets::ModalOptions;
pub use widgets::ModalOpts;
pub use widgets::Text;
pub use widgets::{
    Anchor, Badge, Breadcrumb, Button, CodeBlock, DiffView, Gauge, GutterOpts, Help, LineChart,
    LineGauge, Placement, ProgressBar, Separator, Slider, Stat, TreemapItem,
};

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

mod pointer;
pub(crate) use pointer::*;

mod pointer_shape;
pub use pointer_shape::PointerShape;

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

mod terminal_meta;
pub(crate) use terminal_meta::store_clipboard_status;
pub use terminal_meta::{ClipboardStatus, CursorShape, ProgramState, ProgramStatus, Progress};
#[cfg(feature = "crossterm")]
pub(crate) use terminal_meta::{TerminalMeta, terminal_meta};

#[cfg(all(test, feature = "async"))]
mod async_tasks_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod scheduler_tests;
