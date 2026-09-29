// No console window on Windows: this is a windowed app, and its log goes to a
// file beside the binary.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod documents;
mod fonts;
mod formatting;
mod icon;
mod instance;
mod jobs;
mod log;
#[cfg(target_os = "macos")]
mod mac_menu;
mod maintenance;
mod recovery;
mod render;
mod root;
mod settings;
mod storage;
#[cfg(test)]
mod tests;
mod theme;
mod tray;
mod tree;
mod ui;

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::json;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Window, WindowBuilder};
use wry::http::Request;
use wry::{WebView, WebViewBuilder};

use documents::{Document, Documents};
#[cfg(test)]
use render::Renderer;
use settings::Settings;

fn navigation_allowed(ready: bool, url: &str) -> bool {
    !ready
        || url == "about:blank"
        || url.starts_with("about:blank#")
        || url == "http://localhost/"
        || url.starts_with("http://localhost/#")
}

fn welcome() -> String {
    format!("# {}\n\nYour notes, Markdown and source files in one place.\n\n- **New** starts a note (Ctrl+N).\n- **Open folder** fills the file browser.\n- **Edit** switches to the editor (Ctrl+E).\n- **Options** has themes, fonts and workspace preferences (Ctrl+,).\n\nDrag a file here to open it.\n", root::app_name())
}

enum UserEvent {
    /// A message from the page.
    Page(String),
    #[cfg(target_os = "macos")]
    MacCommand(String),
    BrowserLoaded,
    Rendered {
        generation: u64,
        tab: u64,
        revision: u64,
        key: String,
        document: render::Document,
    },
    Loaded {
        task: jobs::OpenTask,
        result: Result<storage::Loaded, String>,
    },
    TreeLoaded {
        path: PathBuf,
        request: serde_json::Value,
        root: bool,
        serial: u64,
        result: Result<Vec<tree::Entry>, String>,
    },
    DiskChecked {
        tab: u64,
        revision: u64,
        changed: bool,
    },
    RecoveryError(String),
    Dropped(PathBuf),
    #[cfg(target_os = "windows")]
    Tray(String),
    /// A second launch handed us a path instead of starting its own window.
    Handoff(Vec<String>),
}

