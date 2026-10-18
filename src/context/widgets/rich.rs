//! Rich-content family — rich logs, markdown, streaming text / markdown,
//! tool approval, context bars and code blocks.
//!
//! State types ([`crate::widgets::RichLogState`],
//! [`crate::widgets::StreamingTextState`],
//! [`crate::widgets::ToolApprovalState`], [`crate::widgets::ContextItem`], ...)
//! live in `src/widgets/rich.rs`.

use super::*;

mod code_block;
mod diff;
mod fallback_highlight;
mod markdown;
mod rich_log;
mod streaming;

pub use code_block::CodeBlock;
pub use diff::DiffView;

use fallback_highlight::*;

#[cfg(test)]
mod tests;
