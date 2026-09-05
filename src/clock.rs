//! Platform clocks shared by frame timing, input and scheduling.

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) use web_time::{Instant, SystemTime, UNIX_EPOCH};
