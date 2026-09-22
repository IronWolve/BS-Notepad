use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub body_font: String,
    pub code_font: String,
    pub body_size: u32,
    pub code_size: u32,
    pub line_height: f32,
    pub sidebar_visible: bool,
    pub sidebar_width: u32,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".into(),
            // Deliberately generic stacks: the machine may not have any
            // particular family, and a missing font must degrade, not break.
            body_font: "system-ui, -apple-system, Segoe UI, sans-serif".into(),
            code_font: "ui-monospace, Consolas, monospace".into(),
            body_size: 16,
            code_size: 14,
            line_height: 1.65,
            sidebar_visible: true,
            sidebar_width: 260,
            window_width: 1200,
            window_height: 800,
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
}
