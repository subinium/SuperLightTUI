//! Terminal backend unit tests, grouped by the submodule they exercise.

#![allow(clippy::unwrap_used)]
use super::*;

mod capability_tests;
mod detection_tests;
mod flush_tests;
mod graphics_tests;
mod lifecycle_tests;
mod query_tests;
mod selection_tests;

fn contains_seq(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}
