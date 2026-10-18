//! Display family — text and style chains, links, timers, help bars, key
//! hints, badges, stats, breadcrumbs, accordions, definition lists and
//! empty states.
//!
//! Pure display widgets that do not own persistent state. Their supporting
//! types ([`crate::widgets::Trend`], [`crate::widgets::BreadcrumbResponse`])
//! live in `src/widgets/display.rs`.

use super::*;

mod status;
mod text;

pub use status::Breadcrumb;

#[cfg(test)]
mod tests;
