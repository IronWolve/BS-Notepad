use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tmp")
            .join("tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn tree_handles_hidden_and_extensionless_files_and_reports_errors() {
    let f = Fixture::new();
    std::fs::create_dir(f.0.join("folder")).unwrap();
    for name in ["z.md", "A.txt", "Makefile", ".env"] {
        std::fs::write(f.0.join(name), "text").unwrap();
    }
    let entries = tree::list(&f.0, false).unwrap();
    assert_eq!(
        entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["folder", "A.txt", "Makefile", "z.md"]
    );
    assert!(
        entries
            .iter()
            .find(|e| e.name == "Makefile")
            .unwrap()
            .openable
    );
    assert_eq!(tree::list(&f.0, true).unwrap().len(), 5);
    assert!(tree::list(&f.0.join("missing"), false).is_err());
}
#[test]
fn save_replaces_content_and_preserves_unrelated_temp_files() {
    let f = Fixture::new();
    let path = f.0.join("note.md");
    std::fs::write(&path, "old").unwrap();
    std::fs::write(f.0.join("note.md.tmp"), "keep me").unwrap();
    App::write_atomically(&path, "new\nUnicode: café ✎").unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "new\nUnicode: café ✎"
    );
    assert_eq!(
        std::fs::read_to_string(f.0.join("note.md.tmp")).unwrap(),
        "keep me"
    );
    assert_eq!(std::fs::read_dir(&f.0).unwrap().count(), 2);
}
#[test]
fn failed_save_cleans_temp_file_and_preserves_target() {
    let f = Fixture::new();
    let path = f.0.join("directory");
    std::fs::create_dir(&path).unwrap();
    assert!(App::write_atomically(&path, "cannot replace a directory").is_err());
    assert!(path.is_dir());
    assert_eq!(std::fs::read_dir(&f.0).unwrap().count(), 1);
}
#[test]
fn settings_upgrade_reveals_controls_and_keeps_user_fonts() {
    let f = Fixture::new();
    std::fs::write(
        f.0.join("settings.json"),
        r#"{"chrome":"auto","sidebar":"auto","code_font":"Example Mono","recents":["note.md"]}"#,
    )
    .unwrap();
    let s = Settings::load(&f.0);
    assert_eq!(s.chrome, "always");
    assert_eq!(s.sidebar, "always");
    assert_eq!(s.code_font, "Example Mono");
    assert_eq!(s.recents, ["note.md"]);
}
#[test]
fn settings_persist_autohide_and_clamp_invalid_sizes() {
    let f = Fixture::new();
    let mut s = Settings::default();
    s.chrome = "auto".into();
    s.sidebar = "off".into();
    s.sidebar_width = 0;
    s.tab_size = 100;
    s.zoom = -1.;
    s.save(&f.0).unwrap();
    let s = Settings::load(&f.0);
    assert_eq!(s.chrome, "auto");
    assert_eq!(s.sidebar, "off");
    assert_eq!(s.sidebar_width, 180);
    assert_eq!(s.tab_size, 8);
    assert_eq!(s.zoom, 0.5);
    s.save(&f.0).unwrap();
    assert!(!f.0.join("settings.json.tmp").exists());
}
#[test]
fn corrupt_settings_recover_to_usable_defaults() {
    let f = Fixture::new();
    std::fs::write(f.0.join("settings.json"), "{broken").unwrap();
    let s = Settings::load(&f.0);
    assert_eq!(s.sidebar, "always");
    assert_eq!(s.ui_size, 13);
}
#[test]
fn markdown_cannot_inject_controls_into_the_shell() {
    let renderer = Renderer::new();
    let d = renderer.render(
        Some(Path::new("note.md")),
        "# Note\n\n<script>window.ipc.postMessage('quit')</script>\n\n<img onerror=evil() src=x>",
        &Settings::default(),
        &theme::find("dark"),
    );
    assert!(!d.html.contains("<script>"));
    assert!(!d.html.contains("<img onerror"));
    assert!(d.html.contains("&lt;script&gt;"));
}
#[test]
fn icons_have_transparency_and_visible_pixels() {
    for size in [16, 24, 32, 48, 64, 256] {
        let pixels = icon::rgba(size);
        assert_eq!(pixels.len(), (size * size * 4) as usize);
        assert_eq!(pixels[3], 0);
        assert!(pixels.chunks_exact(4).any(|p| p[3] == 255));
    }
}
