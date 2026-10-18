//! Run loops and the shared frame kernel.
//!
//! Private module. The public entry points (`run*`, `RunConfig`, `DebugLayer`
//! and the async handles) are re-exported from the crate root at their
//! historical paths. Crate-internal items are glob re-exported here at
//! `pub(crate)`, and the ones the rest of the crate names through the crate
//! root are re-exported from `lib.rs`.

#[cfg(all(feature = "crossterm", feature = "async"))]
mod async_run;
mod config;
mod frame_state;
mod kernel;
#[cfg(feature = "crossterm")]
mod native;
mod panic;
#[cfg(all(feature = "crossterm", unix))]
mod suspend;
#[cfg(all(test, feature = "crossterm"))]
mod tests;

#[cfg(all(feature = "crossterm", feature = "async"))]
pub use async_run::{AsyncRunError, AsyncRunHandle, AsyncSender, run_async, run_async_with};
pub use config::RunConfig;
pub use frame_state::DebugLayer;
#[cfg(feature = "crossterm")]
pub use native::{run, run_inline, run_inline_with, run_static, run_static_with, run_with};

#[cfg(all(test, feature = "crossterm", feature = "async"))]
use async_run::*;
pub(crate) use frame_state::*;
pub(crate) use kernel::*;
#[cfg(feature = "crossterm")]
pub(crate) use native::*;
pub(crate) use panic::*;
#[cfg(all(feature = "crossterm", unix))]
use suspend::*;
