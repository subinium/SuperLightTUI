//! Display widgets — text, alerts, badges, gauges, code blocks,
//! breadcrumbs, gutters, splits, and the `bordered`/`col`/`row` layout
//! shortcuts.
//!
//! These are Layer 3 widgets that produce visual output without
//! consuming events. For event-consuming widgets see
//! [`super::widgets_interactive`]; for input widgets see
//! [`super::widgets_input`].

use super::*;

mod fallback_highlight;
mod font8x8;
mod gauge;
mod gutter;
mod layout;
mod media;
mod rich_output;
mod split;
mod status;
mod text;
mod tooltip_wrap;

pub use gauge::{Gauge, LineGauge};
pub use gutter::GutterOpts;
pub use layout::Anchor;
pub use status::{Breadcrumb, CodeBlock};

use fallback_highlight::*;
use font8x8::*;
use media::*;
pub(super) use tooltip_wrap::wrap_tooltip_text;

#[cfg(test)]
mod line_wrap_tests;
#[cfg(test)]
mod tests;
