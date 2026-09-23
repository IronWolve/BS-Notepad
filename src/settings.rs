use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const RECENT_MAX: usize = 15;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub ui_font: String,
    pub body_font: String,
    pub code_font: String,
    pub ui_size: u32,
    pub body_size: u32,
    pub code_size: u32,
    pub line_height: f32,
    pub ligatures: bool,
    pub zoom: f32,
    /// How the document is shown: the rendered page, or its own text.
    pub view_mode: String,
    pub syntax_colour: bool,
    /// "auto" keeps the bar out of the way until the mouse reaches the top.
    pub chrome: String,
    /// "auto" slides the pane in when the mouse reaches the left edge,
    /// "always" keeps it docked, "off" never shows it.
    pub sidebar: String,
    pub sidebar_width: u32,
    pub sidebar_tab: String,
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,
    pub recents: Vec<String>,
    pub last_path: String,
    pub last_scroll: f32,
    /// Above this much code in one document, colouring is skipped up front.
    pub highlight_limit_kb: u32,
    /// Above this size a file opens as plain text instead of being parsed.
    pub plain_text_above_mb: u32,
    pub restore_last_file: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            // Generic stacks: the machine may not have any particular family,
            // and a missing font must degrade rather than break.
            ui_font: "system-ui, -apple-system, Segoe UI, sans-serif".into(),
            body_font: "system-ui, -apple-system, Segoe UI, sans-serif".into(),
            code_font: "ui-monospace, Consolas, monospace".into(),
            ui_size: 13,
            body_size: 16,
            code_size: 14,
            line_height: 1.65,
            ligatures: false,
            zoom: 1.0,
            view_mode: "rendered".into(),
            syntax_colour: true,
            chrome: "auto".into(),
            // Starts bare on purpose: panes and chrome appear when asked for.
            sidebar: "auto".into(),
            sidebar_width: 260,
            sidebar_tab: "files".into(),
            window_width: 1200,
            window_height: 800,
            window_maximized: false,
            recents: Vec::new(),
            last_path: String::new(),
            last_scroll: 0.0,
            highlight_limit_kb: 256,
            plain_text_above_mb: 10,
            restore_last_file: true,
        }
    }
}

impl Settings {
    fn file(root: &Path) -> PathBuf {
        root.join("settings.json")
    }

    pub fn load(root: &Path) -> Self {
        std::fs::read_to_string(Self::file(root))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Written through a temporary file so an interrupted save cannot leave a
    /// truncated settings file behind.
    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        let target = Self::file(root);
        let tmp = target.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, target)
    }

    pub fn remember(&mut self, path: &Path) {
        let text = path.display().to_string();
        self.recents.retain(|p| p != &text);
        self.recents.insert(0, text.clone());
        self.recents.truncate(RECENT_MAX);
        self.last_path = text;
    }

    /// Every field with its default, for the options panel.
    pub fn defaults_json() -> serde_json::Value {
        serde_json::to_value(Self::default()).unwrap_or(serde_json::Value::Null)
    }
}
