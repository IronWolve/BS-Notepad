// No console window on Windows: this is a windowed app, and its log goes to a
// file beside the binary.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod documents;
mod fonts;
mod icon;
mod instance;
mod log;
mod render;
mod root;
mod settings;
#[cfg(test)]
mod tests;
mod theme;
mod tray;
mod tree;
mod ui;

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use serde_json::json;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Window, WindowBuilder};
use wry::http::{Request, Response};
use wry::{WebView, WebViewBuilder};

use documents::{Document, Documents};
use render::Renderer;
use settings::Settings;

fn welcome() -> String {
    format!("# {}\n\nYour notes, Markdown and source files in one place.\n\n- **New** starts a note (Ctrl+N).\n- **Open folder** fills the file browser.\n- **Edit** switches to the editor (Ctrl+E).\n- **Options** has themes, fonts and workspace preferences (Ctrl+,).\n\nDrag a file here to open it.\n", root::app_name())
}

#[derive(Debug)]
enum UserEvent {
    /// A message from the page.
    Page(String),
    Dropped(PathBuf),
    #[cfg(target_os = "windows")]
    Tray(String),
    /// A second launch handed us a path instead of starting its own window.
    Handoff(Option<String>),
}

struct App {
    root: PathBuf,
    smoke_started: Option<Instant>,
    settings: Settings,
    documents: Documents,
    renderer: Renderer,
    fonts: Vec<fonts::FontFamily>,
    tray: tray::SystemTray,
    tree_dir: PathBuf,
    window: Window,
    webview: WebView,
}

impl std::ops::Deref for App {
    type Target = Document;
    fn deref(&self) -> &Document {
        self.documents.current()
    }
}
impl std::ops::DerefMut for App {
    fn deref_mut(&mut self) -> &mut Document {
        self.documents.current_mut()
    }
}

fn open_externally(url: &str) {
    if !url.starts_with("https://") && !url.starts_with("http://") && !url.starts_with("mailto:") {
        return;
    }
    // Deliberately not navigating the window: an external link belongs in the
    // browser, and the document view should never leave the document.
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = result {
        log::line(&format!("could not open {}: {}", url, e));
    }
}

impl App {
    fn run_js(&self, script: String) {
        if let Err(e) = self.webview.evaluate_script(&script) {
            log::line(&format!("script failed: {}", e));
        }
    }

    fn theme(&self) -> theme::Theme {
        let mut t = theme::find(&self.settings.theme);
        // Nothing may end up unreadable, whatever the combination.
        t.fg = theme::guard(&t.fg, &t.bg, 4.5);
        t.dim = theme::guard(&t.dim, &t.panel, 4.5);
        t.link = theme::guard(&t.link, &t.bg, 4.5);
        t
    }

    fn send_init(&self) {
        let missing: Vec<&str> = [
            ("ui", self.settings.ui_font.as_str()),
            ("body", self.settings.body_font.as_str()),
            ("code", self.settings.code_font.as_str()),
        ]
        .iter()
        .filter(|(_, family)| !fonts::has_family(&self.fonts, family))
        .map(|(_, family)| *family)
        .collect();

        let payload = json!({
            "name": root::app_name(),
            "logoUrl": assets::brand_url(),
            "maximized": self.window.is_maximized(),
            "githubUrl": env!("CARGO_PKG_HOMEPAGE"),
            "version": env!("CARGO_PKG_VERSION"),
            "trayAvailable": self.tray.available(),
            "settings": self.settings,
            "defaults": Settings::defaults_json(),
            "themes": theme::builtin(),
            "theme": self.theme(),
            "fonts": self.fonts,
            "missingFonts": missing,
        });
        self.run_js(format!("window.app.init({});", payload));
    }

    fn send_settings(&self) {
        let payload = json!({ "settings": self.settings });
        self.run_js(format!("window.app.applySettings({}.settings);", payload));
        let t = self.theme();
        self.run_js(format!(
            "window.app.applyTheme({});",
            serde_json::to_string(&t).unwrap_or_else(|_| "{}".into())
        ));
    }

