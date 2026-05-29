// Included into `crate::widgets::validators` via `include!`. Keep this file
// free of `use` statements that conflict with the parent module — refer to
// items by fully-qualified paths where ambiguity is possible.

/// Reject empty or whitespace-only input.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Name").validate(validators::required("Name is required"));
/// # let _ = field;
/// ```
pub fn required(msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let msg = msg.into();
    move |value| {
        if value.trim().is_empty() {
            Err(msg.clone())
        } else {
            Ok(())
        }
    }
}

/// Require at least `n` characters (counted as Unicode scalar values).
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Password").validate(validators::min_len(8, "min 8 chars"));
/// # let _ = field;
/// ```
pub fn min_len(n: usize, msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let msg = msg.into();
    move |value| {
        if value.chars().count() >= n {
            Ok(())
        } else {
            Err(msg.clone())
        }
    }
}

/// Require at most `n` characters (counted as Unicode scalar values).
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Bio").validate(validators::max_len(140, "max 140 chars"));
/// # let _ = field;
/// ```
pub fn max_len(n: usize, msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let msg = msg.into();
    move |value| {
        if value.chars().count() <= n {
            Ok(())
        } else {
            Err(msg.clone())
        }
    }
}

/// Accept a plausibly-formed email address.
///
/// This is a deliberately small structural check — exactly one `@`, a
/// non-empty local part, a non-empty domain that contains at least one `.`
/// with non-empty labels on both sides, and no whitespace. It is **not** a
/// full RFC 5322 parser; pass a stricter [`regex`] if you need one.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Email").validate(validators::email());
/// # let _ = field;
/// ```
pub fn email() -> impl Fn(&str) -> Result<(), String> {
    move |value| {
        if value.chars().any(char::is_whitespace) {
            return Err("invalid email".to_string());
        }
        let mut parts = value.split('@');
        let local = parts.next().unwrap_or("");
        let domain = parts.next().unwrap_or("");
        // Exactly one '@' (no third part) and both sides non-empty.
        if parts.next().is_some() || local.is_empty() || domain.is_empty() {
            return Err("invalid email".to_string());
        }
        // Domain needs a dot with non-empty labels on each side.
        match domain.rsplit_once('.') {
            Some((host, tld)) if !host.is_empty() && !tld.is_empty() => Ok(()),
            _ => Err("invalid email".to_string()),
        }
    }
}

/// Parse the input as an `i64` and require it to fall within `lo..=hi`.
///
/// Non-numeric input fails with `msg`.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Age").validate(validators::range_i64(0, 120, "0–120 only"));
/// # let _ = field;
/// ```
pub fn range_i64(lo: i64, hi: i64, msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let msg = msg.into();
    move |value| match value.trim().parse::<i64>() {
        Ok(n) if (lo..=hi).contains(&n) => Ok(()),
        _ => Err(msg.clone()),
    }
}

/// Parse the input as an `f64` and require it to fall within `lo..=hi`.
///
/// Non-numeric or non-finite input fails with `msg`.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Rate").validate(validators::range_f64(0.0, 1.0, "0.0–1.0 only"));
/// # let _ = field;
/// ```
pub fn range_f64(lo: f64, hi: f64, msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let msg = msg.into();
    move |value| match value.trim().parse::<f64>() {
        Ok(n) if n.is_finite() && n >= lo && n <= hi => Ok(()),
        _ => Err(msg.clone()),
    }
}

/// Require the input to be one of the allowed values (exact, case-sensitive).
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// let field = FormField::new("Role")
///     .validate(validators::one_of(&["admin", "user"], "admin or user"));
/// # let _ = field;
/// ```
pub fn one_of(allowed: &[&str], msg: impl Into<String>) -> impl Fn(&str) -> Result<(), String> {
    let allowed: Vec<String> = allowed.iter().map(|s| s.to_string()).collect();
    let msg = msg.into();
    move |value| {
        if allowed.iter().any(|a| a == value) {
            Ok(())
        } else {
            Err(msg.clone())
        }
    }
}

/// Match the input against a minimal glob-style pattern.
///
/// This is **not** a full regular-expression engine. The supported syntax is a
/// small literal matcher:
///
/// - `.` matches any single character.
/// - `*` matches zero or more of any character (greedy, with backtracking).
/// - `^` at the start anchors to the beginning (implied; always anchored).
/// - `$` at the end anchors to the end (implied; always anchored).
/// - any other character matches itself literally.
///
/// The whole input must match (the pattern is fully anchored). For real PCRE
/// support, wrap the `regex` crate in your own closure — SLT intentionally
/// ships no regex dependency.
///
/// # Example
///
/// ```no_run
/// # use slt::widgets::{FormField, validators};
/// // Three-letter code followed by digits, e.g. "ABC123".
/// let field = FormField::new("Code").validate(validators::regex("...*", "bad code"));
/// # let _ = field;
/// ```
pub fn regex(
    pattern: impl Into<String>,
    msg: impl Into<String>,
) -> impl Fn(&str) -> Result<(), String> {
    let mut pattern = pattern.into();
    let msg = msg.into();
    // Strip optional explicit anchors; matching is always whole-string.
    if let Some(stripped) = pattern.strip_prefix('^') {
        pattern = stripped.to_string();
    }
    if let Some(stripped) = pattern.strip_suffix('$') {
        pattern = stripped.to_string();
    }
    let pattern: Vec<char> = pattern.chars().collect();
    move |value| {
        let input: Vec<char> = value.chars().collect();
        if glob_match(&pattern, &input) {
            Ok(())
        } else {
            Err(msg.clone())
        }
    }
}

/// Whole-string glob matcher backing [`regex`]. Supports `.` (any char) and
/// `*` (zero-or-more, greedy with backtracking). Recursion depth is bounded by
/// pattern length, which is caller-controlled and small in practice.
fn glob_match(pattern: &[char], input: &[char]) -> bool {
    match pattern.split_first() {
        None => input.is_empty(),
        Some(('*', rest)) => {
            // Try to consume 0..=input.len() chars for '*', shortest first.
            for skip in 0..=input.len() {
                if glob_match(rest, &input[skip..]) {
                    return true;
                }
            }
            false
        }
        Some(('.', rest)) => !input.is_empty() && glob_match(rest, &input[1..]),
        Some((lit, rest)) => match input.split_first() {
            Some((head, tail)) if head == lit => glob_match(rest, tail),
            _ => false,
        },
    }
}
