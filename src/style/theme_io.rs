//! External TOML theme files and (optionally) a filesystem hot-reload watcher.
//!
//! Gated behind the `serde` feature; the [`ThemeWatcher`] additionally requires
//! the `theme-watch` feature (which pulls in `notify`). Neither `toml` nor
//! `notify` is compiled into the default or `wasm32` builds.
//!
//! The format is a single TOML document with a `[theme]` table and an optional
//! `[widgets]` table:
//!
//! ```toml
//! [theme]
//! primary = "#ff6b6b"
//! accent  = "cyan"
//! bg      = "#1e1e2e"
//! text    = "indexed:250"
//! is_dark = true
//!
//! [widgets.button]
//! fg = "#ffffff"
//! ```

use super::Theme;
use crate::WidgetTheme;

/// Error returned when loading a theme from a file or string fails.
///
/// Carries either the underlying I/O failure or a human-readable parse
/// message. Never panics on malformed input — callers decide how to recover.
#[non_exhaustive]
#[derive(Debug)]
pub enum ThemeLoadError {
    /// The theme file could not be read from disk.
    Io(std::io::Error),
    /// The document was read but is not valid TOML, or did not match the
    /// expected [`ThemeFile`] shape. The string carries the parser's message.
    Parse(String),
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeLoadError::Io(e) => write!(f, "failed to read theme file: {e}"),
            ThemeLoadError::Parse(msg) => write!(f, "failed to parse theme TOML: {msg}"),
        }
    }
}

impl core::error::Error for ThemeLoadError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            ThemeLoadError::Io(e) => Some(e),
            ThemeLoadError::Parse(_) => None,
        }
    }
}

impl From<std::io::Error> for ThemeLoadError {
    fn from(e: std::io::Error) -> Self {
        ThemeLoadError::Io(e)
    }
}

/// A parsed theme document: a base [`Theme`] plus optional [`WidgetTheme`] slots.
///
/// Use [`ThemeFile::from_toml_str`] / [`ThemeFile::load`] to construct one, then
/// feed `theme` into [`crate::Context::set_theme`] and `widgets` into
/// [`crate::RunConfig::widget_theme`].
///
/// # Example
///
/// ```no_run
/// use slt::ThemeFile;
///
/// let tf = ThemeFile::from_toml_str(r##"
/// [theme]
/// primary = "#ff0000"
///
/// [widgets.button]
/// fg = "#ffffff"
/// "##).unwrap();
/// assert_eq!(tf.theme.primary, slt::Color::Rgb(255, 0, 0));
/// assert!(tf.widgets.is_some());
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ThemeFile {
    /// The base theme (the `[theme]` table). Missing fields fall back to
    /// [`Theme::dark()`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub theme: Theme,
    /// Optional per-widget color overrides (the `[widgets]` table).
    #[cfg_attr(feature = "serde", serde(default))]
    pub widgets: Option<WidgetTheme>,
}

impl ThemeFile {
    /// Parse a [`ThemeFile`] from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeLoadError::Parse`] for malformed TOML or a shape that
    /// does not match the expected `[theme]` / `[widgets]` layout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::ThemeFile;
    ///
    /// let tf = ThemeFile::from_toml_str("[theme]\nprimary = \"#00ff00\"\n").unwrap();
    /// assert_eq!(tf.theme.primary, slt::Color::Rgb(0, 255, 0));
    /// ```
    pub fn from_toml_str(src: &str) -> Result<ThemeFile, ThemeLoadError> {
        toml::from_str(src).map_err(|e| ThemeLoadError::Parse(e.to_string()))
    }

