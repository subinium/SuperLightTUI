//! Example-parity tests: render the demos in `examples/` headlessly.
//!
//! Each module either includes an example source via `#[path]` and calls its
//! deterministic render entry point, or re-runs the same UI shape inside a
//! `TestBackend`, so a widget or layout change that breaks a shipped demo
//! fails here instead of only in a terminal. File snapshots live in
//! `tests/snapshots/` (`example_snapshots__*.snap`, `visual__dx_shortcuts.snap`).
//!
//! The modules share one test crate so the demos link against the library
//! once instead of once per file.

mod dx_shortcuts;
mod hooks;
mod lib_demos;
mod regression_panel;
mod testing_report;
mod theme_modal;
mod widgets;
