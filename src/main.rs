mod log;
mod render;
mod root;
mod settings;

use std::path::PathBuf;
use std::time::Instant;

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use render::Renderer;
use settings::Settings;

const WELCOME: &str = "# Notepad\n\nOpen a file to begin.\n\nPass a path on the \
command line, or use File then Open once the menu lands in a later stage.\n";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();

    let root = root::app_root();
    log::init(&root);
    log::line(&format!("start root={}", root.display()));

    let settings = Settings::load(&root);
    let renderer = Renderer::new();

    let path = std::env::args().nth(1).map(PathBuf::from);
    let (title, markdown) = match &path {
        Some(p) => {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string());
            match std::fs::read_to_string(p) {
                Ok(text) => {
                    log::line(&format!("open {} ({} bytes)", p.display(), text.len()));
                    (format!("{} - {}", name, root::app_name()), text)
                }
                Err(e) => {
                    log::line(&format!("open failed {}: {}", p.display(), e));
                    (
                        root::app_name().to_string(),
                        format!("# Cannot open\n\n`{}`\n\n{}\n", p.display(), e),
                    )
                }
            }
        }
        None => (root::app_name().to_string(), WELCOME.to_string()),
    };

    let page = renderer.to_page(&markdown, &settings);
    log::line(&format!("rendered in {} ms", started.elapsed().as_millis()));

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(tao::dpi::LogicalSize::new(
            settings.window_width as f64,
            settings.window_height as f64,
        ))
        .build(&event_loop)?;

    let builder = WebViewBuilder::new().with_html(page);

    #[cfg(target_os = "linux")]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().ok_or("no gtk container")?)?
    };
    #[cfg(not(target_os = "linux"))]
    let _webview = builder.build(&window)?;

    log::line(&format!("window ready in {} ms", started.elapsed().as_millis()));
    if std::env::var("EXIT_WHEN_READY").is_ok() {
        println!("READY_MS={}", started.elapsed().as_millis());
        let _ = settings.save(&root);
        return Ok(());
    }

    let save_root = root.clone();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            let _ = settings.save(&save_root);
            log::line("exit");
            *control_flow = ControlFlow::Exit;
        }
    });
}
