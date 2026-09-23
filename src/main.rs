// No console window on Windows: this is a windowed app, and its log goes to a
// file beside the binary.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod log;
mod render;
mod root;
mod settings;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use render::Renderer;
use settings::Settings;

const WELCOME: &str = "# Notepad\n\nPress **Open** in the bar above, or Ctrl+O, \
or drag a file onto this window.\n";

/// Messages the page sends back to the application.
#[derive(Debug)]
enum UserEvent {
    Open,
    TreeVisible(bool),
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Reads a file and turns it into a page. A failure is rendered as a document
/// so the window never just sits there blank.
fn document_for(renderer: &Renderer, settings: &Settings, path: &Path) -> (String, String) {
    let name = display_name(path);
    match std::fs::read_to_string(path) {
        Ok(text) => {
            log::line(&format!("open {} ({} bytes)", path.display(), text.len()));
            (
                format!("{} - {}", name, root::app_name()),
                renderer.to_page(&text, settings, &name),
            )
        }
        Err(e) => {
            log::line(&format!("open failed {}: {}", path.display(), e));
            let body = format!("# Cannot open\n\n`{}`\n\n{}\n", path.display(), e);
            (
                root::app_name().to_string(),
                renderer.to_page(&body, settings, &name),
            )
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();

    let root = root::app_root();
    log::init(&root);
    log::line(&format!("start root={}", root.display()));

    let mut settings = Settings::load(&root);
    let renderer = Renderer::new();
    let current: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));

    let arg = std::env::args().nth(1).map(PathBuf::from);
    let (title, page) = match &arg {
        Some(p) => {
            *current.borrow_mut() = Some(p.clone());
            document_for(&renderer, &settings, p)
        }
        None => (
            root::app_name().to_string(),
            renderer.to_page(WELCOME, &settings, "no file open"),
        ),
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(tao::dpi::LogicalSize::new(
            settings.window_width as f64,
            settings.window_height as f64,
        ))
        .build(&event_loop)?;

    let proxy = event_loop.create_proxy();
    let builder = WebViewBuilder::new()
        .with_html(page)
        .with_ipc_handler(move |request| {
            let message = request.body().as_str();
            let event = match message {
                "open" => Some(UserEvent::Open),
                "tree:1" => Some(UserEvent::TreeVisible(true)),
                "tree:0" => Some(UserEvent::TreeVisible(false)),
                _ => None,
            };
            if let Some(event) = event {
                let _ = proxy.send_event(event);
            }
        });

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().ok_or("no gtk container")?)?
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window)?;

    log::line(&format!("window ready in {} ms", started.elapsed().as_millis()));
    if std::env::var("EXIT_WHEN_READY").is_ok() {
        println!("READY_MS={}", started.elapsed().as_millis());
        let _ = settings.save(&root);
        return Ok(());
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        let show = |path: PathBuf| {
            let (title, page) = document_for(&renderer, &settings, &path);
            window.set_title(&title);
            let _ = webview.load_html(&page);
            *current.borrow_mut() = Some(path);
        };

        match event {
            Event::UserEvent(UserEvent::Open) => {
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("Markdown", &["md", "markdown", "mdown", "mkd"])
                    .add_filter("Text", &["txt"])
                    .add_filter("All files", &["*"]);
                let start_dir = current
                    .borrow()
                    .as_ref()
                    .and_then(|p| p.parent().map(PathBuf::from));
                if let Some(dir) = start_dir {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(picked) = dialog.pick_file() {
                    show(picked);
                }
            }
            Event::UserEvent(UserEvent::TreeVisible(visible)) => {
                settings.sidebar_visible = visible;
            }
            Event::WindowEvent { event: WindowEvent::DroppedFile(path), .. } => show(path),
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                let _ = settings.save(&root);
                log::line("exit");
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