struct App {
    root: PathBuf,
    #[cfg(target_os = "macos")]
    _menu: muda::Menu,
    _instance: instance::Guard,
    smoke_started: Option<Instant>,
    ui_ready: bool,
    boot_started: Instant,
    boot_probe: bool,
    navigation_ready: std::rc::Rc<std::cell::Cell<bool>>,
    settings: Settings,
    preview_theme: Option<String>,
    documents: Documents,
    render_worker: jobs::RenderWorker,
    io: jobs::IoWorker,
    generation: u64,
    tree_serial: u64,
    pending_fragment: String,
    closed_tabs: Vec<Document>,
    session_restore: Vec<settings::SessionTab>,
    startup_paths: Vec<PathBuf>,
    startup_pending: usize,
    startup_target: Option<PathBuf>,
    recovery_checked: bool,
    recovery_pending: std::collections::HashSet<u64>,
    recovery_flush: Instant,
    last_disk_check: Instant,
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
        let mut t = theme::find(
            self.preview_theme
                .as_deref()
                .unwrap_or(&self.settings.theme),
        );
        // Nothing may end up unreadable, whatever the combination.
        t.text_contrast = self.settings.text_contrast;
        t.fg = theme::strengthen(&theme::guard(&t.fg, &t.bg, 4.5), &t.bg, t.text_contrast);
        t.dim = theme::guard(&t.dim, &t.panel, 4.5);
        t.link = theme::strengthen(&theme::guard(&t.link, &t.bg, 4.5), &t.bg, t.text_contrast);
        t
    }

    fn persist_settings(&mut self) {
        if self.startup_pending == 0 {
            self.settings.saved_tabs = self
                .documents
                .tabs
                .iter()
                .filter_map(|d| {
                    d.path.as_ref().map(|p| settings::SessionTab {
                        path: p.to_string_lossy().into_owned(),
                        scroll: d.scroll,
                        editing: d.editing,
                        editor_scroll: d.editor_scroll,
                        selection_start: d.selection_start,
                        selection_end: d.selection_end,
                        image_view: d.image_view.clone(),
                    })
                })
                .collect();
        }
        let result = self.settings.save(&self.root);
        if let Err(error) = &result {
            log::line(&format!("Preferences save failed: {}", error));
        }
        self.run_js(format!("window.app.settingsStatus({});",json!({"ok":result.is_ok(),"message":result.err().map(|e|format!("Preferences not saved: {}",e)).unwrap_or_default()})));
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
            "platform":std::env::consts::OS,
            "logoUrl": assets::brand_url(),
            "helpLogoUrl": assets::brand_banner_url(),
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
        self.tree_serial += 1;
        self.io.send(jobs::IoTask::Tree {
            path: dir,
            hidden: self.settings.show_hidden,
            request: json!(null),
            root: true,
            serial: self.tree_serial,
        });
    }
    fn check_disk(&mut self) {
        if self.last_disk_check.elapsed() < std::time::Duration::from_secs(2) {
            return;
        }
        self.last_disk_check = Instant::now();
        if let (Some(path), Some(fingerprint)) = (self.path.clone(), self.fingerprint) {
            if !self.read_only {
                self.io.send(jobs::IoTask::Check {
                    tab: self.id,
                    path,
                    fingerprint,
                    revision: self.edit_revision,
                });
            }
        }
    }
    fn journal(&mut self, id: u64) {
        if let Some(doc) = self.documents.get_mut(id) {
            let draft = doc.dirty.then(|| recovery::Draft {
                path: doc.path.clone(),
                source: doc.source.clone(),
                saved_source: doc.saved_source.clone(),
                format: doc.format.clone(),
                fingerprint: doc.fingerprint,
            });
            self.io.send(jobs::IoTask::Recovery {
                root: self.root.clone(),
                key: doc.recovery_key.clone(),
                document: draft,
            });
        }
    }
    fn flush_recovery(&mut self) {
        let ids: Vec<_> = self.recovery_pending.drain().collect();
        for id in ids {
            self.journal(id);
        }
        self.recovery_flush = Instant::now();
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
        let closing = self.documents.current().clone();
        self.io.send(jobs::IoTask::Recovery {
            root: self.root.clone(),
            key: closing.recovery_key.clone(),
            document: None,
        });
        let mut closing = closing;
        closing.source = closing.saved_source.clone();
        closing.dirty = false;
        self.closed_tabs.push(closing);
        if self.closed_tabs.len() > 15 {
            self.closed_tabs.remove(0);
        }
        self.documents.remove(id);
        self.activate_tab(self.id);
        true
    }

    fn close_window(&mut self, control_flow: &mut ControlFlow) {
        if self.settings.close_to_tray && self.tray.available() {
            self.remember_window();
            self.persist_settings();
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
        self.persist_settings();
        self.flush_recovery();
        if !self.io.flush() {
            self.notify("Still writing recovery data. Please try Quit again.");
            return;
        }
        for doc in &self.documents.tabs {
            let _ = recovery::write(&self.root, &doc.recovery_key, None);
        }
        instance::release(&self.root);
        *control_flow = ControlFlow::Exit;
    }

    fn render_current(&mut self, scroll: f32) {
        #[cfg(target_os = "macos")]
        mac_menu::document_controls(&self._menu, self.image.is_some(), !self.read_only);
        let t = self.theme();
        if self.image.is_some() {
            self.generation = self.render_worker.cancel();
        } else {
            self.generation = self.render_worker.submit(jobs::RenderTask {
                generation: 0,
                tab: self.id,
                revision: self.edit_revision,
                path: self.path.clone(),
                text: self.source.clone(),
                settings: self.settings.clone(),
                theme: t.clone(),
            });
        }
        let name = self.name();

        self.send_tabs();
        let payload = json!({
            "tab": self.id,
            "themeId": t.id,
            "revision": self.edit_revision,
            "editing": self.editing,
            "editorScroll": self.editor_scroll,
            "selectionStart": self.selection_start,
            "selectionEnd": self.selection_end,
            "dirty": self.dirty,
            "imageView": self.image_view,
            "image":self.image.as_ref().and_then(|image|self.path.as_ref().map(|path|json!({"format":image.format,"bytes":image.bytes,"url":format!("{}?image={}-{}",assets::url_for(&path.to_string_lossy(),None),self.id,self.seen_mtime.and_then(|time|time.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_nanos()).unwrap_or(0))}))),
            "encoding": self.format.encoding, "lineEnding":self.format.ending, "readOnly":self.read_only, "externalChanged":self.external_changed,
            "generation":self.generation,
            "name": name,
            "path": self.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            "scroll": scroll,
        });
        self.run_js(format!("window.app.beginDocument({});", payload));
        self.run_js(format!(
            "window.app.setEditorText({});",
            json!({
                "tab":self.id,"revision":self.edit_revision,"text":self.source
            })
        ));

        if self.image.is_some() {
            self.run_js(format!("window.app.finishDocument({});",json!({"tab":self.id,"revision":self.edit_revision,"generation":self.generation,"renderKey":format!("image-{}",self.id),"html":"","outline":[],"frontMatter":"","note":""})));
        }
        let title = match &self.path {
            Some(_) => format!("{} - {}", name, root::app_name()),
            None => root::app_name().to_string(),
        };
        self.window
            .set_title(&format!("{}{}", if self.dirty { "* " } else { "" }, title));
    }

    fn open(&mut self, path: PathBuf, new_tab: bool) {
        self.open_file(path, new_tab, false, String::new());
    }
    fn open_file(&mut self, path: PathBuf, new_tab: bool, reload: bool, fragment: String) {
        if reload && !self.may_close() {
            return;
        }
        self.io.send(jobs::IoTask::Open(jobs::OpenTask {
            restore: false,
            view: None,
            path,
            new_tab,
            from: self.id,
            revision: self.edit_revision,
            reload,
            fragment,
            limit: self.settings.plain_text_above_mb,
        }));
        self.notify("Opening…");
    }
    fn finish_open(&mut self, task: jobs::OpenTask, loaded: storage::Loaded) {
        if !task.reload {
            if let Some(id) = self.documents.find_path(&task.path) {
                self.pending_fragment = task.fragment;
                self.activate_tab(id);
                return;
            }
        }
        let prior_fingerprint = self.fingerprint;
        let prior_path = self.path.clone();
        if !task.reload
            && !task.new_tab
            && self.id == task.from
            && self.edit_revision == task.revision
            && !self.may_close()
        {
            return;
        }
        if self.path.as_ref() == Some(&task.path)
            && (self.fingerprint != prior_fingerprint || self.path != prior_path)
        {
            self.open_file(task.path, task.new_tab, task.reload, task.fragment);
            return;
        }
        if task.reload && (self.id != task.from || self.edit_revision != task.revision) {
            self.notify("Reload cancelled because the document changed.");
            return;
        }
        let mut document = Document::loaded(task.path.clone(), loaded);
        if task.reload {
            document.editing = self.editing;
            document.scroll = self.scroll;
            document.editor_scroll = self.editor_scroll;
            document.selection_start = self.selection_start;
            document.selection_end = self.selection_end;
            document.image_view = self.image_view.clone();
        }
        if let Some(view) = task.view {
            document.scroll = view.scroll;
            document.editing = view.editing && !document.read_only;
            document.editor_scroll = view.editor_scroll;
            document.selection_start = view.selection_start;
            document.selection_end = view.selection_end;
            document.image_view = view.image_view;
        }
        if task.new_tab || self.id != task.from || self.edit_revision != task.revision {
            self.documents.insert(document);
        } else {
            let old = self.documents.current().clone();
            self.io.send(jobs::IoTask::Recovery {
                root: self.root.clone(),
                key: old.recovery_key,
                document: None,
            });
            self.documents.replace(document);
        }
        assets::set_scope(task.path.parent());
        if !task.restore || self.startup_target.is_none() {
            self.settings.remember(&task.path);
        }
        self.pending_fragment = task.fragment;
        self.render_current(self.scroll);
        self.send_recents();
        self.persist_settings();
        if !task.reload && !task.restore && !task.path.starts_with(&self.tree_dir) {
            if let Some(parent) = task.path.parent() {
                self.send_tree(parent.into());
            }
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
            .add_filter("Images", assets::IMAGE_EXTENSIONS)
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(picked) = dialog.pick_files() {
            for path in picked {
                self.open(path, true);
            }
        }
    }

    /// Writes through a temporary file in the same folder, then renames: an
    /// interrupted save cannot leave the note truncated.
    #[cfg(test)]
    fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
        storage::write_atomic(path, text.as_bytes())
    }

    fn save(&mut self, text: String, save_as: bool) -> bool {
        if self.read_only {
            self.notify(if self.image.is_some() {
                "Images are view-only and cannot be overwritten by the text editor."
            } else {
                "Read-only preview: the original file cannot be overwritten."
            });
            return false;
        }
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
        if self.path.as_ref() == Some(&path)
            && self.fingerprint.is_some()
            && (current != self.seen_mtime
                || self
                    .fingerprint
                    .is_some_and(|hash| storage::disk_fingerprint(&path).ok() != Some(hash)))
        {
            let choice = rfd::MessageDialog::new()
                .set_title("Changed on disk")
                .set_description("This file changed on disk since it was opened.\n\nOverwrite it?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if choice != rfd::MessageDialogResult::Yes {
                self.run_js("window.app.note('save cancelled - file changed on disk');".into());
                return false;
            }
        }

        let new_path = self.path.as_ref() != Some(&path);
        if !new_path
            && text == self.saved_source
            && self.fingerprint.is_some()
            && storage::disk_fingerprint(&path).ok() == self.fingerprint
        {
            if self.source != text {
                self.edit_revision += 1;
                self.source = text;
            }
            self.dirty = false;
            self.journal(self.id);
            self.render_current(self.scroll);
            self.notify("Saved; file content is unchanged.");
            return true;
        }
        let bytes = self.format.encode(&text);
        match storage::write_atomic(&path, &bytes) {
            Ok(()) => {
                self.fingerprint = Some(storage::fingerprint(&bytes));
                self.external_changed = false;
                if self.source != text {
                    self.edit_revision += 1;
                }
                self.saved_source = text.clone();
                self.source = text;
                assets::set_scope(path.parent());
                self.path = Some(path.clone());
                self.seen_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.dirty = false;
                self.journal(self.id);
                self.persist_settings();
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
        let blocked = self.settings.blocked_write;
        let mut current = serde_json::to_value(&self.settings).unwrap_or(json!({}));
        if let Some(map) = current.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        if let Ok(mut updated) = serde_json::from_value::<Settings>(current) {
            updated.blocked_write = blocked;
            updated.normalize();
            let rerender = matches!(
                key,
                "theme"
                    | "text_contrast"
                    | "highlight_limit_kb"
                    | "plain_text_above_mb"
                    | "view_mode"
                    | "syntax_colour"
                    | "remote_images"
            );
            if key == "theme" {
                self.preview_theme = None;
            }
            self.settings = updated;
            if key == "show_hidden" {
                self.send_tree(self.tree_dir.clone());
            }
            self.send_settings();
            if rerender {
                self.render_current(self.scroll);
            }
            self.persist_settings();
        }
    }

    fn handle(&mut self, message: &str, control_flow: &mut ControlFlow) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else {
            return;
        };
        let command = value.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        if !["scroll", "edit", "editPatch", "viewState", "treeState"].contains(&command) {
            log::line(&format!("page: {}", command));
        }

        if let Some(id) = value.get("fromTab").and_then(|v| v.as_u64()) {
            if let Some(view) = value.get("view") {
                if let Some(doc) = self.documents.get_mut(id) {
                    if let Some(v) = view.get("editing").and_then(|v| v.as_bool()) {
                        doc.editing = v && !doc.read_only;
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
                    if doc.image.is_some() {
                        if let Some(view) = view.get("imageView").and_then(|v| {
                            serde_json::from_value::<settings::ImageView>(v.clone()).ok()
                        }) {
                            let mut view = view;
                            view.normalize();
                            doc.image_view = Some(view);
                        }
                    }
                }
            }
        }
        match command {
            "ready" => {
                if self.ui_ready {
                    return;
                }
                self.ui_ready = true;
                self.navigation_ready.set(true);
                log::line("UI handshake received");
                self.send_init();
                self.send_recents();
                let dir = self.tree_dir.clone();
                self.send_tree(dir);
                let scroll = self.settings.last_scroll;
                self.render_current(scroll);
                let paths = std::mem::take(&mut self.startup_paths);
                for (index, path) in paths.into_iter().enumerate() {
                    let view = self
                        .session_restore
                        .iter()
                        .find(|view| Path::new(&view.path) == path)
                        .cloned();
                    self.io.send(jobs::IoTask::Open(jobs::OpenTask {
                        restore: true,
                        view,
                        path,
                        new_tab: index > 0,
                        from: self.id,
                        revision: self.edit_revision,
                        reload: false,
                        fragment: String::new(),
                        limit: self.settings.plain_text_above_mb,
                    }));
                }
                if !self.recovery_checked {
                    self.recovery_checked = true;
                    let drafts = recovery::read(&self.root);
                    if !drafts.is_empty() {
                        let choice = rfd::MessageDialog::new()
                            .set_title("Recover notes")
                            .set_description(format!(
                                "{} unsaved note(s) were found. Restore them?",
                                drafts.len()
                            ))
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .show();
                        for (key, draft) in drafts {
                            if choice == rfd::MessageDialogResult::Yes {
                                let mut doc = Document::new(draft.path, draft.source);
                                doc.saved_source = draft.saved_source;
                                doc.format = draft.format;
                                doc.fingerprint = draft.fingerprint;
                                doc.dirty = true;
                                doc.editing = true;
                                doc.recovery_key = key;
                                self.documents.insert(doc);
                            } else if choice == rfd::MessageDialogResult::No {
                                let _ = recovery::write(&self.root, &key, None);
                            }
                        }
                        if choice == rfd::MessageDialogResult::Yes && self.documents.tabs.len() > 1
                        {
                            let welcome_id = self.documents.tabs[0].id;
                            self.documents.remove(welcome_id);
                        }
                        self.activate_tab(self.id);
                    }
                }
                if !self.settings.load_warning.is_empty() {
                    self.notify(&self.settings.load_warning);
                }
                if self.smoke_started.is_some() {
                    self.run_js(include_str!("smoke.js").into());
                }
            }
            "startupError" => {
                log::line(&format!(
                    "UI startup error: {}",
                    value
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                ));
            }
            "smokeInspect" if self.smoke_started.is_some() => {
                self.flush_recovery();
                self.io.flush();
                let disk = self
                    .path
                    .as_ref()
                    .filter(|p| p.starts_with(&self.root))
                    .and_then(|p| storage::read(p, 1).ok())
                    .map(|d| d.source);
                self.run_js(format!("window.app.smokeState={};",json!({"token":value.get("token"),"source":self.source,"disk":disk,"dirty":self.dirty,"recovery":recovery::read(&self.root).len()})));
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
                self.persist_settings();
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
            "editPatch" => {
                let id = value
                    .get("fromTab")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(self.id);
                let mut accepted = false;
                if let Some(doc) = self.documents.get_mut(id) {
                    if !doc.read_only {
                        if let (Some(start), Some(end), Some(insert), Some(revision)) = (
                            value.get("start").and_then(|v| v.as_u64()),
                            value.get("end").and_then(|v| v.as_u64()),
                            value.get("insert").and_then(|v| v.as_str()),
                            value.get("revision").and_then(|v| v.as_u64()),
                        ) {
                            if revision == doc.edit_revision + 1
                                && storage::apply_patch(
                                    &mut doc.source,
                                    start as usize,
                                    end as usize,
                                    insert,
                                )
                            {
                                doc.edit_revision = revision;
                                doc.dirty = doc.source != doc.saved_source;
                                accepted = true;
                            }
                        }
                    }
                }
                if accepted {
                    self.recovery_pending.insert(id);
                    self.send_tabs();
                } else {
                    self.run_js(format!("window.app.resendEditor({});", id));
                }
            }
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
                            if doc.read_only {
                                return;
                            }
                            doc.edit(text.to_owned());
                            doc.edit_revision = revision;
                        }
                        changed = before != doc.dirty;
                    }
                    self.recovery_pending.insert(id);
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
                    self.open_file(path, false, true, String::new());
                } else {
                    self.notify("Save this note before reloading it.");
                }
            }
            "keepDisk" => {
                self.external_changed = false;
                self.notify("Keeping your edits. Saving will still check for a conflict.");
                self.render_current(self.scroll);
            }
            "cleanHistory" => {
                match maintenance::cleanup(
                    &self.root,
                    self.settings.log_retention_days,
                    self.settings.backup_retention,
                ) {
                    Ok((removed, failed)) => self.notify(&format!(
                        "Cleaned {} old history item(s); {} could not be removed.",
                        removed, failed
                    )),
                    Err(error) => self.notify(&format!("Cleanup failed: {}", error)),
                }
            }
            "clearRecents" => {
                self.settings.recents.clear();
                self.send_recents();
                self.persist_settings();
            }
            "reopenTab" => {
                if let Some(doc) = self.closed_tabs.pop() {
                    if let Some(path) = doc.path.clone() {
                        let view = settings::SessionTab {
                            path: path.to_string_lossy().into_owned(),
                            scroll: doc.scroll,
                            editing: doc.editing,
                            editor_scroll: doc.editor_scroll,
                            selection_start: doc.selection_start,
                            selection_end: doc.selection_end,
                            image_view: doc.image_view.clone(),
                        };
                        self.io.send(jobs::IoTask::Open(jobs::OpenTask {
                            restore: false,
                            view: Some(view),
                            path,
                            new_tab: true,
                            from: self.id,
                            revision: self.edit_revision,
                            reload: false,
                            fragment: String::new(),
                            limit: self.settings.plain_text_above_mb,
                        }));
                    } else {
                        self.documents.insert(doc);
                        self.activate_tab(self.id);
                    }
                }
            }
            "moveTab" => {
                if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                    let before = value.get("before").and_then(|v| v.as_u64());
                    if self.documents.move_before(id, before) {
                        self.send_tabs();
                        self.persist_settings();
                    }
                }
            }
            "treeState"
                if value
                    .get("workspace")
                    .and_then(|v| v.as_str())
                    .is_some_and(|path| Path::new(path) == self.tree_dir) =>
            {
                if let Some(paths) = value.get("expanded").and_then(|v| v.as_array()) {
                    self.settings.expanded_folders = paths
                        .iter()
                        .filter_map(|v| v.as_str())
                        .filter(|path| Path::new(path).starts_with(&self.tree_dir))
                        .map(str::to_owned)
                        .collect();
                }
                if let Some(scroll) = value.get("scroll").and_then(|v| v.as_f64()) {
                    self.settings.tree_scroll = scroll;
                }
                if let Some(filter) = value.get("filter").and_then(|v| v.as_str()) {
                    self.settings.tree_filter = filter.into();
                }
                self.settings
                    .expanded_folders
                    .retain(|path| path.len() <= 131072);
                self.settings.tree_scroll = if self.settings.tree_scroll.is_finite() {
                    self.settings.tree_scroll.clamp(0.0, 100_000_000.0)
                } else {
                    0.0
                };
                if value
                    .get("persist")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    self.persist_settings();
                }
            }
            "cycleSidebar" => {
                self.settings.cycle_sidebar();
                self.send_settings();
                self.persist_settings();
            }
            "refreshTree" => self.send_tree(self.tree_dir.clone()),
            "openPath" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    self.open_file(
                        PathBuf::from(path),
                        value
                            .get("newTab")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        false,
                        value
                            .get("fragment")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .into(),
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
                    self.io.send(jobs::IoTask::Tree {
                        path: path.into(),
                        hidden: self.settings.show_hidden,
                        request: value.get("request").cloned().unwrap_or(json!(null)),
                        root: false,
                        serial: 0,
                    });
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
                let Some(text) = value
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                else {
                    self.notify("Save refused: document text was not supplied.");
                    return;
                };
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
            "previewTheme" => {
                if let Some(id) = value.get("theme").and_then(|v| v.as_str()) {
                    if theme::builtin().iter().any(|theme| theme.id == id) {
                        self.preview_theme = Some(id.to_string());
                        self.run_js(format!(
                            "window.app.themePreview({});",
                            json!({
                                "id":id,"token":value.get("token"),"theme":self.theme()
                            })
                        ));
                        self.render_current(self.scroll);
                    }
                }
            }
            "setting" => {
                if let Some(key) = value.get("key").and_then(|k| k.as_str()) {
                    let new_value = value.get("value").cloned().unwrap_or(json!(null));
                    self.apply_setting(key, new_value);
                }
            }
            "resetSettings" => {
                self.preview_theme = None;
                self.run_js("window.app.clearThemePreview();".into());
                let old = self.settings.clone();
                self.settings = Settings::default();
                self.settings.recents = old.recents;
                self.settings.last_path = old.last_path;
                self.settings.workspace = old.workspace;
                self.settings.expanded_folders = old.expanded_folders;
                self.settings.tree_scroll = old.tree_scroll;
                self.settings.tree_filter = old.tree_filter;
                self.settings.blocked_write = old.blocked_write;
                self.settings.last_scroll = old.last_scroll;
                self.send_tree(self.tree_dir.clone());
                self.send_settings();
                self.render_current(0.0);
                self.persist_settings();
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
    std::fs::create_dir_all(&root)?;
    log::init(&root);
    log::line(&format!("start root={}", root.display()));

    let arguments: Vec<PathBuf> = std::env::args()
        .skip(1)
        .filter(|arg| !(cfg!(target_os = "macos") && arg.starts_with("-psn_")))
        .take(128)
        .map(PathBuf::from)
        .map(|p| p.canonicalize().unwrap_or(p))
        .collect();
    let instance_guard = match instance::acquire(&root) {
        Ok(guard) => guard,
        Err(error) => {
            for _ in 0..20 {
                if instance::hand_off(&root, &arguments) {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            return Err(
                format!("A running copy did not acknowledge the request: {}", error).into(),
            );
        }
    };
    let argument = arguments.first().cloned();

    let settings = Settings::load(&root);
    let font_list = fonts::families();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let render_worker = jobs::RenderWorker::new(proxy.clone());
    let io = jobs::IoWorker::new(proxy.clone());

    let handoff_proxy = proxy.clone();
    instance::listen(&root, move |paths| {
        let _ = handoff_proxy.send_event(UserEvent::Handoff(paths));
    })?;

    let monitor = event_loop.primary_monitor();
    let bounds = monitor
        .as_ref()
        .map(|m| m.size().to_logical::<u32>(m.scale_factor()));
    let window = WindowBuilder::new()
        .with_title(root::app_name())
        .with_decorations(false)
        .with_window_icon(tao::window::Icon::from_rgba(icon::rgba(64), 64, 64).ok())
        .with_min_inner_size(tao::dpi::LogicalSize::new(620.0, 400.0))
        .with_inner_size(tao::dpi::LogicalSize::new(
            settings.window_width.min(
                bounds
                    .map(|s| s.width.saturating_sub(32).max(620))
                    .unwrap_or(7680),
            ) as f64,
            settings.window_height.min(
                bounds
                    .map(|s| s.height.saturating_sub(64).max(400))
                    .unwrap_or(4320),
            ) as f64,
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
    #[cfg(target_os = "macos")]
    let menu = mac_menu::create(proxy.clone())?;
    let tray = tray::SystemTray::new(proxy.clone());

    let drop_proxy = proxy.clone();
    let ipc_proxy = proxy.clone();
    let asset_worker = assets::worker_pool();
    let navigation_ready = std::rc::Rc::new(std::cell::Cell::new(false));
    let navigation_gate = navigation_ready.clone();
    let page_proxy = proxy.clone();
    let builder = WebViewBuilder::new()
        .with_html(ui::shell())
        .with_hotkeys_zoom(false)
        .with_navigation_handler(move |url| {
            // Before the handshake only our supplied HTML is loading. Engine startup
            // navigations need not expose the final about:blank URI yet.
            let booting = !navigation_gate.get();
            let allowed = navigation_allowed(navigation_gate.get(), &url);
            log::line(&format!(
                "navigation boot={} allowed={} uri={}",
                booting,
                allowed,
                url.chars().take(160).collect::<String>()
            ));
            allowed
        })
        .with_on_page_load_handler(move |phase, url| {
            let finished = matches!(phase, wry::PageLoadEvent::Finished);
            log::line(&format!(
                "page load {} uri={}",
                if finished { "finished" } else { "started" },
                url.chars().take(160).collect::<String>()
            ));
            if finished {
                let _ = page_proxy.send_event(UserEvent::BrowserLoaded);
            }
        })
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_drag_drop_handler(move |event| {
            if let wry::DragDropEvent::Drop { paths, .. } = event {
                for path in paths {
                    let _ = drop_proxy.send_event(UserEvent::Dropped(path.clone()));
                }
            }
            true
        })
        .with_ipc_handler(move |request: Request<String>| {
            let _ = ipc_proxy.send_event(UserEvent::Page(request.body().to_string()));
        })
        .with_asynchronous_custom_protocol(
            "asset".into(),
            move |_id, request: Request<Vec<u8>>, responder| {
                let _ = asset_worker.send((request.uri().to_string(), responder));
            },
        );

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

    let source = welcome();
    let session_restore = settings.saved_tabs.clone();
    let mut startup_paths = arguments.clone();
    if startup_paths.is_empty() {
        if settings.restore_last_file && settings.restore_tabs {
            startup_paths = session_restore
                .iter()
                .map(|s| PathBuf::from(&s.path))
                .collect();
        }
        if startup_paths.is_empty() {
            if let Some(path) = initial.take() {
                startup_paths.push(path);
            }
        }
    }

    let app_last_path = settings.last_path.clone();
    let startup_target = if arguments.is_empty() && !app_last_path.is_empty() {
        Some(PathBuf::from(&app_last_path))
    } else {
        None
    };
    let mut app = App {
        _instance: instance_guard,
        #[cfg(target_os = "macos")]
        _menu: menu,
        ui_ready: false,
        boot_started: Instant::now(),
        boot_probe: false,
        navigation_ready,
        documents: Documents::new(Document::new(None, source)),
        tray,
        tree_dir,
        root: root.clone(),
        smoke_started: std::env::var_os("EXIT_WHEN_READY").map(|_| started),
        settings,
        preview_theme: None,
        render_worker,
        io,
        generation: 0,
        tree_serial: 0,
        pending_fragment: String::new(),
        closed_tabs: Vec::new(),
        session_restore,
        startup_pending: startup_paths.len(),
        startup_target,
        startup_paths,
        recovery_checked: false,
        recovery_pending: Default::default(),
        recovery_flush: Instant::now(),
        last_disk_check: Instant::now(),
        fonts: font_list,
        window,
        webview,
    };

    log::line(&format!(
        "window ready in {} ms",
        started.elapsed().as_millis()
    ));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now()+std::time::Duration::from_millis(400));
        if !app.ui_ready&&!app.boot_probe&&app.boot_started.elapsed()>std::time::Duration::from_secs(3){
            app.boot_probe=true;
            let _=app.webview.evaluate_script_with_callback("JSON.stringify({url:location.href,readyState:document.readyState,app:!!window.app,ipc:!!window.ipc,nativeBridge:!!window.chrome?.webview,bodyLength:document.body?.innerHTML.length,errors:window.startupErrors||[]})",|result|log::line(&format!("startup diagnostic: {}",result)));
            app.run_js("window.app?.requestReady?.();".into());
        }
        if app.window.is_focused(){app.check_disk();}
        if app.recovery_flush.elapsed()>std::time::Duration::from_millis(700){app.flush_recovery();}
        match event {
            Event::UserEvent(UserEvent::Rendered{generation,tab,revision,key,document}) if generation==app.generation&&tab==app.id&&revision==app.edit_revision => {
                    let fragment=std::mem::take(&mut app.pending_fragment);
                    app.run_js(format!("window.app.finishDocument({});",json!({"tab":tab,"revision":revision,"generation":generation,"renderKey":key,"html":document.html,"outline":document.outline,"frontMatter":document.front_matter,"note":if app.read_only{"Large file: showing a read-only preview of the first 256 KB.".into()}else{document.note},"fragment":fragment})));
            }
            Event::UserEvent(UserEvent::Loaded{task,result})=>{
                let restoring=task.restore;
                match result{Ok(loaded)=>app.finish_open(task,loaded),Err(error)=>app.notify(&format!("Cannot open {}: {}",task.path.display(),error))}
                if restoring{app.startup_pending=app.startup_pending.saturating_sub(1);if app.startup_pending==0{if let Some(target)=app.startup_target.take(){if let Some(id)=app.documents.find_path(&target){app.activate_tab(id);}}app.persist_settings();}}
            },
            Event::UserEvent(UserEvent::TreeLoaded{path,request,root,serial,result})=>{
                if root{if serial==app.tree_serial{match result{Ok(entries)=>{if Path::new(&app.settings.workspace)!=path {app.settings.expanded_folders.clear();app.settings.tree_scroll=0.0;app.settings.tree_filter.clear();}app.tree_dir=path.clone();app.settings.workspace=path.to_string_lossy().into_owned();app.run_js(format!("window.app.setTree({});",json!({"dir":path.to_string_lossy(),"parent":path.parent().is_some(),"entries":entries,"view":{"expanded":app.settings.expanded_folders,"scroll":app.settings.tree_scroll,"filter":app.settings.tree_filter}})));app.persist_settings();},Err(error)=>app.notify(&format!("Cannot read folder: {}",error))}}}
                else{let(entries,error)=match result{Ok(entries)=>(entries,None),Err(error)=>(Vec::new(),Some(error))};app.run_js(format!("window.app.setEntries({});",json!({"path":path.to_string_lossy(),"request":request,"entries":entries,"error":error})));}
            }
            Event::UserEvent(UserEvent::DiskChecked{tab,revision,changed})=>{if let Some(doc)=app.documents.get_mut(tab){if revision==doc.edit_revision{doc.external_changed=changed;}}
                if app.id==tab{app.run_js(format!("window.app.diskStatus({});",app.external_changed));}}
            Event::UserEvent(UserEvent::RecoveryError(error))=>app.notify(&format!("Draft recovery could not be saved: {}",error)),
            Event::WindowEvent{event:WindowEvent::Focused(true),..}=>app.check_disk(),
            #[cfg(target_os="macos")]
            Event::Opened{urls}=>{app.show();app.startup_target=None;for url in urls{if let Ok(path)=url.to_file_path(){if app.ui_ready{app.open(path,true);}else{app.startup_paths.push(path);app.startup_pending=app.startup_paths.len();}}}}
            #[cfg(target_os="macos")]
            Event::UserEvent(UserEvent::MacCommand(command))=>{
                app.show();match command.as_str(){
                    "quit"=>app.quit(control_flow),
                    "help"=>app.run_js("window.app?.help(true);".into()),
                    "replace"=>app.run_js("window.app?.showFind(true);if(document.getElementById('replace-row').hidden)document.getElementById('find-replace').click();".into()),
                    "options"=>app.run_js("window.app?.options(true);".into()),
                    "close"=>app.handle(&json!({"cmd":"closeTab","id":app.id}).to_string(),control_flow),
                    action=>{let button=match action{"new"=>"b-new","open"=>"b-open","folder"=>"folder-open","save"=>"b-save","saveAs"=>"b-saveas","find"=>"b-find",_=>""};if !button.is_empty(){app.run_js(format!("document.getElementById({}).click();",json!(button)));}}
                }
            }
            Event::UserEvent(UserEvent::BrowserLoaded)=>{app.run_js("window.app?.requestReady?.();".into());}
            Event::UserEvent(UserEvent::Page(message)) => app.handle(&message, control_flow),
            Event::UserEvent(UserEvent::Dropped(path)) => app.open(path, true),
            Event::UserEvent(UserEvent::Handoff(paths)) => {
                app.show();
                for path in paths { app.open(PathBuf::from(path), true); }
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
