//! Widget state types passed to [`Context`](crate::Context) widget methods.
//!
//! Each interactive widget (text input, list, tabs, table, etc.) has a
//! corresponding state struct defined here. Create the state once, then pass
//! a `&mut` reference each frame.

use crate::clock::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;

use crate::Style;
use crate::context::Response;

/// Bare function-pointer validator used by the deprecated positional
/// [`FormState::validate`](crate::widgets::FormState::validate) API.
///
/// Retained for backward compatibility. New code should attach
/// [`Validator`] closures per field via
/// [`FormField::validate`](crate::widgets::FormField::validate); see the
/// [`validators`] module for built-ins.
pub type FormValidator = fn(&str) -> Result<(), String>;
type TextInputValidator = Box<dyn Fn(&str) -> Result<(), String>>;

pub mod validators;

mod collections;
mod commanding;
mod feedback;
mod input;
mod responses;
mod selection;

pub use collections::*;
pub use commanding::*;
pub use feedback::*;
pub use input::*;
pub use responses::*;
pub use selection::*;

#[cfg(test)]
mod tests;
