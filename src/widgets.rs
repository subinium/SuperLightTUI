//! Widget state types passed to [`Context`](crate::Context) widget methods.
//!
//! Each interactive widget (text input, list, tabs, table, etc.) has a
//! corresponding state struct defined here. Create the state once, then pass
//! a `&mut` reference each frame.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use unicode_width::UnicodeWidthStr;

use crate::context::Response;
use crate::Style;

type FormValidator = fn(&str) -> Result<(), String>;
type TextInputValidator = Box<dyn Fn(&str) -> Result<(), String>>;

include!("widgets/input.rs");
include!("widgets/collections.rs");
include!("widgets/feedback.rs");
include!("widgets/selection.rs");
include!("widgets/commanding.rs");
include!("widgets/responses.rs");

#[cfg(test)]
mod tests;
