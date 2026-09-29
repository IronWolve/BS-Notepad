use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SessionTab {
    pub path: String,
    pub scroll: f32,
    pub editing: bool,
    pub editor_scroll: f64,
    pub selection_start: u64,
    pub selection_end: u64,
}

pub const RECENT_MAX: usize = 15;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    #[serde(skip)]
    pub load_warning: String,
    #[serde(skip)]
    pub blocked_write: bool,
    pub settings_version: u32,
    pub restore_tabs: bool,
    pub saved_tabs: Vec<SessionTab>,
    pub theme_favorites: Vec<String>,
    pub log_retention_days: u32,
    pub backup_retention: u32,
    pub workspace: String,
    pub show_hidden: bool,
    pub minimap: bool,
    pub status_bar: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
    pub close_to_tray: bool,
    pub theme: String,
    pub text_contrast: u32,
    pub tab_shape: String,
    pub tab_highlight: String,
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
    pub remote_images: bool,
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
            load_warning: String::new(),
            blocked_write: false,
            restore_tabs: true,
            saved_tabs: Vec::new(),
            theme_favorites: Vec::new(),
            log_retention_days: 30,
            backup_retention: 3,
            settings_version: 1,
            workspace: String::new(),
            show_hidden: false,
            minimap: true,
            status_bar: true,
            word_wrap: true,
            tab_size: 4,
            close_to_tray: false,
            theme: "dark".into(),
            text_contrast: 0,
            tab_shape: "rounded".into(),
            tab_highlight: "soft".into(),
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
            remote_images: true,
            chrome: "always".into(),
            // Start with discoverable controls; edge reveal remains optional.
            sidebar: "always".into(),
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
        let target = Self::file(root);
        let raw = std::fs::read_to_string(&target).unwrap_or_default();
        let mut settings: Self = match serde_json::from_str(&raw) {
            Ok(settings) => settings,
            Err(_) if target.exists() => {
                let mut recovered = std::fs::read(target.with_extension("json.bak"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
                    .unwrap_or_default();
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let preserved = root.join(format!("settings.corrupt-{}.json", stamp));
                recovered.blocked_write = std::fs::rename(&target, &preserved).is_err();
                recovered.load_warning=if recovered.blocked_write {"Settings could not be read or preserved. Saving preferences is disabled until folder access is restored."}else{"Damaged settings were preserved; preferences were recovered from backup where available."}.into();
                recovered
            }
            Err(_) => Self::default(),
        };
        // Make formerly hidden controls discoverable once when upgrading.
        if !raw.contains("\"settings_version\"") {
            settings.chrome = "always".into();
            settings.sidebar = "always".into();
        }
        settings.normalize();
        settings
    }

    /// Cycle one toolbar control through outline, hidden sidebar, and files.
    pub fn cycle_sidebar(&mut self) {
        if self.sidebar == "off" {
            self.sidebar_tab = "files".into();
            self.sidebar = "always".into();
        } else if self.sidebar_tab == "outline" {
            self.sidebar = "off".into();
        } else {
            self.sidebar_tab = "outline".into();
            self.sidebar = "always".into();
        }
    }

    pub fn normalize(&mut self) {
        self.window_width = self.window_width.clamp(620, 7680);
        self.window_height = self.window_height.clamp(400, 4320);
        self.log_retention_days = self.log_retention_days.clamp(1, 365);
        self.backup_retention = self.backup_retention.clamp(1, 20);
        self.saved_tabs.truncate(40);
        self.theme_favorites
            .retain(|id| crate::theme::builtin().iter().any(|t| t.id == *id));
        self.theme_favorites.sort();
        self.theme_favorites.dedup();
        self.theme = crate::theme::find(&self.theme).id;
        self.text_contrast = self.text_contrast.min(100);
        if !["rounded", "square"].contains(&self.tab_shape.as_str()) {
            self.tab_shape = "rounded".into();
        }
        if !["soft", "line", "glow"].contains(&self.tab_highlight.as_str()) {
            self.tab_highlight = "soft".into();
        }
        self.ui_size = self.ui_size.clamp(10, 28);
        self.body_size = self.body_size.clamp(10, 48);
        self.code_size = self.code_size.clamp(10, 40);
        self.line_height = self.line_height.clamp(1.0, 2.5);
        self.zoom = self.zoom.clamp(0.5, 3.0);
        self.sidebar_width = self.sidebar_width.clamp(180, 640);
        self.tab_size = self.tab_size.clamp(1, 8);
        self.highlight_limit_kb = self.highlight_limit_kb.clamp(1, 4096);
        self.plain_text_above_mb = self.plain_text_above_mb.clamp(1, 32);
        if !["always", "auto"].contains(&self.chrome.as_str()) {
            self.chrome = "always".into();
        }
        if !["always", "auto", "off"].contains(&self.sidebar.as_str()) {
            self.sidebar = "always".into();
        }
        if !["files", "outline"].contains(&self.sidebar_tab.as_str()) {
            self.sidebar_tab = "files".into();
        }
        if !["source", "rendered"].contains(&self.view_mode.as_str()) {
            self.view_mode = "rendered".into();
        }
    }

    /// Written through a temporary file so an interrupted save cannot leave a
    /// truncated settings file behind.
    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        if self.blocked_write {
            return Err(std::io::Error::other(
                "Settings file could not be preserved; check folder permissions.",
            ));
        }
        let target = Self::file(root);
        if let Ok(previous) = std::fs::read(&target) {
            if serde_json::from_slice::<Self>(&previous).is_ok() {
                crate::storage::write_atomic(&target.with_extension("json.bak"), &previous)?;
            }
        }
        crate::storage::write_atomic(&target, &serde_json::to_vec_pretty(self)?)
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
