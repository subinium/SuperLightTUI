//! Cookbook: file picker with text preview.
//!
//! Left pane: `FilePickerState` browsing the current working directory.
//! Right pane: preview of the selected text file (first ~30 lines, up to 64 KB).
//!
//! Non-text files show a short placeholder. Large files are skipped up-front
//! so we never buffer a huge blob into the UI.
//!
//! Keys:
//! - Enter: open directory / select file
//! - Backspace: go up one level
//! - q or Esc: quit

use std::fs;
use std::path::Path;

use slt::{Border, Color, Context, FilePickerState, KeyCode, KeyModifiers};

const MAX_PREVIEW_BYTES: u64 = 64 * 1024;
const MAX_PREVIEW_LINES: usize = 30;
const TEXT_EXTS: &[&str] = &["txt", "md", "rs", "toml", "log"];

fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            TEXT_EXTS.iter().any(|allowed| *allowed == e)
        })
        .unwrap_or(false)
}

fn read_preview(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| format!("stat failed: {e}"))?;
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err(format!(
            "file too large: {} bytes (limit {MAX_PREVIEW_BYTES})",
            meta.len()
        ));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("read failed: {e}"))?;
    let preview: String = text
        .lines()
        .take(MAX_PREVIEW_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    Ok(preview)
}

fn main() -> std::io::Result<()> {
    let start = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let mut picker = FilePickerState::new(start);
    let mut preview: Option<Result<String, String>> = None;
    let mut preview_path: Option<std::path::PathBuf> = None;

    slt::run(|ui: &mut Context| {
        if ui.key('q') || ui.key_mod('q', KeyModifiers::CONTROL) || ui.key_code(KeyCode::Esc) {
            ui.quit();
        }

        // If the picker surfaced a new selection this frame, refresh the preview.
        if picker.selected_file.as_ref() != preview_path.as_ref() {
            preview_path = picker.selected_file.clone();
            preview = preview_path.as_ref().map(|p| {
                if is_text(p) {
                    read_preview(p)
                } else {
                    Err("Binary or unsupported file type.".into())
                }
            });
        }

        let _ = ui
            .bordered(Border::Rounded)
            .title("Cookbook — File Picker")
            .pad(1)
            .gap(1)
            .grow(1)
            .col(|ui| {
                let _ = ui.container().grow(1).row(|ui| {
                    let _ = ui
                        .bordered(Border::Single)
                        .title("Files")
                        .pad(1)
                        .grow(1)
                        .col(|ui| {
                            let _ = ui.file_picker(&mut picker);
                        });

                    let _ = ui
                        .bordered(Border::Single)
                        .title("Preview")
                        .pad(1)
                        .grow(2)
                        .col(|ui| match preview.as_ref() {
                            Some(Ok(text)) if !text.is_empty() => {
                                for line in text.lines() {
                                    ui.text(line.to_string());
                                }
                            }
                            Some(Ok(_)) => {
                                ui.text("(empty file)").dim();
                            }
                            Some(Err(msg)) => {
                                ui.text(msg.as_str()).fg(Color::Yellow);
                            }
                            None => {
                                ui.text("Select a text file to preview.").dim();
                                ui.text("").dim();
                                ui.text("Supported: .txt .md .rs .toml .log").dim();
                            }
                        });
                });

                ui.text("Enter: open/select   Backspace: up   q/Esc: quit")
                    .dim();
            });
    })
}
