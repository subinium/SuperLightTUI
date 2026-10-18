//! Diff viewer demo — review an agent-proposed patch hunk by hunk.
//!
//! Run: `cargo run --example diff_viewer`
//! With Rust highlighting: `cargo run --example diff_viewer --features syntax-rust`
//!
//! Keys:
//!   j / k, Up / Down   — move the cursor
//!   ] / n, [ / p       — next / previous hunk
//!   Enter / Space      — collapse or expand the focused hunk (or click a header)
//!   a / r              — accept / reject the focused hunk
//!   s                  — toggle unified / side-by-side layout
//!   q / Esc            — quit

use slt::{Border, Context, DiffLayout, DiffState, HunkAction, HunkDecision, KeyCode, RunConfig};

const PATCH: &str = r#"diff --git a/src/config.rs b/src/config.rs
index 3f2a1c0..9b4e7d2 100644
--- a/src/config.rs
+++ b/src/config.rs
@@ -1,9 +1,10 @@
 use std::time::Duration;

 pub struct Config {
-    pub retries: u32,
+    pub retries: u8,
     pub timeout: Duration,
+    pub verbose: bool,
 }

 impl Default for Config {
     fn default() -> Self {
@@ -12,6 +13,7 @@ impl Default for Config {
         Self {
             retries: 3,
             timeout: Duration::from_secs(30),
+            verbose: false,
         }
     }
 }
diff --git a/docs/README.md b/docs/README.md
similarity index 88%
rename from README.md
rename to docs/README.md
index 1111111..2222222 100644
--- a/README.md
+++ b/docs/README.md
@@ -1,3 +1,3 @@
 # Demo
-Greeting:	hello
+Greeting:	안녕하세요 (CJK and tabs line up)
 Done.
diff --git a/assets/logo.png b/assets/logo.png
index 0a1b2c3..4d5e6f7 100644
Binary files a/assets/logo.png and b/assets/logo.png differ
"#;

fn main() -> std::io::Result<()> {
    let mut diff = DiffState::from_unified(PATCH).map_err(std::io::Error::other)?;
    let mut layout = DiffLayout::Unified;
    let mut last_event = String::from("No decisions yet");

    slt::run_with(RunConfig::default().mouse(true), move |ui: &mut Context| {
        if ui.key('q') || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }
        if ui.key('s') {
            layout = match layout {
                DiffLayout::Unified => DiffLayout::Split,
                _ => DiffLayout::Unified,
            };
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Review patch")
            .p(1)
            .grow(1)
            .col(|ui: &mut Context| {
                let rows = ui.height().saturating_sub(6);
                let response = ui
                    .diff(&mut diff)
                    .layout(layout)
                    .split_min_width(90)
                    .h(rows)
                    .show();
                for (id, action) in &response.hunk_actions {
                    let verb = match action {
                        HunkAction::Accept => "accepted",
                        HunkAction::Reject => "rejected",
                    };
                    last_event = format!("{verb} hunk {} of file {}", id.hunk + 1, id.file + 1);
                }

                let accepted = diff
                    .hunk_ids()
                    .filter(|id| diff.decision(*id) == Some(HunkDecision::Accepted))
                    .count();
                let rejected = diff
                    .hunk_ids()
                    .filter(|id| diff.decision(*id) == Some(HunkDecision::Rejected))
                    .count();
                ui.text(format!(
                    "{accepted} accepted · {rejected} rejected · {} pending · {last_event}",
                    diff.hunk_count() - accepted - rejected
                ))
                .dim();
                ui.text("j/k move · ]/[ hunk · Enter fold · a/r accept/reject · s layout · q quit")
                    .dim();
            });
    })
}