    /// Serialize this [`ThemeFile`] back to a TOML string.
    ///
    /// The output round-trips through [`ThemeFile::from_toml_str`]. Colors are
    /// emitted as human-friendly tokens (`#rrggbb`, named, or `indexed:N`).
    ///
    /// # Errors
    ///
    /// Returns [`ThemeLoadError::Parse`] if serialization fails (e.g. a value
    /// that TOML cannot represent).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::{Theme, ThemeFile};
    ///
    /// let tf = ThemeFile { theme: Theme::dracula(), widgets: None };
    /// let toml = tf.to_toml_string().unwrap();
    /// assert!(toml.contains("[theme]"));
    /// ```
    pub fn to_toml_string(&self) -> Result<String, ThemeLoadError> {
        toml::to_string(self).map_err(|e| ThemeLoadError::Parse(e.to_string()))
    }

    /// Read and parse a [`ThemeFile`] from a TOML file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeLoadError::Io`] if the file cannot be read, or
    /// [`ThemeLoadError::Parse`] if its contents are not valid TOML.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::ThemeFile;
    ///
    /// let tf = ThemeFile::load("theme.toml").unwrap();
    /// println!("primary = {:?}", tf.theme.primary);
    /// ```
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<ThemeFile, ThemeLoadError> {
        let src = std::fs::read_to_string(path)?;
        Self::from_toml_str(&src)
    }
}

/// A non-blocking filesystem watcher that hot-reloads a TOML theme file.
///
/// Requires the `theme-watch` feature. The watcher runs `notify`'s own
/// background thread and buffers change events on a channel; [`poll`] drains
/// the channel, re-reads the file, and returns the freshly parsed
/// [`ThemeFile`]. On a parse error it logs context to stderr and keeps the last
/// good theme, so a half-saved edit never breaks the running app.
///
/// Designed for SLT's immediate-mode loop: call [`poll`] once per frame and
/// apply the result via [`crate::Context::set_theme`].
///
/// [`poll`]: ThemeWatcher::poll
///
/// # Example
///
/// ```no_run
/// use slt::ThemeWatcher;
///
/// let mut watcher = ThemeWatcher::new("theme.toml").unwrap();
/// slt::run(move |ui| {
///     if let Some(tf) = watcher.poll() {
///         ui.set_theme(tf.theme);
///     }
///     ui.button("Themed");
/// })
/// .unwrap();
/// ```
#[cfg(feature = "theme-watch")]
#[cfg_attr(docsrs, doc(cfg(feature = "theme-watch")))]
pub struct ThemeWatcher {
    // Held to keep the watch alive; dropping it stops the background thread.
    _watcher: notify::RecommendedWatcher,
    rx: std::sync::mpsc::Receiver<()>,
    path: std::path::PathBuf,
    last_source: String,
    last_good: ThemeFile,
}

