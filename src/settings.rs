use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ImageView {
    pub scale: f64,
    pub fit: bool,
    pub loupe: bool,
    pub left: f64,
    pub top: f64,
}
impl ImageView {
    pub fn normalize(&mut self) {
        self.scale = if self.scale.is_finite() {
            self.scale.clamp(0.001, 8.0)
        } else {
            1.0
        };
        self.left = if self.left.is_finite() {
            self.left.clamp(0.0, 100_000_000.0)
        } else {
            0.0
        };
        self.top = if self.top.is_finite() {
            self.top.clamp(0.0, 100_000_000.0)
        } else {
            0.0
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SessionTab {
    pub path: String,
    pub scroll: f32,
    pub editing: bool,
    pub editor_scroll: f64,
    pub selection_start: u64,
    pub selection_end: u64,
    pub image_view: Option<ImageView>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct HeadingStyle {
    /// Empty means the current theme's readable body color.
    pub color: String,
    pub shadow: bool,
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
    pub expanded_folders: Vec<String>,
    pub tree_scroll: f64,
    pub tree_filter: String,
    pub show_hidden: bool,
    pub minimap: bool,
    pub status_bar: bool,
    pub word_wrap: bool,
    pub tab_size: u32,
    pub tab_style: String,
    pub continue_lists: bool,
    pub close_to_tray: bool,
    pub theme: String,
    pub text_contrast: u32,
    pub heading_styles: std::collections::BTreeMap<String, HeadingStyle>,
    pub tab_shape: String,
    pub tab_highlight: String,
    pub icon_style: String,
    pub icon_visibility: u32,
    pub ui_font: String,
    pub body_font: String,
    pub code_font: String,
    pub ui_size: u32,
    pub files_size: u32,
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
    pub sidebar_return: String,
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
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
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
            settings_version: 2,
            workspace: String::new(),
            expanded_folders: Vec::new(),
            tree_scroll: 0.0,
            tree_filter: String::new(),
            show_hidden: false,
            minimap: true,
            status_bar: true,
            word_wrap: true,
            tab_size: 4,
            tab_style: "spaces".into(),
            continue_lists: true,
            close_to_tray: false,
            theme: "dark".into(),
            text_contrast: 0,
            heading_styles: std::collections::BTreeMap::new(),
            tab_shape: "rounded".into(),
            tab_highlight: "soft".into(),
            icon_style: "soft".into(),
            icon_visibility: 35,
            // Generic stacks: the machine may not have any particular family,
            // and a missing font must degrade rather than break.
            ui_font: "system-ui, -apple-system, Segoe UI, sans-serif".into(),
            body_font: "system-ui, -apple-system, Segoe UI, sans-serif".into(),
            code_font: "ui-monospace, Consolas, monospace".into(),
            ui_size: 13,
            files_size: 16,
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
            sidebar_return: "always".into(),
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
            extra: Default::default(),
        }
    }
}

impl Settings {
    fn file(root: &Path) -> PathBuf {
        root.join("settings.json")
    }

    fn decode(raw: &str) -> Result<(Self, Vec<String>, u64), serde_json::Error> {
        let value: serde_json::Value =
            serde_json::from_str(raw.strip_prefix('\u{feff}').unwrap_or(raw))?;
        let mut input = value
            .as_object()
            .cloned()
            .ok_or_else(|| serde::de::Error::custom("Preferences must be an object"))?;
        let version = input
            .get("settings_version")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let defaults = serde_json::to_value(Self::default())?;
        let defaults = defaults.as_object().unwrap();
        let mut invalid = Vec::new();
        for (key, value) in input.clone() {
            if !defaults.contains_key(&key) {
                continue;
            }
            let mut probe = serde_json::Map::new();
            probe.insert(key.clone(), value);
            if serde_json::from_value::<Self>(serde_json::Value::Object(probe)).is_err() {
                input.insert(key.clone(), defaults[&key].clone());
                invalid.push(key);
            }
        }
        let settings = serde_json::from_value(serde_json::Value::Object(input))?;
        Ok((settings, invalid, version))
    }

    pub fn load(root: &Path) -> Self {
        let target = Self::file(root);
        let raw = match std::fs::read_to_string(&target) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                return Self {
                    blocked_write: true,
                    load_warning: format!(
                        "Preferences could not be read; saving them is disabled: {}",
                        error
                    ),
                    ..Self::default()
                };
            }
        };
        let (mut settings, invalid, version, recovered) = match Self::decode(&raw) {
            Ok((settings, invalid, version)) => (settings, invalid, version, false),
            Err(_) => {
                let backup = std::fs::read_to_string(target.with_extension("json.bak"))
                    .ok()
                    .and_then(|raw| Self::decode(&raw).ok());
                let (settings, invalid, version) =
                    backup.unwrap_or_else(|| (Self::default(), Vec::new(), 2));
                (settings, invalid, version, true)
            }
        };
        if version > 2 {
            settings.blocked_write = true;
            settings.load_warning="These preferences were written by a newer app. They will not be overwritten by this version.".into();
        } else if recovered || !invalid.is_empty() {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let preserved = root.join(format!(
                "settings.corrupt-{}-{}.json",
                std::process::id(),
                stamp
            ));
            settings.blocked_write = std::fs::rename(&target, &preserved).is_err();
            settings.load_warning = if settings.blocked_write {
                "Preferences could not be preserved. Saving them is disabled until folder access is restored.".into()
            } else if recovered {
                "Damaged preferences were preserved; valid backup choices were restored where available.".into()
            } else {
                format!(
                    "Invalid preference values were preserved; only these fields were reset: {}",
                    invalid.join(", ")
                )
            };
        }
        if version == 0 {
            settings.chrome = "always".into();
            settings.sidebar = "always".into();
        }
        if version <= 2 {
            settings.settings_version = 2;
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
        if !["auto", "always"].contains(&self.sidebar_return.as_str()) {
            self.sidebar_return = "always".into();
        }
        if !self.line_height.is_finite() {
            self.line_height = 1.65;
        }
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        self.last_scroll = if self.last_scroll.is_finite() {
            self.last_scroll.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.window_width = self.window_width.clamp(620, 7680);
        self.window_height = self.window_height.clamp(400, 4320);
        self.log_retention_days = self.log_retention_days.clamp(1, 365);
        self.backup_retention = self.backup_retention.clamp(1, 20);
        self.expanded_folders
            .retain(|path| !path.is_empty() && path.len() <= 131072);
        self.expanded_folders.sort();
        self.expanded_folders.dedup();
        self.tree_scroll = if self.tree_scroll.is_finite() {
            self.tree_scroll.clamp(0.0, 100_000_000.0)
        } else {
            0.0
        };
        self.icon_visibility = self.icon_visibility.clamp(20, 100);
        if ![
            "soft", "crisp", "fine", "bold", "square", "rounded", "duotone", "color", "pastel",
            "contrast",
        ]
        .contains(&self.icon_style.as_str())
        {
            self.icon_style = "soft".into();
        }
        for tab in &mut self.saved_tabs {
            tab.scroll = if tab.scroll.is_finite() {
                tab.scroll.clamp(0.0, 1.0)
            } else {
                0.0
            };
            tab.editor_scroll = if tab.editor_scroll.is_finite() {
                tab.editor_scroll.clamp(0.0, 100_000_000.0)
            } else {
                0.0
            };
            if let Some(view) = &mut tab.image_view {
                view.normalize();
            }
        }
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
        self.files_size = self.files_size.clamp(12, 48);
        self.body_size = self.body_size.clamp(10, 48);
        self.code_size = self.code_size.clamp(10, 40);
        self.line_height = self.line_height.clamp(1.0, 2.5);
        self.zoom = self.zoom.clamp(0.5, 3.0);
        self.sidebar_width = self.sidebar_width.clamp(180, 640);
        self.tab_size = self.tab_size.clamp(1, 8);
        if !["spaces", "tabs"].contains(&self.tab_style.as_str()) {
            self.tab_style = "spaces".into();
        }
        let themes = crate::theme::builtin();
        self.heading_styles.retain(|id, style| {
            if !(style.color.len() == 7
                && style.color.starts_with('#')
                && style.color.as_bytes()[1..]
                    .iter()
                    .all(u8::is_ascii_hexdigit))
            {
                style.color.clear();
            }
            style.color.make_ascii_lowercase();
            themes.iter().any(|theme| theme.id == *id) && (!style.color.is_empty() || style.shadow)
        });
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
                crate::storage::write_private_atomic(
                    &target.with_extension("json.bak"),
                    &previous,
                )?;
            }
        }
        crate::storage::write_private_atomic(&target, &serde_json::to_vec_pretty(self)?)
    }

