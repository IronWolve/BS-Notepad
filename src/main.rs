// No console window on Windows: this is a windowed app, and its log goes to a
// file beside the binary.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod fonts;
mod instance;
mod log;
mod render;
mod root;
mod settings;
mod theme;
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

use render::Renderer;
use settings::Settings;

const WELCOME: &str = "# Notepad\n\nOpen a file from the bar, the tree on the \
left, or by dragging one onto this window.\n";

#[derive(Debug)]
enum UserEvent {
    /// A message from the page.
    Page(String),
    /// A second launch handed us a path instead of starting its own window.
    Handoff(Option<String>),
}

struct App {
    root: PathBuf,
    settings: Settings,
    renderer: Renderer,
    fonts: Vec<fonts::FontFamily>,
    path: Option<PathBuf>,
    source: String,
    seen_mtime: Option<SystemTime>,
    dirty: bool,
    tree_dir: PathBuf,
    window: Window,
    webview: WebView,
}

fn open_externally(url: &str) {
    // Deliberately not navigating the window: an external link belongs in the
    // browser, and the document view should never leave the document.
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()
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
        t.dim = theme::guard(&t.dim, &t.panel, 3.0);
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
        let entries = tree::list(&dir);
        let payload = json!({ "dir": dir.display().to_string(), "entries": entries });
        self.tree_dir = dir;
        self.run_js(format!("window.app.setTree({});", payload));
    }

    fn render_current(&mut self, scroll: f32) {
        let t = self.theme();
        let document = self.renderer.render(
            self.path.as_deref(),
            &self.source,
            &self.settings,
            &t,
        );
        let name = self
            .path
            .as_ref()
            .map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()))
            .unwrap_or_else(|| "no file open".into());

        let payload = json!({
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
            serde_json::to_string(&self.source).unwrap_or_else(|_| "\"\"".into())
        ));

        let title = match &self.path {
            Some(_) => format!("{} - {}", name, root::app_name()),
            None => root::app_name().to_string(),
        };
        self.window.set_title(&title);
    }

    fn open(&mut self, path: PathBuf) {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                log::line(&format!("open {} ({} bytes)", path.display(), size));
                self.source = text;
                self.seen_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                assets::set_scope(path.parent());
                self.settings.remember(&path);
                self.path = Some(path.clone());
                self.dirty = false;
                self.render_current(0.0);
                self.send_recents();
                if let Some(parent) = path.parent() {
                    if parent != self.tree_dir {
                        let dir = parent.to_path_buf();
                        self.send_tree(dir);
                    }
                }
            }
            Err(e) => {
                log::line(&format!("open failed {}: {}", path.display(), e));
                self.source = format!("# Cannot open\n\n`{}`\n\n{}\n", path.display(), e);
                self.path = None;
                self.render_current(0.0);
            }
        }
    }

    fn pick_and_open(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "mdown", "mkd", "mkdn"])
            .add_filter("Text and source", &["txt", "rs", "py", "js", "ts", "go", "c", "h",
                                             "cpp", "json", "toml", "yaml", "yml", "sh"])
            .add_filter("All files", &["*"]);
        if let Some(dir) = self.path.as_ref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(picked) = dialog.pick_file() {
            self.open(picked);
        }
    }

    /// Writes through a temporary file in the same folder, then renames: an
    /// interrupted save cannot leave the note truncated.
    fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
        let tmp = path.with_extension(format!(
            "{}.tmp",
            path.extension().and_then(|e| e.to_str()).unwrap_or("save")
        ));
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }

    fn save(&mut self, text: String) {
        let path = match self.path.clone() {
            Some(p) => p,
            None => {
                let picked = rfd::FileDialog::new()
                    .set_file_name("untitled.md")
                    .add_filter("Markdown", &["md"])
                    .save_file();
                match picked {
                    Some(p) => p,
                    None => return,
                }
            }
        };

        // If the file moved underneath us, say so rather than overwriting.
        let current = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if let (Some(seen), Some(now)) = (self.seen_mtime, current) {
            if now != seen {
                let choice = rfd::MessageDialog::new()
                    .set_title("Changed on disk")
                    .set_description(
                        "This file changed on disk since it was opened.\n\nOverwrite it?",
                    )
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .show();
                if choice != rfd::MessageDialogResult::Yes {
                    self.run_js("window.app.note('save cancelled - file changed on disk');".into());
                    return;
                }
            }
        }

        match Self::write_atomically(&path, &text) {
            Ok(()) => {
                self.source = text;
                self.path = Some(path.clone());
                self.seen_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.dirty = false;
                self.settings.remember(&path);
                log::line(&format!("saved {}", path.display()));
                self.render_current(0.0);
                self.send_recents();
                self.run_js("window.app.note('saved');".into());
            }
            Err(e) => {
                log::line(&format!("save failed {}: {}", path.display(), e));
                self.run_js(format!(
                    "window.app.note({});",
                    serde_json::to_string(&format!("save failed: {}", e)).unwrap_or_default()
                ));
            }
        }
    }

    fn apply_setting(&mut self, key: &str, value: serde_json::Value) {
        let mut current = serde_json::to_value(&self.settings).unwrap_or(json!({}));
        if let Some(map) = current.as_object_mut() {
            map.insert(key.to_string(), value);
        }
        if let Ok(updated) = serde_json::from_value::<Settings>(current) {
            let rerender = matches!(
                key,
                "theme" | "highlight_limit_kb" | "plain_text_above_mb"
                    | "view_mode" | "syntax_colour"
            );
            self.settings = updated;
            self.send_settings();
            if rerender {
                self.render_current(self.settings.last_scroll);
            }
            let _ = self.settings.save(&self.root);
        }
    }

    fn handle(&mut self, message: &str, control_flow: &mut ControlFlow) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(message) else { return };
        let command = value.get("cmd").and_then(|c| c.as_str()).unwrap_or("");
        if command != "scroll" {
            log::line(&format!("page: {}", command));
        }

        match command {
            "ready" => {
                self.send_init();
                self.send_recents();
                let dir = self.tree_dir.clone();
                self.send_tree(dir);
                let scroll = self.settings.last_scroll;
                self.render_current(scroll);
            }
            "open" => self.pick_and_open(),
            "openPath" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    self.open(PathBuf::from(path));
                }
            }
            "expand" => {
                if let Some(path) = value.get("path").and_then(|p| p.as_str()) {
                    let entries = tree::list(Path::new(path));
                    self.run_js(format!(
                        "window.app.setEntries({});",
                        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into())
                    ));
                }
            }
            "treeUp" => {
                let parent = self.tree_dir.parent().map(PathBuf::from);
                if let Some(dir) = parent {
                    self.send_tree(dir);
                }
            }
            "save" => {
                let text = value.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                self.save(text);
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
                let recents = self.settings.recents.clone();
                self.settings = Settings::default();
                self.settings.recents = recents;
                self.send_settings();
                self.render_current(0.0);
                let _ = self.settings.save(&self.root);
            }
            "scroll" => {
                if let Some(v) = value.get("value").and_then(|v| v.as_f64()) {
                    self.settings.last_scroll = v as f32;
                }
            }
            "quit" => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    }

    /// Asks before losing edits.
    fn may_close(&self) -> bool {
        if !self.dirty {
            return true;
        }
        rfd::MessageDialog::new()
            .set_title("Unsaved changes")
            .set_description("This document has unsaved changes.\n\nClose without saving?")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
            == rfd::MessageDialogResult::Yes
    }

    fn remember_window(&mut self) {
        let size = self.window.inner_size();
        if size.width > 200 && size.height > 200 {
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

    let argument = std::env::args().nth(1).map(PathBuf::from);

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
        .with_inner_size(tao::dpi::LogicalSize::new(
            settings.window_width as f64,
            settings.window_height as f64,
        ))
        .with_maximized(settings.window_maximized)
        .build(&event_loop)?;

    let ipc_proxy = proxy.clone();
    let builder = WebViewBuilder::new()
        .with_html(ui::SHELL)
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
    let initial = argument.clone().or_else(|| {
        if settings.restore_last_file && !settings.last_path.is_empty() {
            let candidate = PathBuf::from(&settings.last_path);
            candidate.exists().then_some(candidate)
        } else {
            None
        }
    });
    let tree_dir = initial
        .as_ref()
        .and_then(|p| p.parent().map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| root.clone());

    let source = match &initial {
        Some(p) => std::fs::read_to_string(p).unwrap_or_else(|_| WELCOME.to_string()),
        None => WELCOME.to_string(),
    };
    if let Some(p) = &initial {
        assets::set_scope(p.parent());
        settings.remember(p);
    }

    let mut app = App {
        seen_mtime: initial.as_ref().and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok()),
        path: initial,
        source,
        dirty: false,
        tree_dir,
        root: root.clone(),
        settings,
        renderer,
        fonts: font_list,
        window,
        webview,
    };

    log::line(&format!("window ready in {} ms", started.elapsed().as_millis()));
    if std::env::var("EXIT_WHEN_READY").is_ok() {
        println!("READY_MS={}", started.elapsed().as_millis());
        let _ = app.settings.save(&root);
        instance::release(&root);
        return Ok(());
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Page(message)) => app.handle(&message, control_flow),
            Event::UserEvent(UserEvent::Handoff(path)) => {
                app.window.set_focus();
                if let Some(path) = path {
                    app.open(PathBuf::from(path));
                }
            }
            Event::WindowEvent { event: WindowEvent::DroppedFile(path), .. } => app.open(path),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                if app.may_close() {
                    app.remember_window();
                    let _ = app.settings.save(&app.root);
                    instance::release(&app.root);
                    log::line("exit");
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}