#[cfg(feature = "theme-watch")]
impl ThemeWatcher {
    /// Start watching the theme file at `path`, loading it once up front.
    ///
    /// # Errors
    ///
    /// Returns [`ThemeLoadError::Io`] if the initial read fails or the watch
    /// cannot be registered, or [`ThemeLoadError::Parse`] if the initial file
    /// is not valid TOML.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::ThemeWatcher;
    ///
    /// let watcher = ThemeWatcher::new("theme.toml").unwrap();
    /// ```
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<ThemeWatcher, ThemeLoadError> {
        use notify::{RecursiveMode, Watcher};

        let path = path.as_ref();
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        // Keep the last observed source as well as the parsed theme. Some
        // backends emit an initial event when a watch is registered, and
        // editors can emit several events for one save. Identical contents
        // must not look like a hot reload to the application.
        let last_source = std::fs::read_to_string(&path)?;
        let last_good = ThemeFile::from_toml_str(&last_source)?;

        // A filesystem burst only means "re-read the current file once". A
        // capacity-one channel coalesces duplicate events and prevents a noisy
        // parent directory from growing an unbounded notification queue while
        // the UI is busy or polls infrequently.
        let (tx, rx) = std::sync::mpsc::sync_channel::<()>(1);
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            // Some backends report only the watched directory rather than the
            // changed file. Forward every successful event and let poll()
            // compare the source, which is both portable and preserves
            // atomic-save support without surfacing sibling-file changes.
            if res.is_ok() {
                let _ = tx.try_send(());
            }
        })
        .map_err(|e| ThemeLoadError::Io(std::io::Error::other(e.to_string())))?;

        // Watch the parent directory: editors often replace the file (rename)
        // rather than writing in place, which a file-level watch can miss.
        let watch_target = path.parent().filter(|p| !p.as_os_str().is_empty());
        let (target, mode) = match watch_target {
            Some(dir) => (dir, RecursiveMode::NonRecursive),
            None => (path.as_path(), RecursiveMode::NonRecursive),
        };
        watcher
            .watch(target, mode)
            .map_err(|e| ThemeLoadError::Io(std::io::Error::other(e.to_string())))?;

        Ok(ThemeWatcher {
            _watcher: watcher,
            rx,
            path,
            last_source,
            last_good,
        })
    }

    /// The most recently parsed theme (the initial load, or the last good
    /// hot-reload). Never returns a theme from a failed parse.
    pub fn current(&self) -> &ThemeFile {
        &self.last_good
    }

    /// Non-blocking poll for a hot-reloaded theme.
    ///
    /// Drains pending filesystem events; if any occurred, re-reads and parses
    /// the watched file. Returns `Some(theme)` only when the file changed *and*
    /// parsed cleanly. Returns `None` when nothing changed, or when the new
    /// contents failed to parse — in which case the previous theme is retained
    /// (accessible via [`ThemeWatcher::current`]) and a message is logged to
    /// stderr.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use slt::ThemeWatcher;
    ///
    /// let mut watcher = ThemeWatcher::new("theme.toml").unwrap();
    /// if let Some(tf) = watcher.poll() {
    ///     println!("reloaded: {:?}", tf.theme.primary);
    /// }
    /// ```
    // Intentional stderr diagnostic on a half-saved theme file: the hot-reload
    // loop must surface why a reload was skipped without aborting the app.
    #[allow(clippy::print_stderr)]
    pub fn poll(&mut self) -> Option<ThemeFile> {
        // Drain all buffered events; a burst of writes collapses to one reload.
        let mut changed = false;
        while self.rx.try_recv().is_ok() {
            changed = true;
        }
        if !changed {
            return None;
        }

        let source = match std::fs::read_to_string(&self.path) {
            Ok(source) => source,
            Err(e) => {
                eprintln!(
                    "slt: theme hot-reload skipped for {}: {}",
                    self.path.display(),
                    ThemeLoadError::Io(e)
                );
                return None;
            }
        };
        if source == self.last_source {
            return None;
        }
        self.last_source.clone_from(&source);

        match ThemeFile::from_toml_str(&source) {
            Ok(tf) => {
                self.last_good = tf.clone();
                Some(tf)
            }
            Err(e) => {
                // Keep the last good theme; never panic on a half-saved file.
                eprintln!(
                    "slt: theme hot-reload skipped for {}: {e}",
                    self.path.display()
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::Color;

    fn all_presets() -> Vec<(&'static str, Theme)> {
        vec![
            ("dark", Theme::dark()),
            ("light", Theme::light()),
            ("dracula", Theme::dracula()),
            ("catppuccin", Theme::catppuccin()),
            ("nord", Theme::nord()),
            ("solarized_dark", Theme::solarized_dark()),
            ("solarized_light", Theme::solarized_light()),
            ("tokyo_night", Theme::tokyo_night()),
            ("gruvbox_dark", Theme::gruvbox_dark()),
            ("one_dark", Theme::one_dark()),
        ]
    }

    fn theme_eq(a: &Theme, b: &Theme) -> bool {
        a.primary == b.primary
            && a.secondary == b.secondary
            && a.accent == b.accent
            && a.text == b.text
            && a.text_dim == b.text_dim
            && a.border == b.border
            && a.bg == b.bg
            && a.success == b.success
            && a.warning == b.warning
            && a.error == b.error
            && a.selected_bg == b.selected_bg
            && a.selected_fg == b.selected_fg
            && a.surface == b.surface
            && a.surface_hover == b.surface_hover
            && a.surface_text == b.surface_text
            && a.is_dark == b.is_dark
            && a.spacing == b.spacing
    }

    #[test]
    fn parses_minimal_theme_doc() {
        let toml = r##"
            [theme]
            primary = "#ff6b6b"
            bg = "#1e1e2e"
            is_dark = true
        "##;
        let tf = ThemeFile::from_toml_str(toml).unwrap();
        assert_eq!(tf.theme.primary, Color::Rgb(255, 107, 107));
        assert_eq!(tf.theme.bg, Color::Rgb(30, 30, 46));
        assert!(tf.theme.is_dark);
        // Unspecified fields fall back to dark() defaults.
        assert_eq!(tf.theme.text, Theme::dark().text);
        assert!(tf.widgets.is_none());
    }

    #[test]
    fn named_and_indexed_colors_parse() {
        let toml = r#"
            [theme]
            primary = "cyan"
            text = "indexed:250"
            bg = "reset"
        "#;
        let tf = ThemeFile::from_toml_str(toml).unwrap();
        assert_eq!(tf.theme.primary, Color::Cyan);
        assert_eq!(tf.theme.text, Color::Indexed(250));
        assert_eq!(tf.theme.bg, Color::Reset);
    }

    #[test]
    fn round_trips_every_preset() {
        for (name, theme) in all_presets() {
            let tf = ThemeFile {
                theme,
                widgets: None,
            };
            let serialized = tf.to_toml_string().unwrap();
            let parsed = Theme::from_toml_str(&serialized).unwrap();
            assert!(
                theme_eq(&theme, &parsed),
                "preset {name} did not round-trip: {theme:?} != {parsed:?}\nTOML:\n{serialized}"
            );
        }
    }

    #[test]
    fn widgets_block_deserializes() {
        let toml = r##"
            [theme]
            primary = "#ff0000"

            [widgets.table]
            fg = "#00ff00"
            theme_bg = "Surface"
        "##;
        let tf = ThemeFile::from_toml_str(toml).unwrap();
        let widgets = tf.widgets.expect("widgets block present");
        assert_eq!(widgets.table.fg, Some(Color::Rgb(0, 255, 0)));
        assert_eq!(widgets.table.theme_bg, Some(crate::ThemeColor::Surface));
        // Unset slots default to empty WidgetColors.
        assert_eq!(widgets.button.fg, None);
    }

    #[test]
    fn malformed_toml_is_parse_error_not_panic() {
        let err = ThemeFile::from_toml_str("this is = not [valid").unwrap_err();
        assert!(matches!(err, ThemeLoadError::Parse(_)));
    }

    #[test]
    fn bad_color_token_is_parse_error() {
        let toml = r##"
            [theme]
            primary = "#zzzzzz"
        "##;
        let err = ThemeFile::from_toml_str(toml).unwrap_err();
        assert!(matches!(err, ThemeLoadError::Parse(_)));
    }

    #[test]
    fn from_hex_parses_short_and_long_forms() {
        assert_eq!(Color::from_hex("#ff6b6b"), Some(Color::Rgb(255, 107, 107)));
        assert_eq!(Color::from_hex("#abc"), Some(Color::Rgb(170, 187, 204)));
        assert_eq!(Color::from_hex("#000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(Color::from_hex("#FFFFFF"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(Color::from_hex("ffffff"), None);
        assert_eq!(Color::from_hex("#xyz"), None);
        assert_eq!(Color::from_hex("#ff"), None);
        assert_eq!(Color::from_hex(""), None);
    }

    #[test]
    fn from_hex_to_hex_round_trip() {
        for r in [0u8, 1, 127, 200, 255] {
            for g in [0u8, 64, 128, 255] {
                for b in [0u8, 99, 255] {
                    let c = Color::Rgb(r, g, b);
                    assert_eq!(Color::from_hex(&c.to_hex()), Some(c));
                }
            }
        }
    }

    #[test]
    fn theme_load_ignores_widgets() {
        let toml = r##"
            [theme]
            primary = "#abcdef"

            [widgets.button]
            fg = "#123456"
        "##;
        let theme = Theme::from_toml_str(toml).unwrap();
        assert_eq!(theme.primary, Color::Rgb(0xab, 0xcd, 0xef));
    }
}

#[cfg(all(test, feature = "crossterm"))]
mod render_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{ButtonVariant, Color, TestBackend};

    #[test]
    fn loaded_primary_paints_focused_button() {
        let tf = ThemeFile::from_toml_str(
            r##"
            [theme]
            primary = "#ff0000"
            "##,
        )
        .unwrap();
        let loaded_primary = tf.theme.primary;
        assert_eq!(loaded_primary, Color::Rgb(255, 0, 0));

        let mut tb = TestBackend::new(20, 5);
        // Focus index 0 so the single button is focused; the Default variant
        // paints `theme.primary` as the label foreground when focused.
        tb.render_with_events(Vec::new(), 0, 1, move |ui| {
            ui.set_theme(tf.theme);
            let _ = ui.button_with("Go", ButtonVariant::Default);
        });

        // The widget rendered.
        tb.assert_contains("Go");

        // The loaded primary is the load-bearing change: it must appear as a
        // foreground color on at least one painted cell of the focused button.
        let buffer = tb.buffer();
        let mut found_primary = false;
        for y in 0..tb.height() {
            for x in 0..tb.width() {
                if buffer.get(x, y).style.fg == Some(loaded_primary) {
                    found_primary = true;
                }
            }
        }
        assert!(
            found_primary,
            "expected loaded primary {loaded_primary:?} to paint at least one cell"
        );
    }
}

#[cfg(all(test, feature = "theme-watch"))]
mod watch_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::Color;
    use std::time::{Duration, Instant};

    /// Spin on poll() until it returns a theme or the deadline elapses.
    fn poll_until_change(watcher: &mut ThemeWatcher, timeout: Duration) -> Option<ThemeFile> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(tf) = watcher.poll() {
                return Some(tf);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = format!(
            "slt_theme_watch_{}_{}_{name}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        dir.push(unique);
        std::fs::create_dir_all(&dir).unwrap();
        dir.push(name);
        dir
    }

    #[test]
    fn watcher_reports_changes_and_survives_bad_toml() {
        let path = temp_path("theme.toml");
        std::fs::write(&path, "[theme]\nprimary = \"#0000ff\"\n").unwrap();

        let mut watcher = ThemeWatcher::new(&path).unwrap();
        assert_eq!(watcher.current().theme.primary, Color::Rgb(0, 0, 255));

        // Registration events and same-content rewrites are not reloads.
        assert!(watcher.poll().is_none());
        std::fs::write(&path, "[theme]\nprimary = \"#0000ff\"\n").unwrap();
        std::thread::sleep(Duration::from_millis(200));
        assert!(watcher.poll().is_none());

        // The parent directory is watched for atomic saves, but sibling files
        // must not trigger a reload of the theme.
        let sibling = path.with_file_name("unrelated.toml");
        std::fs::write(&sibling, "unrelated = true\n").unwrap();
        for i in 0..1_024 {
            std::fs::write(&sibling, format!("unrelated = {i}\n")).unwrap();
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(watcher.poll().is_none());

        // Rewrite with a new primary; expect a reload.
        std::fs::write(&path, "[theme]\nprimary = \"#ff0000\"\n").unwrap();
        let reloaded = poll_until_change(&mut watcher, Duration::from_secs(5))
            .expect("watcher should observe the rewrite");
        assert_eq!(reloaded.theme.primary, Color::Rgb(255, 0, 0));
        assert_eq!(watcher.current().theme.primary, Color::Rgb(255, 0, 0));

        // Write invalid TOML: poll() must not surface it and must keep last good.
        std::fs::write(&path, "this = is [ not valid").unwrap();
        // Give notify a moment, then drain — should never return Some.
        std::thread::sleep(Duration::from_millis(200));
        assert!(watcher.poll().is_none());
        assert_eq!(watcher.current().theme.primary, Color::Rgb(255, 0, 0));

        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