    pub fn remember(&mut self, path: &Path) {
        let text = path.display().to_string();
        self.recents.retain(|p| p != &text);
        self.recents.insert(0, text.clone());
        self.recents.truncate(RECENT_MAX);
        self.last_path = text;
    }

    pub fn editable_key(key: &str) -> bool {
        matches!(
            key,
            "theme"
                | "theme_favorites"
                | "text_contrast"
                | "heading_styles"
                | "tab_shape"
                | "tab_highlight"
                | "icon_style"
                | "icon_visibility"
                | "chrome"
                | "zoom"
                | "ui_font"
                | "body_font"
                | "code_font"
                | "ui_size"
                | "files_size"
                | "body_size"
                | "code_size"
                | "line_height"
                | "ligatures"
                | "sidebar"
                | "sidebar_width"
                | "sidebar_tab"
                | "show_hidden"
                | "restore_last_file"
                | "restore_tabs"
                | "close_to_tray"
                | "log_retention_days"
                | "backup_retention"
                | "word_wrap"
                | "tab_size"
                | "tab_style"
                | "continue_lists"
                | "minimap"
                | "status_bar"
                | "view_mode"
                | "syntax_colour"
                | "highlight_limit_kb"
                | "plain_text_above_mb"
                | "remote_images"
        )
    }

    /// Every field with its default, for the options panel.
    pub fn defaults_json() -> serde_json::Value {
        serde_json::to_value(Self::default()).unwrap_or(serde_json::Value::Null)
    }
}
