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
    ApprovalAction, ButtonVariant, CalendarState, CommandPaletteState, ContextItem,
    FilePickerState, FormField, FormState, GridColumn, ListState, MultiSelectState, RadioState,
    ScreenState, ScrollState, SelectState, SpinnerState, StreamingTextState, TableState, TabsState,
    TextInputState, TextareaState, ToastLevel, ToastState, ToolApprovalState, TreeState,
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
    eprintln!("[33m[SLT warning][0m {}", msg);
}

#[cfg(not(debug_assertions))]
#[allow(dead_code)]
fn slt_warn(_msg: &str) {}

mod widgets_display;
mod widgets_input;
mod widgets_interactive;
mod widgets_viz;
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