    fn send_recents(&self) {
        self.run_js(format!(
            "window.app.setRecents({});",
            serde_json::to_string(&self.settings.recents).unwrap_or_else(|_| "[]".into())
        ));
    }

    fn send_tree(&mut self, dir: PathBuf) {
        match tree::list(&dir, self.settings.show_hidden) {
            Ok(entries) => {
                let payload = json!({ "dir": dir.display().to_string(), "entries": entries,
                    "parent": dir.parent().is_some() });
                self.settings.workspace = dir.display().to_string();
                self.tree_dir = dir;
                self.run_js(format!("window.app.setTree({});", payload));
                let _ = self.settings.save(&self.root);
            }
            Err(e) => self.notify(&format!("Cannot read folder: {}", e)),
        }
    }

    fn notify(&self, message: &str) {
        self.run_js(format!("window.app.note({});", json!(message)));
    }

    fn show(&self) {
        self.window.set_visible(true);
        self.window.set_minimized(false);
        self.window.set_focus();
    }

    fn send_tabs(&self) {
        let tabs: Vec<_> = self.documents.tabs.iter().map(|doc| json!({
            "id":doc.id, "name":doc.name(), "path":doc.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(), "dirty":doc.dirty
        })).collect();
        self.run_js(format!(
            "window.app.setTabs({});",
            json!({"active":self.id,"tabs":tabs})
        ));
    }

    fn activate_tab(&mut self, id: u64) {
        if self.documents.activate(id) {
            assets::set_scope(self.path.as_deref().and_then(Path::parent));
            if let Some(path) = self.path.clone() {
                self.settings.remember(&path);
            }
            self.render_current(self.scroll);
        }
    }

    fn new_note(&mut self) {
        let mut doc = Document::new(None, String::new());
        doc.editing = true;
        self.documents.insert(doc);
        assets::set_scope(None);
        self.render_current(0.0);
    }

    fn close_tab(&mut self, id: u64) -> bool {
        if !self.documents.activate(id) {
            return true;
        }
        self.render_current(self.scroll);
        if !self.may_close() {
            return false;
        }
        self.documents.remove(id);
        self.activate_tab(self.id);
        true
    }

    fn close_window(&mut self, control_flow: &mut ControlFlow) {
        if self.settings.close_to_tray && self.tray.available() {
            self.remember_window();
            let _ = self.settings.save(&self.root);
            self.window.set_visible(false);
        } else {
            self.quit(control_flow);
        }
    }

    fn quit(&mut self, control_flow: &mut ControlFlow) {
        self.show();
        let original = self.id;
        let dirty: Vec<u64> = self
            .documents
            .tabs
            .iter()
            .filter(|d| d.dirty)
            .map(|d| d.id)
            .collect();
        for id in dirty {
            self.activate_tab(id);
            if !self.may_close() {
                return;
            }
        }
        self.activate_tab(original);
        self.remember_window();
        let _ = self.settings.save(&self.root);
        instance::release(&self.root);
        *control_flow = ControlFlow::Exit;
    }

    fn render_current(&mut self, scroll: f32) {
        let t = self.theme();
        let document = self
            .renderer
            .render(self.path.as_deref(), &self.source, &self.settings, &t);
        let name = self.name();

        self.send_tabs();
        let payload = json!({
            "tab": self.id,
            "revision": self.edit_revision,
            "editing": self.editing,
            "editorScroll": self.editor_scroll,
            "selectionStart": self.selection_start,
            "selectionEnd": self.selection_end,
            "dirty": self.dirty,
            "html": document.html,
            "outline": document.outline,
            "note": document.note,
            "frontMatter": document.front_matter,
            "name": name,
            "path": self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            "scroll": scroll,
        });
        self.run_js(format!("window.app.setDocument({});", payload));
        self.run_js(format!(
            "window.app.setEditorText({});",
            json!({
                "tab":self.id,"revision":self.edit_revision,"text":self.source
            })
        ));

        let title = match &self.path {
            Some(_) => format!("{} - {}", name, root::app_name()),
            None => root::app_name().to_string(),
        };
        self.window
            .set_title(&format!("{}{}", if self.dirty { "* " } else { "" }, title));
    }

