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

#[test]
fn medium_themes_keep_readable_text_and_toned_code_panels() {
    let renderer = Renderer::new();
    for id in ["mist", "sage", "slate", "graphite"] {
        let theme = theme::find(id);
        assert_eq!(theme.id, id);
        assert!(
            theme::contrast(&theme.fg, &theme.bg) >= 4.5,
            "{} body text",
            id
        );
        assert!(
            theme::contrast(&theme.dim, &theme.panel) >= 4.5,
            "{} secondary text",
            id
        );
        let doc = renderer.render(
            Some(Path::new("sample.rs")),
            "// A comment\nfn main() { let message = \"Hello\"; println!(\"{}\", message); }",
            &Settings::default(),
            &theme,
        );
        assert!(
            doc.html
                .contains(&format!("background-color:{}", theme.panel)),
            "{} code background",
            id
        );
        let mut colours = 0;
        for fragment in doc.html.split("style=\"").skip(1) {
            let style = fragment.split('"').next().unwrap();
            for property in style.split(';') {
                if let Some(fg) = property.strip_prefix("color:") {
                    assert!(
                        theme::contrast(fg, &theme.panel) >= 4.5,
                        "{} token {}",
                        id,
                        fg
                    );
                    colours += 1;
                }
            }
        }
        assert!(colours > 0);
    }
}
#[test]
fn contrast_guard_chooses_readable_text_on_middle_gray() {
    let background = "#888888";
    let foreground = theme::guard("#999999", background, 4.5);
    assert!(theme::contrast(&foreground, background) >= 4.5);
}

#[test]
fn tabs_keep_independent_drafts_and_view_state() {
    let mut tabs = Documents::new(Document::new(
        Some(PathBuf::from("first.md")),
        "First".into(),
    ));
    tabs.current_mut().edit("First draft".into());
    tabs.current_mut().editing = true;
    tabs.current_mut().scroll = 0.4;
    tabs.current_mut().selection_start = 3;
    let first = tabs.current().id;
    tabs.insert(Document::new(
        Some(PathBuf::from("second.md")),
        "Second".into(),
    ));
    let second = tabs.current().id;
    tabs.current_mut().edit("Second draft".into());
    assert!(tabs.activate(first));
    assert_eq!(tabs.current().source, "First draft");
    assert_eq!(tabs.current().saved_source, "First");
    assert!(tabs.current().dirty && tabs.current().editing);
    assert_eq!(tabs.current().scroll, 0.4);
    assert_eq!(tabs.current().selection_start, 3);
    assert!(tabs.activate(second));
    assert_eq!(tabs.current().source, "Second draft");
    assert_eq!(tabs.find_path(Path::new("first.md")), Some(first));
}
#[test]
fn replacing_or_closing_tabs_retires_stale_document_ids() {
    let mut tabs = Documents::new(Document::new(None, "Old".into()));
    let old = tabs.current().id;
    tabs.replace(Document::new(None, "New".into()));
    assert_ne!(tabs.current().id, old);
    assert!(tabs.get_mut(old).is_none());
    let first = tabs.current().id;
    tabs.insert(Document::new(None, "Other".into()));
    let second = tabs.current().id;
    tabs.remove(first);
    assert_eq!(tabs.current().id, second);
    tabs.remove(second);
    assert_eq!(tabs.tabs.len(), 1);
    assert!(tabs.current().source.is_empty());
    assert!(tabs.current().editing);
    assert!(!tabs.current().dirty);
    assert_ne!(tabs.current().id, second);
}
#[test]
fn help_logo_is_embedded_and_does_not_need_a_document_scope() {
    let (bytes, mime, status) = assets::serve(assets::brand_url());
    assert_eq!(status, 200);
    assert_eq!(mime, "image/png");
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(root::app_name(), "BS Notepad");
}
