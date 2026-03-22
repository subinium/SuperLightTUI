use crate::chart::{build_histogram_config, render_chart, Candle, ChartBuilder, HistogramBuilder};
use crate::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseKind};
use crate::halfblock::HalfBlockImage;
use crate::layout::{Command, Direction};
use crate::rect::Rect;
use crate::style::{
    Align, Border, BorderSides, Breakpoint, Color, Constraints, ContainerStyle, Justify, Margin,
    Modifiers, Padding, Style, Theme, WidgetColors,
};
use crate::widgets::{
    ApprovalAction, ButtonVariant, CalendarState, CommandPaletteState, ContextItem,
    FilePickerState, FormField, FormState, ListState, MultiSelectState, RadioState, ScreenState,
    ScrollState, SelectState, SpinnerState, StreamingTextState, TableState, TabsState,
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

include!("context/state.rs");
include!("context/bars.rs");
include!("context/widget.rs");
include!("context/core.rs");
include!("context/container.rs");
include!("context/runtime.rs");
include!("context/helpers.rs");

#[cfg(test)]
mod tests;