    fn open(&mut self, path: PathBuf, new_tab: bool) {
        let path = path.canonicalize().unwrap_or(path);
        if let Some(id) = self.documents.find_path(&path) {
            self.activate_tab(id);
            return;
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                if !new_tab && !self.may_close() {
                    return;
                }
                log::line(&format!("open {} ({} bytes)", path.display(), text.len()));
                let document = Document::new(Some(path.clone()), text);
                if new_tab {
                    self.documents.insert(document);
                } else {
                    self.documents.replace(document);
                }
                assets::set_scope(path.parent());
                self.settings.remember(&path);
                self.render_current(0.0);
                self.send_recents();
                if let Some(parent) = path.parent() {
                    if !path.starts_with(&self.tree_dir) {
                        self.send_tree(parent.to_path_buf());
                    }
                }
            }
            Err(e) => self.notify(&format!("Cannot open {}: {}", path.display(), e)),
        }
    }

    fn pick_and_open(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "mdown", "mkd", "mkdn"])
            .add_filter(
                "Text and source",
                &[
                    "txt", "rs", "py", "js", "ts", "go", "c", "h", "cpp", "json", "toml", "yaml",
                    "yml", "sh",
                ],
            )
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(picked) = dialog.pick_file() {
            self.open(picked, true);
        }
    }

    /// Writes through a temporary file in the same folder, then renames: an
    /// interrupted save cannot leave the note truncated.
    fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
        use std::io::Write;
        let filename = path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "No filename"))?;
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = path.with_file_name(format!(
            ".{}.{}.{}.tmp",
            filename.to_string_lossy(),
            std::process::id(),
            nonce
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        let result = (|| {
            file.write_all(text.as_bytes())?;
            if let Ok(meta) = std::fs::metadata(path) {
                file.set_permissions(meta.permissions())?;
            }
            file.sync_all()?;
            drop(file);
            std::fs::rename(&tmp, path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        result
    }

    fn save(&mut self, text: String, save_as: bool) -> bool {
        let path = match self.path.clone().filter(|_| !save_as) {
            Some(p) => p,
            None => {
                let picked = rfd::FileDialog::new()
                    .set_directory(&self.tree_dir)
                    .set_file_name(
                        self.path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|p| p.to_str())
                            .unwrap_or("untitled.md"),
                    )
                    .add_filter("All files", &["*"])
                    .add_filter("Markdown", &["md"])
                    .save_file();
                match picked {
                    Some(p) => p,
                    None => return false,
                }
            }
        };

        let path = path.canonicalize().unwrap_or(path);
        if let Some(id) = self.documents.find_path(&path) {
            if id != self.id {
                self.notify("That file is already open in another tab. Save from that tab, or choose another filename.");
                return false;
            }
        }
        // If the file moved underneath us, say so rather than overwriting.
        let current = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if self.path.as_ref() == Some(&path) && self.seen_mtime.is_some() {
            if current != self.seen_mtime {
                let choice = rfd::MessageDialog::new()
                    .set_title("Changed on disk")
                    .set_description(
                        "This file changed on disk since it was opened.\n\nOverwrite it?",
                    )
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if choice != rfd::MessageDialogResult::Yes {
                    self.run_js("window.app.note('save cancelled - file changed on disk');".into());
                    return false;
                }
            }
        }

        let new_path = self.path.as_ref() != Some(&path);
        match Self::write_atomically(&path, &text) {
            Ok(()) => {
                self.saved_source = text.clone();
                self.source = text;
                assets::set_scope(path.parent());
                self.path = Some(path.clone());
                self.seen_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.dirty = false;
                self.settings.remember(&path);
                log::line(&format!("saved {}", path.display()));
                self.render_current(0.0);
                self.send_recents();
                if new_path {
                    self.send_tree(self.tree_dir.clone());
                }
                self.run_js("window.app.note('Saved');".into());
                true
            }
            Err(e) => {
                log::line(&format!("save failed {}: {}", path.display(), e));
                self.run_js(format!(
                    "window.app.note({});",
                    serde_json::to_string(&format!("save failed: {}", e)).unwrap_or_default()
                ));
                false
            }
        }
    }

    fn apply_setting(&mut self, key: &str, value: serde_json::Value) {
        let mut current = serde_json::to_value(&self.settings).unwrap_or(json!({}));
        if let Some(map) = current.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        if let Ok(mut updated) = serde_json::from_value::<Settings>(current) {
            updated.normalize();
            let rerender = matches!(
                key,
                "theme"
                    | "highlight_limit_kb"
                    | "plain_text_above_mb"
                    | "view_mode"
                    | "syntax_colour"
            );
            self.settings = updated;
            if key == "show_hidden" {
                self.send_tree(self.tree_dir.clone());
            }
            self.send_settings();
            if rerender {
                self.render_current(self.scroll);
            }
            let _ = self.settings.save(&self.root);
        }
    }

    fn handle(&mut self, message: &str, control_flow: &mut ControlFlow) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
            return;
        };
        let command = value.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        if command != "scroll" && command != "edit" {
            log::line(&format!("page: {}", command));
        }

        if let Some(id) = value.get("fromTab").and_then(|v| v.as_u64()) {
            if let Some(view) = value.get("view") {
                if let Some(doc) = self.documents.get_mut(id) {
                    if let Some(v) = view.get("editing").and_then(|v| v.as_bool()) {
                        doc.editing = v;
                    }
                    if let Some(v) = view.get("scroll").and_then(|v| v.as_f64()) {
                        doc.scroll = v as f32;
                    }
                    if let Some(v) = view.get("editorScroll").and_then(|v| v.as_f64()) {
                        doc.editor_scroll = v;
                    }
                    if let Some(v) = view.get("selectionStart").and_then(|v| v.as_u64()) {
                        doc.selection_start = v;
                    }
                    if let Some(v) = view.get("selectionEnd").and_then(|v| v.as_u64()) {
                        doc.selection_end = v;
                    }
                }
            }
        }
        match command {
            "ready" => {
                self.send_init();
                self.send_recents();
                let dir = self.tree_dir.clone();
                self.send_tree(dir);
                let scroll = self.settings.last_scroll;
                self.render_current(scroll);
                if self.smoke_started.is_some() {
                    self.run_js(include_str!("smoke.js").into());
                }
            }
            "smokeReady" if self.smoke_started.is_some() => {
                if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                    println!(
                        "READY_MS={}",
                        self.smoke_started.unwrap().elapsed().as_millis()
                    );
                } else {
                    eprintln!(
                        "UI smoke test failed: {}",
                        value
                            .get("error")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown error")
                    );
                }
                let _ = self.settings.save(&self.root);
                instance::release(&self.root);
                *control_flow = ControlFlow::Exit;
            }
            "windowMinimize" => self.window.set_minimized(true),
            "windowMaximize" => self.window.set_maximized(!self.window.is_maximized()),
            "windowDrag" => {
                let _ = self.window.drag_window();
            }
            "windowResize" => {
                use tao::window::ResizeDirection as D;
                let direction = match value.get("direction").and_then(|v| v.as_str()) {
                    Some("n") => Some(D::North),
                    Some("s") => Some(D::South),
                    Some("e") => Some(D::East),
                    Some("w") => Some(D::West),
                    Some("ne") => Some(D::NorthEast),
                    Some("nw") => Some(D::NorthWest),
                    Some("se") => Some(D::SouthEast),
                    Some("sw") => Some(D::SouthWest),
                    _ => None,
                };
                if let Some(direction) = direction {
                    let _ = self.window.drag_resize_window(direction);
                }
            }
            "closeWindow" => self.close_window(control_flow),
            "new" => self.new_note(),
            "activateTab" => {
                if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                    self.activate_tab(id);
                }
            }
            "closeTab" => {
                if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                    self.close_tab(id);
                }
            }
            "closeOtherTabs" => {
                if let Some(keep) = value.get("id").and_then(|v| v.as_u64()) {
                    let ids: Vec<u64> = self
                        .documents
                        .tabs
                        .iter()
                        .filter(|d| d.id != keep)
                        .map(|d| d.id)
                        .collect();
                    let mut completed = true;
                    for id in ids {
                        if !self.close_tab(id) {
                            completed = false;
                            break;
                        }
                    }
                    if completed {
                        self.activate_tab(keep);
                    }
                }
            }
            "viewState" => {}
            "edit" => {
                if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
                    let id = value
                        .get("fromTab")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(self.id);
                    let mut changed = false;
                    if let Some(doc) = self.documents.get_mut(id) {
                        let before = doc.dirty;
                        let revision = value
                            .get("revision")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(doc.edit_revision + 1);
                        if revision >= doc.edit_revision {
                            doc.edit(text.to_owned());
                            doc.edit_revision = revision;
                        }
                        changed = before != doc.dirty;
                    }
                    if changed {
                        self.send_tabs();
                    }
                    if id == self.id {
                        self.run_js(format!("window.app.setDirty({});", self.dirty));
                        self.window.set_title(&format!(
                            "{}{} - {}",
                            if self.dirty { "* " } else { "" },
                            self.name(),
                            root::app_name()
                        ));
                    }
                }
            }
            "preview" => self.render_current(self.scroll),
            "open" => self.pick_and_open(),
            "openFolder" => {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_directory(&self.tree_dir)
                    .pick_folder()
                {
                    self.apply_setting("sidebar", json!("always"));
                    self.apply_setting("sidebar_tab", json!("files"));
                    self.send_tree(dir);
                }
            }
            "reload" => {
                if let Some(path) = self.path.clone() {
                    if self.may_close() {
                        match std::fs::read_to_string(&path) {
                            Ok(text) => {
                                let mut doc = Document::new(Some(path), text);
                                doc.editing = self.editing;
                                doc.scroll = self.scroll;
                                self.documents.replace(doc);
                                self.render_current(self.scroll);
                            }
                            Err(e) => self.notify(&format!("Cannot reload: {}", e)),
                        }
                    }
                } else {
                    self.notify("Save this note before reloading it.");
                }
            }
            "refreshTree" => self.send_tree(self.tree_dir.clone()),
            "openPath" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    self.open(
                        PathBuf::from(path),
                        value
                            .get("newTab")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                    );
                }
            }
            "workspacePath" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    self.send_tree(PathBuf::from(path));
                }
            }
            "showFolder" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    let path = Path::new(path);
                    let folder = if path.is_dir() {
                        path
                    } else {
                        path.parent().unwrap_or(path)
                    };
                    let command = if cfg!(target_os = "windows") {
                        "explorer.exe"
                    } else if cfg!(target_os = "macos") {
                        "open"
                    } else {
                        "xdg-open"
                    };
                    let folder_text = folder.to_string_lossy();
                    let folder_text = if cfg!(target_os = "windows") {
                        if let Some(rest) = folder_text.strip_prefix(r"\\?\UNC\") {
                            format!(r"\\{}", rest)
                        } else {
                            folder_text
                                .strip_prefix(r"\\?\")
                                .unwrap_or(&folder_text)
                                .to_string()
                        }
                    } else {
                        folder_text.into_owned()
                    };
                    if let Err(e) = std::process::Command::new(command).arg(folder_text).spawn() {
                        self.notify(&format!("Cannot open folder: {}", e));
                    }
                }
            }
            "expand" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    let result = tree::list(Path::new(path), self.settings.show_hidden);
                    let (entries, error) = match result {
                        Ok(entries) => (entries, None),
                        Err(e) => (Vec::new(), Some(e.to_string())),
                    };
                    self.run_js(format!("window.app.setEntries({});", json!({
                        "path": path, "request": value.get("request"), "entries": entries, "error": error
                    })));
                }
            }
            "treeUp" => {
                let parent = self.tree_dir.parent().map(PathBuf::from);
                if let Some(dir) = parent {
                    self.send_tree(dir);
                }
            }
            "save" | "saveAs" => {
                if value
                    .get("fromTab")
                    .and_then(|v| v.as_u64())
                    .is_some_and(|id| id != self.id)
                {
                    self.notify("Select the document tab before saving it.");
                    return;
                }
                let text = value
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                self.save(text, command == "saveAs");
            }
            "wantSource" => {
                self.run_js(format!(
                    "window.app.setEditorText({});",
                    serde_json::to_string(&self.source).unwrap_or_else(|_| "\"\"".into())
                ));
            }
            "external" => {
                if let Some(url) = value.get("url").and_then(|u| u.as_str()) {
                    open_externally(url);
                }
            }
            "setting" => {
                if let Some(key) = value.get("key").and_then(|k| k.as_str()) {
                    let new_value = value.get("value").cloned().unwrap_or(json!(null));
                    self.apply_setting(key, new_value);
                }
            }
            "resetSettings" => {
                let old = self.settings.clone();
                self.settings = Settings::default();
                self.settings.recents = old.recents;
                self.settings.last_path = old.last_path;
                self.settings.workspace = old.workspace;
                self.settings.last_scroll = old.last_scroll;
                self.send_tree(self.tree_dir.clone());
                self.send_settings();
                self.render_current(0.0);
                let _ = self.settings.save(&self.root);
            }
            "scroll" => {
                if let Some(v) = value.get("value").and_then(|v| v.as_f64()) {
                    self.scroll = v as f32;
                    self.settings.last_scroll = v as f32;
                }
            }
            "quit" => self.quit(control_flow),
            _ => {}
        }
    }

    /// Asks before losing edits.
    fn may_close(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let choice = rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(format!(
                "Save changes to {} before continuing?",
                self.name()
            ))
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                "Save".into(),
                "Discard".into(),
                "Cancel".into(),
            ))
            .show();
        match choice {
            rfd::MessageDialogResult::Yes => self.save(self.source.clone(), false),
            rfd::MessageDialogResult::No => true,
            rfd::MessageDialogResult::Custom(label) if label == "Save" => {
                self.save(self.source.clone(), false)
            }
            rfd::MessageDialogResult::Custom(label) if label == "Discard" => true,
            _ => false,
        }
    }

    fn remember_window(&mut self) {
        let size = self
            .window
            .inner_size()
            .to_logical::<u32>(self.window.scale_factor());
        if !self.window.is_maximized()
            && !self.window.is_minimized()
            && size.width > 200
            && size.height > 200
        {
            self.settings.window_width = size.width;
            self.settings.window_height = size.height;
        }
        self.settings.window_maximized = self.window.is_maximized();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let root = root::app_root();
    log::init(&root);
    log::line(&format!("start root={}", root.display()));

    let argument = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .map(|p| p.canonicalize().unwrap_or(p));

    // A copy already running takes the file and raises its own window.
    if instance::hand_off(&root, argument.as_deref()) {
        log::line("handed the file to the running copy");
        return Ok(());
    }

    let mut settings = Settings::load(&root);
    let renderer = Renderer::new();
    let font_list = fonts::families();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let handoff_proxy = proxy.clone();
    instance::listen(&root, move |path| {
        let _ = handoff_proxy.send_event(UserEvent::Handoff(path));
    });

    let window = WindowBuilder::new()
        .with_title(root::app_name())
        .with_decorations(false)
        .with_window_icon(tao::window::Icon::from_rgba(icon::rgba(64), 64, 64).ok())
        .with_min_inner_size(tao::dpi::LogicalSize::new(620.0, 400.0))
        .with_inner_size(tao::dpi::LogicalSize::new(
            settings.window_width as f64,
            settings.window_height as f64,
        ))
        .with_maximized(settings.window_maximized)
        .build(&event_loop)?;

    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;
        use windows_sys::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        };
        window.set_undecorated_shadow(true);
        let preference = DWMWCP_ROUND;
        // The window handle is owned by this live window; the API copies the value.
        unsafe {
            DwmSetWindowAttribute(
                window.hwnd() as _,
                DWMWA_WINDOW_CORNER_PREFERENCE as _,
                (&preference as *const i32).cast(),
                std::mem::size_of_val(&preference) as _,
            );
        }
    }
    let tray = tray::SystemTray::new(proxy.clone());

    let drop_proxy = proxy.clone();
    let ipc_proxy = proxy.clone();
    let builder = WebViewBuilder::new()
        .with_html(ui::shell())
        .with_drag_drop_handler(move |event| {
            if let wry::DragDropEvent::Drop { paths, .. } = event {
                if let Some(path) = paths.first() {
                    let _ = drop_proxy.send_event(UserEvent::Dropped(path.clone()));
                }
            }
            true
        })
        .with_ipc_handler(move |request: Request<String>| {
            let _ = ipc_proxy.send_event(UserEvent::Page(request.body().to_string()));
        })
        .with_custom_protocol("asset".into(), move |_id, request: Request<Vec<u8>>| {
            let (body, mime, status) = assets::serve(&request.uri().to_string());
            Response::builder()
                .status(status)
                .header("Content-Type", mime)
                .header("Access-Control-Allow-Origin", "*")
                .body(std::borrow::Cow::from(body))
                .unwrap_or_else(|_| Response::new(std::borrow::Cow::from(Vec::new())))
        });

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().ok_or("no gtk container")?)?
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    // Which file to show: the argument, else the last one if it still exists.
    let mut initial = argument.clone().or_else(|| {
        if settings.restore_last_file && !settings.last_path.is_empty() {
            let candidate = PathBuf::from(&settings.last_path);
            candidate.exists().then_some(candidate)
        } else {
            None
        }
    });
    let tree_dir = PathBuf::from(&settings.workspace)
        .is_dir()
        .then(|| PathBuf::from(&settings.workspace))
        .or_else(|| initial.as_ref().and_then(|p| p.parent().map(PathBuf::from)))
        .unwrap_or_else(|| root.clone());

    let source = match &initial {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(text) => text,
            Err(e) => {
                log::line(&format!("initial open failed: {}", e));
                initial = None;
                welcome()
            }
        },
        None => welcome(),
    };
    if let Some(p) = &initial {
        assets::set_scope(p.parent());
        settings.remember(p);
    }

    let mut app = App {
        documents: Documents::new(Document::new(initial, source)),
        tray,
        tree_dir,
        root: root.clone(),
        smoke_started: std::env::var_os("EXIT_WHEN_READY").map(|_| started),
        settings,
        renderer,
        fonts: font_list,
        window,
        webview,
    };

    log::line(&format!(
        "window ready in {} ms",
        started.elapsed().as_millis()
    ));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Page(message)) => app.handle(&message, control_flow),
            Event::UserEvent(UserEvent::Dropped(path)) => app.open(path, true),
            Event::UserEvent(UserEvent::Handoff(path)) => {
                app.show();
                if let Some(path) = path {
                    app.open(PathBuf::from(path), true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::DroppedFile(path),
                ..
            } => app.open(path, true),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                app.close_window(control_flow);
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                app.run_js(format!(
                    "window.app && window.app.windowState({});",
                    app.window.is_maximized()
                ));
            }
            #[cfg(target_os = "windows")]
            Event::UserEvent(UserEvent::Tray(command)) => {
                app.show();
                match command.as_str() {
                    "new" => app.new_note(),
                    "open" => app.pick_and_open(),
                    "quit" => app.quit(control_flow),
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
