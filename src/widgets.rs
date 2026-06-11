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

/// Built-in field validator constructors (required / length / email / range /
/// regex / one-of).
///
/// Each function returns a closure suitable for
/// [`FormField::validate`](crate::widgets::FormField::validate) or
/// [`Validator::new`](crate::widgets::Validator::new). All matchers are
/// handwritten — there is no `regex` or email-parsing dependency.
pub mod validators {
    include!("widgets/validators.rs");
}

include!("widgets/input.rs");
include!("widgets/collections.rs");
include!("widgets/feedback.rs");
include!("widgets/selection.rs");
include!("widgets/commanding.rs");
include!("widgets/responses.rs");

#[cfg(test)]
mod tests;
