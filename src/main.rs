// No console window on Windows: this is a windowed app, and its log goes to a
// file beside the binary.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod dialogs;
mod disk;
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
mod paths;
mod preferences;
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
mod workspace_files;

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
    FontsLoaded(Vec<fonts::FontFamily>),
    PreferencesSaved {
        serial: u64,
        error: Option<String>,
    },
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
        path: PathBuf,
        epoch: u64,
        observed: Option<u64>,
    },
    QuickMatches(workspace_files::SearchResult),
    FileOperation(workspace_files::OperationResult),
    RecoveryStatus(Option<String>),
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
    _instance: Option<instance::Guard>,
    smoke_started: Option<Instant>,
    ui_ready: bool,
    boot_started: Instant,
    boot_probe: bool,
    navigation_ready: std::rc::Rc<std::cell::Cell<bool>>,
    settings: Settings,
    preferences: preferences::Writer,
    preference_serial: u64,
    view_changed: Option<Instant>,
    preview_theme: Option<String>,
    documents: Documents,
    render_worker: jobs::RenderWorker,
    io: jobs::IoWorker,
    workspace_worker: workspace_files::Worker,
    quick_request: u64,
    file_request: Option<u64>,
    generation: u64,
    tree_serial: u64,
    closed_tabs: Vec<Document>,
    session_restore: Vec<settings::SessionTab>,
    startup_paths: Vec<PathBuf>,
    startup_pending: usize,
    startup_target: Option<PathBuf>,
    recovery_checked: bool,
    recovery_pending: std::collections::HashSet<u64>,
    recovery_flush: Instant,
    recovery_activity: Instant,
    last_disk_check: Instant,
    disk_check_pending: bool,
    fonts: Vec<fonts::FontFamily>,
    fonts_loaded: bool,
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
        theme::find(
            self.preview_theme
                .as_deref()
                .unwrap_or(&self.settings.theme),
        )
        .readable(self.settings.text_contrast)
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
        self.preference_serial += 1;
        self.view_changed = None;
        self.preferences
            .submit(self.preference_serial, self.settings.clone());
    }

    fn send_init(&self) {
        let missing: Vec<&str> = [
            ("ui", self.settings.ui_font.as_str()),
            ("body", self.settings.body_font.as_str()),
            ("code", self.settings.code_font.as_str()),
        ]
        .iter()
        .filter(|(_, family)| self.fonts_loaded && !fonts::has_family(&self.fonts, family))
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
        if dir.as_os_str().is_empty() {
            self.run_js(format!("window.app.setTree({});", json!({"dir":"","parent":false,"entries":[],"view":{"expanded":[],"scroll":0,"filter":""}})));
            return;
        }
        self.io.send(jobs::IoTask::Tree {
            path: dir,
            hidden: self.settings.show_hidden,
            dates: self.settings.file_dates,
            sort_date: self.settings.file_sort_date,
            request: json!(null),
            root: true,
            serial: self.tree_serial,
        });
    }
    fn check_disk(&mut self, force: bool) {
        if self.disk_check_pending
            || (!force && self.last_disk_check.elapsed() < std::time::Duration::from_secs(2))
        {
            return;
        }
        self.last_disk_check = Instant::now();
        if let (Some(path), Some(_)) = (self.path.clone(), self.fingerprint) {
            if !self.read_only {
                self.disk_check_pending = true;
                self.io.send(jobs::IoTask::Check {
                    tab: self.id,
                    path,
                    epoch: self.disk.epoch,
                    force,
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
    fn recovery_due(&self) -> Instant {
        (self.recovery_activity + std::time::Duration::from_millis(700))
            .min(self.recovery_flush + std::time::Duration::from_secs(5))
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

    fn send_dirty(&self, id: u64) {
        if let Some(doc) = self.documents.tabs.iter().find(|d| d.id == id) {
            self.run_js(format!(
                "window.app.tabDirty({});",
                json!({"tab":id,"dirty":doc.dirty})
            ));
            if id == self.id {
                self.window.set_title(&format!(
                    "{}{} - {}",
                    if doc.dirty { "* " } else { "" },
                    doc.name(),
                    root::app_name()
                ));
            }
        }
    }

    fn activate_tab(&mut self, id: u64) {
        if self.documents.activate(id) {
            if self.unloaded && !self.loading {
                self.loading = true;
                self.load_error = None;
                if let Some(path) = self.path.clone() {
                    self.io.send(jobs::IoTask::Open(jobs::OpenTask {
                        restore: true,
                        view: self.view(),
                        path,
                        new_tab: false,
                        from: self.id,
                        revision: self.edit_revision,
                        reload: false,
                        fragment: String::new(),
                        limit: self.settings.plain_text_above_mb,
                    }));
                }
            }
            if !self.unloaded {
                assets::set_scope(self.path.as_deref().and_then(Path::parent));
            }
            if let Some(path) = self.path.clone() {
                self.settings.remember(&path);
            }
            self.render_current(self.scroll);
        }
    }

    fn approve_network_path(&self, path: &Path) -> bool {
        if !paths::network_or_device(path) {
            return true;
        }
        if paths::device(path) {
            self.notify("Device paths cannot be opened as documents.");
            return false;
        }
        crate::dialogs::MessageDialog::new().set_title("Open network location?")
            .set_description(format!("This link opens {}. Connecting may send your operating-system credentials to that server. Continue?", path.display()))
            .set_buttons(crate::dialogs::MessageButtons::YesNo).show() == crate::dialogs::MessageDialogResult::Yes
    }

    fn queue_launch(&mut self, path: PathBuf) {
        if self.ui_ready {
            self.open(path, true);
        } else if !self.startup_paths.iter().any(|p| paths::same(p, &path)) {
            self.startup_target = Some(path.clone());
            self.startup_paths.push(path);
        }
    }

    fn remember_closed(&mut self, mut doc: Document) {
        if doc.path.is_some() && !doc.dirty {
            doc.source.clear();
            doc.saved_source.clear();
        }
        self.closed_tabs.push(doc);
        while self.closed_tabs.len() > 15
            || (self.closed_tabs.len() > 1
                && self
                    .closed_tabs
                    .iter()
                    .map(|d| d.source.len() + d.saved_source.len())
                    .sum::<usize>()
                    > 64 * 1024 * 1024)
        {
            self.closed_tabs.remove(0);
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
        let original = self.id;
        let Some(closing) = self.documents.tabs.iter().find(|d| d.id == id) else {
            return true;
        };
        if closing.dirty {
            self.activate_tab(id);
            if !self.may_close() {
                self.activate_tab(original);
                return false;
            }
        }
        let closing = self
            .documents
            .tabs
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .clone();
        self.io.send(jobs::IoTask::Recovery {
            root: self.root.clone(),
            key: closing.recovery_key.clone(),
            document: None,
        });
        self.recovery_pending.remove(&id);
        self.remember_closed(closing);
        self.documents.remove(id);
        if id != original {
            self.documents.activate(original);
        }
        self.activate_tab(self.id);
        self.persist_settings();
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
        if self.file_request.is_some() {
            self.notify("A file operation is still finishing. Please try Quit again afterward.");
            return;
        }
        let original = self.id;
        let dirty: Vec<u64> = self
            .documents
            .tabs
            .iter()
            .filter(|d| d.dirty)
            .map(|d| d.id)
            .collect();
        for id in dirty {
            self.show();
            self.activate_tab(id);
            if !self.may_close() {
                return;
            }
        }
        self.activate_tab(original);
        self.remember_window();
        self.persist_settings();
        if !self.preferences.flush() {
            self.show();
            if crate::dialogs::MessageDialog::new()
                .set_title("Preferences not saved")
                .set_description(
                    "The latest workspace and preferences could not be saved. Quit anyway?",
                )
                .set_buttons(crate::dialogs::MessageButtons::YesNo)
                .show()
                != crate::dialogs::MessageDialogResult::Yes
            {
                return;
            }
        }
        self.flush_recovery();
        if !self.io.flush() {
            self.notify("Still writing recovery data. Please try Quit again.");
            return;
        }
        for doc in &self.documents.tabs {
            let _ = recovery::write(&self.root, &doc.recovery_key, None);
        }
        if self._instance.is_some() {
            instance::release(&self.root);
        }
        *control_flow = ControlFlow::Exit;
    }

    fn render_current(&mut self, scroll: f32) {
        #[cfg(target_os = "macos")]
        mac_menu::document_controls(&self._menu, self.image.is_some(), !self.read_only);
        let t = self.theme();
        if self.image.is_some() || self.unloaded {
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
            "editing": self.editing && !self.unloaded,
            "editorScroll": self.editor_scroll,
            "selectionStart": self.selection_start,
            "selectionEnd": self.selection_end,
            "dirty": self.dirty,
            "imageView": self.image_view,
            "image":self.image.as_ref().and_then(|image|self.path.as_ref().map(|path|json!({"format":image.format,"bytes":image.bytes,"url":format!("{}?image={}-{}",assets::url_for(&path.to_string_lossy(),None),self.id,self.seen_mtime.and_then(|time|time.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_nanos()).unwrap_or(0))}))),
            "encoding": self.format.encoding, "lineEnding":self.format.label(), "readOnly":self.read_only, "externalChanged":self.external_changed,
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

        if self.unloaded {
            let message = if self.loading {
                "Opening file…".to_owned()
            } else {
                format!(
                    "File unavailable. Use Reload to try again. {}",
                    self.load_error.as_deref().unwrap_or("")
                )
            };
            self.run_js(format!("window.app.finishDocument({});", json!({"tab":self.id,"revision":self.edit_revision,"generation":self.generation,"renderKey":format!("unavailable-{}-{}",self.id,self.loading),"html":"","outline":[],"frontMatter":"","note":message})));
        } else if self.image.is_some() {
            self.pending_fragment.clear();
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
        if task.restore {
            let Some(existing) = self.documents.get_mut(task.from) else {
                return;
            };
            if !existing.unloaded || !existing.loading || existing.edit_revision != task.revision {
                return;
            }
            let mut document = Document::loaded(task.path.clone(), loaded);
            if let Some(view) = existing.view() {
                document.apply_view(&view);
            }
            document.editing &= !document.read_only;
            document.id = existing.id;
            *existing = document;
            if self.id == task.from {
                assets::set_scope(self.path.as_deref().and_then(Path::parent));
                self.render_current(self.scroll);
            } else {
                self.send_tabs();
            }
            self.persist_settings();
            return;
        }
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
            if !task.reload {
                self.remember_closed(old.clone());
            }
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

    fn finish_file_operation(&mut self, completed: workspace_files::OperationResult) {
        if self.file_request != Some(completed.request) {
            return;
        }
        self.file_request = None;
        match completed.result {
            Err(error) => self.run_js(format!(
                "window.app.fileOperation({});",
                json!({"request":completed.request,"error":error})
            )),
            Ok((old, destination)) => {
                let renamed = old.is_some();
                if let Some(old) = old {
                    for doc in &mut self.documents.tabs {
                        if let Some(path) = &doc.path {
                            if let Some(new) = workspace_files::rebase(path, &old, &destination) {
                                doc.path = Some(new);
                                if doc.dirty {
                                    self.recovery_pending.insert(doc.id);
                                }
                            }
                        }
                    }
                    for doc in &mut self.closed_tabs {
                        if let Some(path) = &doc.path {
                            if let Some(new) = workspace_files::rebase(path, &old, &destination) {
                                doc.path = Some(new);
                            }
                        }
                    }
                    for path in self
                        .settings
                        .recents
                        .iter_mut()
                        .chain(self.settings.expanded_folders.iter_mut())
                        .chain(std::iter::once(&mut self.settings.last_path))
                    {
                        if let Some(new) =
                            workspace_files::rebase(Path::new(path), &old, &destination)
                        {
                            *path = new.to_string_lossy().into_owned();
                        }
                    }
                    assets::set_scope(self.path.as_deref().and_then(Path::parent));
                    self.flush_recovery();
                    self.render_current(self.scroll);
                    self.send_tabs();
                    self.send_recents();
                }
                if !renamed {
                    if let Some(parent) = destination.parent() {
                        if parent != self.tree_dir {
                            self.settings
                                .expanded_folders
                                .push(parent.to_string_lossy().into_owned());
                        }
                    }
                }
                self.run_js(format!(
                    "window.app.fileOperation({});",
                    json!({"request":completed.request,"path":destination.to_string_lossy(),"restoreTree":true})
                ));
                self.send_tree(self.tree_dir.clone());
                self.persist_settings();
            }
        }
    }

    fn pick_and_open(&mut self) {
        let mut dialog = crate::dialogs::FileDialog::new()
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
        if self.file_request.is_some() {
            self.notify("A file operation is still finishing. Please try Save again afterward.");
            return false;
        }
        if self.read_only {
            self.notify(if self.image.is_some() {
                "Images are view-only and cannot be overwritten by the text editor."
            } else {
                "Read-only preview: the original file cannot be overwritten."
            });
            return false;
        }
        if !self.documents.current().accepts_save(&text) {
            self.notify("Save refused: the editor is still synchronizing. Your file was not changed; try Save again when loading finishes.");
            return false;
        }
        let path = match self.path.clone().filter(|_| !save_as) {
            Some(p) => p,
            None => {
                let picked = crate::dialogs::FileDialog::new()
                    .set_directory(
                        self.path
                            .as_deref()
                            .and_then(Path::parent)
                            .unwrap_or(&self.tree_dir),
                    )
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

        let path = paths::normalize(&path);
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
            let choice = crate::dialogs::MessageDialog::new()
                .set_title("Changed on disk")
                .set_description("This file changed on disk since it was opened.\n\nOverwrite it?")
                .set_buttons(crate::dialogs::MessageButtons::YesNo)
                .show();
            if choice != crate::dialogs::MessageDialogResult::Yes {
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
        if self.format.mixed {
            let choice = crate::dialogs::MessageDialog::new().set_title("Mixed line endings")
                .set_description(format!("This file contains different line endings. Saving these changes will standardize them to {}. Continue?", self.format.ending))
                .set_buttons(crate::dialogs::MessageButtons::YesNoCancel).show();
            if choice != crate::dialogs::MessageDialogResult::Yes {
                self.notify("Save cancelled; original line endings preserved.");
                return false;
            }
        }
        let bytes = self.format.encode(&text);
        match storage::write_document(&path, &bytes) {
            Ok(report) => {
                self.format.mixed = false;
                self.fingerprint = Some(storage::fingerprint(&bytes));
                self.external_changed = false;
                self.disk.saved();
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
                self.settings.remember(&path);
                self.persist_settings();
                log::line(&format!("saved {}", path.display()));
                self.render_current(self.scroll);
                self.send_recents();
                if new_path {
                    self.send_tree(self.tree_dir.clone());
                }
                self.notify(report.warning.as_deref().unwrap_or("Saved"));
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
        if !Settings::editable_key(key) {
            self.send_settings();
            self.notify("That preference cannot be changed through this control.");
            return;
        }
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
            if matches!(key, "show_hidden" | "file_dates" | "file_sort_date") {
                self.send_tree(self.tree_dir.clone());
            }
            self.send_settings();
            if rerender {
                self.render_current(self.scroll);
            }
            self.persist_settings();
        } else {
            self.send_settings();
            self.notify("That preference value was not valid; the saved value was kept.");
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
                if let Some(doc) = self.documents.get_mut(id).filter(|doc| !doc.unloaded) {
                    self.view_changed.get_or_insert_with(Instant::now);
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
                    self.send_init();
                    self.send_recents();
                    self.send_tree(self.tree_dir.clone());
                    self.render_current(self.scroll);
                    return;
                }
                self.ui_ready = true;
                self.navigation_ready.set(true);
                log::line("UI handshake received");
                self.send_init();
                self.send_recents();
                let dir = self.tree_dir.clone();
                self.send_tree(dir);
                let paths = std::mem::take(&mut self.startup_paths);
                let welcome_id = self.id;
                for path in paths {
                    if self.documents.find_path(&path).is_some() {
                        continue;
                    }
                    let view = self
                        .session_restore
                        .iter()
                        .find(|view| paths::same(Path::new(&view.path), &path))
                        .cloned()
                        .unwrap_or_else(|| settings::SessionTab {
                            path: path.to_string_lossy().into_owned(),
                            ..Default::default()
                        });
                    self.documents.insert(Document::deferred(&view));
                }
                if self.documents.tabs.len() > 1 {
                    self.documents.remove(welcome_id);
                }
                self.startup_pending = 0;
                if let Some(target) = self.startup_target.take() {
                    if let Some(id) = self.documents.find_path(&target) {
                        self.documents.activate(id);
                    }
                }
                if !self.recovery_checked {
                    self.recovery_checked = true;
                    let drafts = recovery::read(&self.root);
                    if !drafts.is_empty() {
                        let choice = recovery::choice(
                            crate::dialogs::MessageDialog::new()
                                .set_title("Recover notes")
                                .set_description(format!(
                                    "{} unsaved note(s) were found. Restore them?",
                                    drafts.len()
                                ))
                                .set_buttons(crate::dialogs::MessageButtons::YesNoCancelCustom(
                                    "Restore".into(),
                                    "Discard".into(),
                                    "Later".into(),
                                ))
                                .show(),
                        );
                        for (key, draft) in drafts {
                            if choice == recovery::Choice::Restore {
                                let mut doc = Document::new(draft.path, draft.source);
                                doc.saved_source = draft.saved_source;
                                doc.format = draft.format;
                                doc.fingerprint = draft.fingerprint;
                                doc.dirty = true;
                                doc.editing = true;
                                doc.recovery_key = key;
                                if let Some(id) = doc
                                    .path
                                    .as_deref()
                                    .and_then(|p| self.documents.find_path(p))
                                {
                                    self.documents.activate(id);
                                    self.documents.replace(doc);
                                } else {
                                    self.documents.insert(doc);
                                }
                            } else if choice == recovery::Choice::Discard {
                                let _ = recovery::write(&self.root, &key, None);
                            }
                        }
                        if choice == recovery::Choice::Restore
                            && self.documents.tabs.len() > 1
                            && self.documents.tabs[0].path.is_none()
                            && !self.documents.tabs[0].dirty
                        {
                            let welcome_id = self.documents.tabs[0].id;
                            self.documents.remove(welcome_id);
                        }
                    }
                }
                self.activate_tab(self.id);
                self.persist_settings();
                if !self.settings.load_warning.is_empty() {
                    self.notify(&self.settings.load_warning);
                }
                #[cfg(feature = "smoke")]
                if self.smoke_started.is_some() {
                    self.run_js(
                        if std::env::var("SMOKE_SCENARIO").as_deref() == Ok("session") {
                            concat!(
                                "/* PRIVATE_NATIVE_SMOKE_BUILD_ONLY */",
                                include_str!("session_smoke.js")
                            )
                            .into()
                        } else {
                            concat!(
                                "/* PRIVATE_NATIVE_SMOKE_BUILD_ONLY */",
                                include_str!("smoke.js")
                            )
                            .into()
                        },
                    );
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
            "smokeDialog" if self.smoke_started.is_some() => {
                self.send_tree(self.tree_dir.clone());
                let result = crate::dialogs::MessageDialog::new()
                    .set_title("Recovery dialog test")
                    .set_description(
                        "Literal filename: 100% %s %n. Closing this dialog must keep drafts.",
                    )
                    .set_buttons(crate::dialogs::MessageButtons::YesNoCancelCustom(
                        "Restore".into(),
                        "Discard".into(),
                        "Later".into(),
                    ))
                    .show();
                self.run_js(format!(
                    "window.app.smokeDialogResult={};",
                    json!(format!("{:?}", recovery::choice(result)))
                ));
            }
            "smokeCreateUnavailable" if self.smoke_started.is_some() => {
                let _ = std::fs::write(self.root.join("missing-session.md"), "File is back.\n");
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
                self.run_js(format!("window.app.smokeState={};",json!({"token":value.get("token"),"workspace":self.settings.workspace,"savedTabs":self.settings.saved_tabs,"tabs":self.documents.tabs.iter().map(|doc|json!({"id":doc.id,"unloaded":doc.unloaded})).collect::<Vec<_>>(),"source":self.source,"disk":disk,"dirty":self.dirty,"recovery":recovery::read(&self.root).len()})));
            }
            "smokeReady" if self.smoke_started.is_some() => {
                self.persist_settings();
                let preferences_saved = self.preferences.flush();
                if value.get("ok").and_then(|v| v.as_bool()) == Some(true) && preferences_saved {
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
                if self._instance.is_some() {
                    instance::release(&self.root);
                }
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
                    if !self.documents.tabs.iter().any(|doc| doc.id == keep) {
                        return;
                    }
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
                                && doc.patch_index.apply(
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
                    self.recovery_activity = Instant::now();
                    self.recovery_pending.insert(id);
                    self.send_dirty(id);
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
                    self.recovery_activity = Instant::now();
                    self.recovery_pending.insert(id);
                    if changed {
                        self.send_dirty(id);
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
                if let Some(dir) = crate::dialogs::FileDialog::new()
                    .set_directory(&self.tree_dir)
                    .pick_folder()
                {
                    self.apply_setting("sidebar", json!("always"));
                    self.apply_setting("sidebar_tab", json!("files"));
                    self.send_tree(dir);
                }
            }
            "reload" => {
                if self.unloaded {
                    self.activate_tab(self.id);
                    return;
                }
                if let Some(path) = self.path.clone() {
                    self.open_file(path, false, true, String::new());
                } else {
                    self.notify("Save this note before reloading it.");
                }
            }
            "keepDisk" => {
                self.disk.acknowledge();
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
                    if let Some(path) = doc.path.clone().filter(|_| !doc.dirty) {
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
                        let mut doc = doc;
                        if doc
                            .path
                            .as_deref()
                            .is_some_and(|p| self.documents.find_path(p).is_some())
                        {
                            doc.path = None;
                        }
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
            "toggleFiles" => {
                if self.settings.sidebar != "off" && self.settings.sidebar_tab == "files" {
                    self.settings.sidebar_return = self.settings.sidebar.clone();
                    self.settings.sidebar = "off".into();
                } else {
                    self.settings.sidebar = self.settings.sidebar_return.clone();
                    self.settings.sidebar_tab = "files".into();
                }
                self.send_settings();
                self.persist_settings();
            }
            "cycleSidebar" => {
                self.settings.cycle_sidebar();
                self.send_settings();
                self.persist_settings();
            }
            "quickSearch" => {
                self.quick_request = value.get("request").and_then(|v| v.as_u64()).unwrap_or(0);
                self.workspace_worker.search(workspace_files::Search {
                    request: self.quick_request,
                    root: self.tree_dir.clone(),
                    query: value
                        .get("query")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .chars()
                        .take(256)
                        .collect(),
                    hidden: self.settings.show_hidden,
                });
            }
            "cancelQuickSearch" => {
                self.quick_request = self.quick_request.wrapping_add(1);
                self.workspace_worker.cancel_search(self.quick_request);
            }
            "fileAction" => {
                if self.startup_pending > 0
                    || !matches!(
                        value.get("kind").and_then(|v| v.as_str()),
                        Some("rename" | "folder")
                    )
                {
                    self.run_js(format!("window.app.fileOperation({});",json!({"request":value.get("request"),"error":"Wait for files to finish opening, then try again."})));
                    return;
                }
                let request = value.get("request").and_then(|v| v.as_u64()).unwrap_or(0);
                let mut path =
                    PathBuf::from(value.get("path").and_then(|v| v.as_str()).unwrap_or(""));
                if value.get("parent").and_then(|v| v.as_bool()) == Some(true) {
                    path = path.parent().unwrap_or(&path).to_path_buf();
                }
                let operation = workspace_files::Operation {
                    request,
                    root: self.tree_dir.clone(),
                    path,
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .into(),
                    rename: value.get("kind").and_then(|v| v.as_str()) == Some("rename"),
                };
                if self.file_request.is_some() || !self.workspace_worker.operate(operation) {
                    self.run_js(format!("window.app.fileOperation({});",json!({"request":request,"error":"Another file operation is still finishing."})));
                } else {
                    self.file_request = Some(request);
                }
            }
            "refreshTree" => self.send_tree(self.tree_dir.clone()),
            "openPath" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    if value.get("link").and_then(|v| v.as_bool()) == Some(true)
                        && !self.approve_network_path(Path::new(path))
                    {
                        return;
                    }
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
                        dates: self.settings.file_dates,
                        sort_date: self.settings.file_sort_date,
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
                self.settings.load_warning = old.load_warning;
                self.settings.extra = old.extra;
                if old.settings_version > 2 {
                    self.settings.settings_version = old.settings_version;
                }
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
        let choice = crate::dialogs::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description(format!(
                "Save changes to {} before continuing?",
                self.name()
            ))
            .set_buttons(crate::dialogs::MessageButtons::YesNoCancelCustom(
                "Save".into(),
                "Discard".into(),
                "Cancel".into(),
            ))
            .show();
        match choice {
            crate::dialogs::MessageDialogResult::Yes => self.save(self.source.clone(), false),
            crate::dialogs::MessageDialogResult::No => true,
            crate::dialogs::MessageDialogResult::Custom(label) if label == "Save" => {
                self.save(self.source.clone(), false)
            }
            crate::dialogs::MessageDialogResult::Custom(label) if label == "Discard" => true,
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

fn main() {
    if let Err(error) = run() {
        log::line(&format!("Startup failed: {error}"));
        eprintln!("Startup failed: {error}");
        dialogs::startup_error(&error.to_string());
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let root = root::app_root()?;
    if let Err(error) = std::fs::create_dir_all(&root) {
        eprintln!("Application data folder unavailable: {error}");
    }
    log::init(&root);
    let prior_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::line(&format!("Fatal error: {info}"));
        prior_hook(info);
    }));
    log::line(&format!("start root={}", root.display()));

    let arguments: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .filter(|arg| !(cfg!(target_os = "macos") && arg.to_string_lossy().starts_with("-psn_")))
        .take(128)
        .map(PathBuf::from)
        .map(|p| paths::absolute(&p))
        .collect();
    if arguments.iter().any(|path| path.to_str().is_none()) {
        return Err("A filename contains an unsupported character encoding. Rename it in your file manager before opening it.".into());
    }
    let instance_guard = match instance::acquire(&root) {
        Ok(guard) => Some(guard),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
            let deadline = Instant::now() + std::time::Duration::from_secs(8);
            let acquired = loop {
                if instance::hand_off(&root, &arguments) {
                    return Ok(());
                }
                if let Ok(guard) = instance::acquire(&root) {
                    break guard;
                }
                if Instant::now() >= deadline {
                    return Err("The running copy did not respond. Check its window before opening another copy.".into());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            };
            Some(acquired)
        }
        Err(error) => {
            log::line(&format!(
                "Single-instance locking unavailable: {error}. Continuing without hand-off."
            ));
            None
        }
    };

    let settings = Settings::load(&root);
    let font_list = Vec::new();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let font_proxy = proxy.clone();
    std::thread::spawn(move || {
        let _ = font_proxy.send_event(UserEvent::FontsLoaded(fonts::families()));
    });
    let render_worker = jobs::RenderWorker::new(proxy.clone());
    let io = jobs::IoWorker::new(proxy.clone());
    let workspace_worker = workspace_files::Worker::new(proxy.clone());
    let preferences = preferences::Writer::new(root.clone(), proxy.clone());

    let handoff_proxy = proxy.clone();
    if instance_guard.is_some() {
        if let Err(error) = instance::listen(&root, move |paths| {
            let _ = handoff_proxy.send_event(UserEvent::Handoff(paths));
        }) {
            log::line(&format!("Single-instance listener unavailable: {error}"));
        }
    }

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
        .with_download_started_handler(|_, _| false)
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

    #[cfg(target_os = "windows")]
    let builder = {
        use wry::WebViewBuilderExtWindows;
        builder.with_browser_accelerator_keys(false)
    };

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().ok_or("no gtk container")?)?
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    let session_restore = settings.saved_tabs.clone();
    let mut startup_paths: Vec<PathBuf> = if settings.restore_last_file && settings.restore_tabs {
        session_restore
            .iter()
            .map(|view| paths::display_form(Path::new(&view.path)))
            .collect()
    } else {
        Vec::new()
    };
    if startup_paths.is_empty() && settings.restore_last_file && !settings.last_path.is_empty() {
        startup_paths.push(paths::display_form(Path::new(&settings.last_path)));
    }
    for argument in &arguments {
        if !startup_paths.iter().any(|p| paths::same(p, argument)) {
            startup_paths.push(argument.clone());
        }
    }
    let startup_target = arguments.last().cloned().or_else(|| {
        (!settings.last_path.is_empty())
            .then(|| paths::display_form(Path::new(&settings.last_path)))
    });
    let tree_dir = if !settings.workspace.is_empty() {
        paths::display_form(Path::new(&settings.workspace))
    } else {
        startup_target
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(Path::new(""))
            .to_owned()
    };
    let source = welcome();

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
        smoke_started: (root::smoke_authorized(&root)
            && !arguments.is_empty()
            && arguments.iter().all(|path| path.starts_with(&root)))
        .then_some(started),
        settings,
        preferences,
        preference_serial: 0,
        view_changed: None,
        preview_theme: None,
        render_worker,
        io,
        workspace_worker,
        quick_request: 0,
        file_request: None,
        generation: 0,
        tree_serial: 0,
        closed_tabs: Vec::new(),
        session_restore,
        startup_pending: startup_paths.len(),
        startup_target,
        startup_paths,
        recovery_checked: false,
        recovery_pending: Default::default(),
        recovery_flush: Instant::now(),
        recovery_activity: Instant::now(),
        last_disk_check: Instant::now(),
        disk_check_pending: false,
        fonts: font_list,
        fonts_loaded: false,
        window,
        webview,
    };

    log::line(&format!(
        "window ready in {} ms",
        started.elapsed().as_millis()
    ));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if !app.ui_ready&&!app.boot_probe&&app.boot_started.elapsed()>std::time::Duration::from_secs(3){
            app.boot_probe=true;
            let _=app.webview.evaluate_script_with_callback("JSON.stringify({url:location.href,readyState:document.readyState,app:!!window.app,ipc:!!window.ipc,nativeBridge:!!window.chrome?.webview,bodyLength:document.body?.innerHTML.length,errors:window.startupErrors||[]})",|result|log::line(&format!("startup diagnostic: {}",result)));
            app.run_js("window.app?.requestReady?.();".into());
        }
        if app.view_changed.is_some_and(|time| time.elapsed() > std::time::Duration::from_millis(500)) { app.persist_settings(); }
        if app.window.is_focused(){app.check_disk(false);}
        if !app.recovery_pending.is_empty() && Instant::now() >= app.recovery_due() { app.flush_recovery(); }
        match event {
            Event::UserEvent(UserEvent::Rendered{generation,tab,revision,key,document}) if generation==app.generation&&tab==app.id&&revision==app.edit_revision => {
                    let fragment=std::mem::take(&mut app.pending_fragment);
                    app.run_js(format!("window.app.finishDocument({});",json!({"tab":tab,"revision":revision,"generation":generation,"renderKey":key,"html":document.html,"outline":document.outline,"frontMatter":document.front_matter,"note":if app.read_only{"Large file: showing a read-only preview of the first 256 KB.".into()}else{document.note},"fragment":fragment})));
            }
            Event::UserEvent(UserEvent::Rendered { generation, tab, .. }) if generation == app.generation && tab == app.id => { app.render_current(app.scroll); }
            Event::UserEvent(UserEvent::Loaded{task,result})=>{
                match result {
                    Ok(loaded) => app.finish_open(task, loaded),
                    Err(error) => {
                        if task.restore {
                            if let Some(doc) = app.documents.get_mut(task.from) { doc.loading = false; doc.load_error = Some(error.clone()); }
                            if app.id == task.from { app.render_current(app.scroll); }
                        }
                        app.notify(&format!("Cannot open {}: {}", task.path.display(), error));
                    }
                }
            },
            Event::UserEvent(UserEvent::QuickMatches(result)) if result.request == app.quick_request => {
                app.run_js(format!("window.app.quickMatches({});", json!(result)));
            }
            Event::UserEvent(UserEvent::FileOperation(result)) => app.finish_file_operation(result),
            Event::UserEvent(UserEvent::TreeLoaded{path,request,root,serial,result})=>{
                if root{if serial==app.tree_serial{match result{Ok(entries)=>{if !paths::same(Path::new(&app.settings.workspace), &path) {app.settings.expanded_folders.clear();app.settings.tree_scroll=0.0;app.settings.tree_filter.clear();}app.tree_dir=path.clone();app.settings.workspace=path.to_string_lossy().into_owned();app.run_js(format!("window.app.setTree({});",json!({"dir":path.to_string_lossy(),"parent":path.parent().is_some(),"entries":entries,"view":{"expanded":app.settings.expanded_folders,"scroll":app.settings.tree_scroll,"filter":app.settings.tree_filter}})));app.persist_settings();},Err(error)=>{app.run_js(format!("window.app.setTree({});",json!({"dir":path.to_string_lossy(),"parent":path.parent().is_some(),"entries":[],"error":error})));app.notify(&format!("Cannot read folder: {}",error));}}}}
                else{let(entries,error)=match result{Ok(entries)=>(entries,None),Err(error)=>(Vec::new(),Some(error))};app.run_js(format!("window.app.setEntries({});",json!({"path":path.to_string_lossy(),"request":request,"entries":entries,"error":error})));}
            }
            Event::UserEvent(UserEvent::DiskChecked { tab, path, epoch, observed }) => {
                app.disk_check_pending = false;
                if let Some(doc) = app.documents.get_mut(tab) {
                    if doc.disk.epoch == epoch && doc.path.as_ref() == Some(&path) {
                        if let Some(baseline) = doc.fingerprint {
                            doc.external_changed = doc.disk.observe(baseline, observed);
                        }
                    }
                }
                if app.id == tab { app.run_js(format!("window.app.diskStatus({});", app.external_changed)); }
            }
            Event::UserEvent(UserEvent::RecoveryStatus(error)) => app.run_js(format!("window.app.recoveryStatus({});", json!(error))),
            Event::WindowEvent{event:WindowEvent::Focused(true),..}=>app.check_disk(true),
            #[cfg(target_os="macos")]
            Event::Opened{urls}=>{app.show();for url in urls{if let Ok(path)=url.to_file_path(){app.queue_launch(path);}}}
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
            Event::UserEvent(UserEvent::PreferencesSaved { serial, error }) if serial == app.preference_serial => {
                    if let Some(message) = &error { log::line(&format!("Preferences save failed: {message}")); }
                    app.run_js(format!("window.app.settingsStatus({});", json!({"ok":error.is_none(),"message":error.map(|e|format!("Preferences not saved: {e}")).unwrap_or_default()})));
            }
            Event::UserEvent(UserEvent::FontsLoaded(fonts)) => {
                app.fonts = fonts; app.fonts_loaded = true;
                if app.ui_ready { app.run_js(format!("window.app.setFonts({});", json!(app.fonts))); }
            }
            Event::UserEvent(UserEvent::BrowserLoaded)=>{app.run_js("window.app?.requestReady?.();".into());}
            Event::UserEvent(UserEvent::Page(message)) => app.handle(&message, control_flow),
            Event::UserEvent(UserEvent::Dropped(path)) => app.open(path, true),
            Event::UserEvent(UserEvent::Handoff(paths)) => {
                app.show();
                for path in paths { let path = PathBuf::from(path); if app.approve_network_path(&path) { app.queue_launch(path); } }
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
                app.remember_window();
                app.view_changed.get_or_insert_with(Instant::now);
                app.run_js(format!(
                    "window.app && window.app.windowState({});",
                    app.window.is_maximized()
                ));
            }
            #[cfg(target_os = "windows")]
            Event::UserEvent(UserEvent::Tray(command)) => {
                if command != "quit" { app.show(); }
                match command.as_str() {
                    "new" => app.new_note(),
                    "open" => app.pick_and_open(),
                    "quit" => app.quit(control_flow),
                    _ => {}
                }
            }
            _ => {}
        }
        if *control_flow != ControlFlow::Exit {
            let mut deadline: Option<Instant> = None;
            let mut include = |time| { deadline = Some(deadline.map_or(time, |current| current.min(time))); };
            if !app.ui_ready && !app.boot_probe { include(app.boot_started + std::time::Duration::from_secs(3)); }
            if app.window.is_focused() && !app.disk_check_pending && app.fingerprint.is_some() && !app.read_only { include(app.last_disk_check + std::time::Duration::from_secs(2)); }
            if !app.recovery_pending.is_empty() { include(app.recovery_due()); }
            if let Some(time) = app.view_changed { include(time + std::time::Duration::from_millis(500)); }
            if let Some(time) = deadline { *control_flow = ControlFlow::WaitUntil(time); }
        }
    });
}
