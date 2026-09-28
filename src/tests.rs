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
    let s = Settings {
        chrome: "auto".into(),
        sidebar: "off".into(),
        sidebar_width: 0,
        tab_size: 100,
        zoom: -1.,
        ..Default::default()
    };
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

#[test]
fn markdown_preserves_report_colors_and_formatted_headings() {
    let renderer = Renderer::new();
    let source = r##"# Ranked Local LLM Results

Campaign: 189 fixed single-request tests across 63 profiles.

<span style="color:#22c55e">🟩</span> all three prompts completed · <span style="color:#eab308">🟨</span> partial · <span style="color:#ef4444">🟥</span> launch/health failure · 🔍 manual response review required

> Quality is not honestly reducible to tok/s. This report uses response capture and test completion as an automated signal, then preserves raw outputs for review.

## A **bold** `code` [link](https://example.com) and <span style="color:#22c55e">green</span> heading

<div style="color: rgb(20, 90, 120); font-weight:600"><em>Embedded formatting</em><br>Second line</div>

Literal code: `<span style="color:red">example</span>`.
"##;
    let doc = renderer.render(
        Some(Path::new("report.md")),
        source,
        &Settings::default(),
        &theme::find("light"),
    );
    for (color, marker) in [("#22c55e", "🟩"), ("#eab308", "🟨"), ("#ef4444", "🟥")] {
        assert!(
            doc.html.contains(&format!(
                "<span style=\"color:{}\">{}</span>",
                color, marker
            )),
            "{} marker",
            marker
        );
    }
    assert!(doc.html.contains("<strong>bold</strong> <code>code</code>"));
    assert!(doc.html.contains("data-external=\"1\""));
    assert!(doc
        .html
        .contains("<span style=\"color:#22c55e\">green</span> heading</h2>"));
    assert_eq!(doc.outline[1].text, "A bold code link and green heading");
    assert!(doc
        .html
        .contains("<em>Embedded formatting</em><br>Second line"));
    assert!(doc.html.contains("<code>&lt;span style="));
    if let Some(path) = std::env::var_os("RENDER_PREVIEW") {
        let path = PathBuf::from(path);
        std::fs::write(&path, &doc.html).unwrap();
        std::fs::write(path.with_extension("md"), source).unwrap();
    }
}

#[test]
fn embedded_html_keeps_formatting_but_drops_app_hooks_and_layout_overrides() {
    let html = formatting::html(
        r##"<span id="text" class="on" onclick="window.ipc.postMessage('quit')" style="position:fixed;inset:0;z-index:999;color:&#35;22c55e;background:url(https://invalid.example);font-weight:bold">Green</span><input type="checkbox" checked onclick="evil()"><input type="text" autofocus><svg onload="evil()"><a href="java&#10;script:evil()">bad</a></svg>"##,
        None,
    );
    assert!(html.contains("<span style=\"color:#22c55e;font-weight:bold\">Green</span>"));
    assert!(html.contains("<input type=\"checkbox\" disabled checked>"));
    for forbidden in [
        "onclick",
        "onload",
        "autofocus",
        "id=",
        "class=",
        "position:",
        "z-index",
        "url(",
        "type=\"text\"",
        "href=\"javascript",
        "<svg",
    ] {
        assert!(!html.contains(forbidden), "unexpected {}", forbidden);
    }
}

#[test]
fn embedded_html_links_and_images_use_existing_document_routes() {
    let html = formatting::html(
        r#"<a href="HTTPS://example.com/read?a=1&amp;b=2" target="_self">Web</a><a href="next.md">Local</a><img src="images/picture.png" alt="A &amp; B" width="80" onerror="evil()">"#,
        Some(Path::new("notes")),
    );
    assert!(html.contains("href=\"https://example.com/read?a=1&amp;b=2\" data-external=\"1\""));
    assert!(html.contains(&format!(
        "data-open=\"{}\"",
        Path::new("notes").join("next.md").display()
    )));
    assert!(
        html.contains("src=\"asset://localhost/notes/images/picture.png\"")
            || html.contains("src=\"http://asset.localhost/notes/images/picture.png\"")
    );
    assert!(html.contains("alt=\"A &amp; B\" width=\"80\""));
    assert!(!html.contains("onerror"));
}

#[test]
fn inline_formatting_inside_headings_keeps_links_and_alt_text_in_the_heading() {
    let renderer = Renderer::new();
    let doc = renderer.render(Some(Path::new("headings.md")), "## Read *this* [guide](next.md) ![badge](badge.png)\n\n## Read *this* [guide](next.md) ![badge](badge.png)", &Settings::default(), &theme::find("dark"));
    assert_eq!(doc.outline[0].text, "Read this guide badge");
    assert_ne!(doc.outline[0].anchor, doc.outline[1].anchor);
    assert!(doc.html.contains("Read <em>this</em> <a"));
    assert!(doc.html.contains("alt=\"badge\""));
    assert_eq!(doc.html.matches("</h2>").count(), 2);
}

#[test]
fn unknown_saved_theme_normalizes_to_the_rendered_fallback() {
    let f = Fixture::new();
    std::fs::write(f.0.join("settings.json"), r#"{"theme":"removed-theme"}"#).unwrap();
    let settings = Settings::load(&f.0);
    assert_eq!(settings.theme, "dark");
}

#[test]
fn audit_headings_links_limits_and_front_matter() {
    let renderer = Renderer::new();
    let settings = Settings {
        highlight_limit_kb: 1,
        ..Settings::default()
    };
    let theme = theme::find("light");
    let doc=renderer.render(Some(Path::new("note.md")),"# Text\n# Editor\n# Panel\n# Minimap\n[mail](mailto:a@example.invalid) [section](next.md#part) ![picture](my%20picture.png)",&settings,&theme);
    for id in ["text", "editor", "panel", "minimap"] {
        assert!(doc.html.contains(&format!("id=\"doc-heading-{}\"", id)));
        assert!(!doc.html.contains(&format!("id=\"{}\"", id)));
    }
    assert!(doc.html.contains("data-external=\"1\""));
    assert!(doc.html.contains("data-fragment=\"doc-heading-part\""));
    assert!(doc.html.contains("my%20picture.png"));
    assert!(!doc.html.contains("my%2520"));
    let code = "let value=42;\n".repeat(200);
    for (path, text) in [
        ("source.rs", code.clone()),
        ("note.md", format!("~~~rust\n{code}~~~\n")),
        ("note.md", format!("```rust\n{code}```\n")),
        ("note.md", format!("    {}", code.replace('\n', "\n    "))),
    ] {
        assert!(!renderer
            .render(Some(Path::new(path)), &text, &settings, &theme)
            .html
            .contains("<span style="));
    }
    for text in [
        "---\r\ntitle: Example\r\n---\r\nBody",
        "---\ntitle: Example\n---",
    ] {
        assert_eq!(
            renderer.render(None, text, &settings, &theme).front_matter,
            "title: Example"
        );
    }
}
#[test]
fn audit_storage_formats_limits_and_unicode_patches() {
    let f = Fixture::new();
    let path = f.0.join("test.txt");
    for encoding in ["UTF-8", "UTF-8 BOM", "UTF-16 LE", "UTF-16 BE"] {
        for ending in ["LF", "CRLF", "CR"] {
            let format = storage::TextFormat {
                encoding: encoding.into(),
                ending: ending.into(),
            };
            let bytes = format.encode("Hello 😀\nWorld\n");
            std::fs::write(&path, &bytes).unwrap();
            let loaded = storage::read(&path, 1).unwrap();
            assert_eq!(loaded.source, "Hello 😀\nWorld\n");
            assert_eq!(loaded.format, format);
            assert_eq!(loaded.format.encode(&loaded.source), bytes);
        }
    }
    std::fs::write(&path, vec![b'a'; 2 * 1024 * 1024]).unwrap();
    let large = storage::read(&path, 1).unwrap();
    assert!(large.read_only);
    assert_eq!(large.source.len(), storage::PREVIEW_BYTES);
    let mut text = "A😀B".to_string();
    assert!(!storage::apply_patch(&mut text, 2, 3, "x"));
    assert!(storage::apply_patch(&mut text, 1, 3, "😎"));
    assert_eq!(text, "A😎B");
}
#[test]
fn audit_settings_backup_and_recovery() {
    let f = Fixture::new();
    let mut s = Settings {
        theme: "sage".into(),
        ..Settings::default()
    };
    s.save(&f.0).unwrap();
    s.theme = "light".into();
    s.save(&f.0).unwrap();
    std::fs::write(f.0.join("settings.json"), "broken").unwrap();
    let recovered = Settings::load(&f.0);
    assert_eq!(recovered.theme, "sage");
    assert!(!recovered.load_warning.is_empty());
    assert!(std::fs::read_dir(&f.0).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("settings.corrupt-")));
    let key = recovery::key();
    let draft = recovery::Draft {
        path: None,
        source: "Unsaved 😀".into(),
        saved_source: String::new(),
        format: Default::default(),
        fingerprint: None,
    };
    recovery::write(&f.0, &key, Some(&draft)).unwrap();
    assert_eq!(recovery::read(&f.0)[0].1.source, draft.source);
    recovery::write(&f.0, &key, None).unwrap();
    assert!(recovery::read(&f.0).is_empty());
    assert!(recovery::write(&f.0, "../bad", None).is_err());
    let mut bounds = Settings {
        window_width: u32::MAX,
        window_height: 0,
        ..Settings::default()
    };
    bounds.normalize();
    assert_eq!((bounds.window_width, bounds.window_height), (7680, 400));
}
#[test]
fn audit_instance_lock_and_acknowledgment() {
    let f = Fixture::new();
    let guard = instance::acquire(&f.0).unwrap();
    assert!(instance::acquire(&f.0).is_err());
    let (tx, rx) = std::sync::mpsc::channel();
    instance::listen(&f.0, move |paths| {
        let _ = tx.send(paths);
    })
    .unwrap();
    assert!(instance::hand_off(&f.0, &[PathBuf::from("note.md")]));
    assert_eq!(
        rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap(),
        ["note.md"]
    );
    let endpoint: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.0.join("instance.port")).unwrap()).unwrap();
    let held = std::net::TcpStream::connect(format!("127.0.0.1:{}", endpoint["port"])).unwrap();
    assert!(instance::hand_off(&f.0, &[]));
    drop(held);
    drop(guard);
    assert!(instance::acquire(&f.0).is_ok());
}
#[test]
fn export_browser_fixture() {
    let source = "# Text\n\nHello **beautiful** world.\n\n```text\npayload\n```\n";
    let settings = Settings::default();
    let rendered = Renderer::new().render(
        Some(Path::new("audit.md")),
        source,
        &settings,
        &theme::find(&settings.theme),
    );
    let mut bootstrap = include_str!("../tests/ui-fixture.js").to_string();
    for (key, value) in [
        ("FIXTURE_SOURCE", json!(source)),
        ("FIXTURE_RENDER", json!(rendered.html)),
        ("FIXTURE_SETTINGS", json!(settings)),
        ("FIXTURE_THEMES", json!(theme::builtin())),
        ("FIXTURE_NAME", json!(root::app_name())),
        ("FIXTURE_VERSION", json!(env!("CARGO_PKG_VERSION"))),
    ] {
        bootstrap = bootstrap.replace(key, &value.to_string().replace("</", "<\\/"));
    }
    let shell = ui::shell().replace(
        "const shellNodes =",
        &format!("{}\nconst shellNodes =", bootstrap),
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tmp")
        .join("browser-regressions");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("fixture.html"), shell).unwrap();
}

#[test]
fn footnotes_cannot_claim_application_ids_and_remote_images_can_be_disabled() {
    let renderer = Renderer::new();
    let settings = Settings {
        remote_images: false,
        ..Settings::default()
    };
    let theme = theme::find("dark");
    let doc=renderer.render(None,"Note[^text].\n\n[^text]: Body.\n\n![remote](https://example.invalid/image.png)\n\n<img src=\"https://example.invalid/other.png\">",&settings,&theme);
    assert!(doc.html.contains("id=\"doc-footnote-text\""));
    assert!(!doc.html.contains("id=\"text\""));
    assert!(!doc.html.contains("src=\"https://"));
}
#[test]
fn cleanup_preserves_unmarked_and_unknown_files() {
    let f = Fixture::new();
    for index in 1..=3 {
        let dir = f.0.join(format!("previous-20260925-{}", index));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join(".release-backup"), env!("CARGO_PKG_NAME")).unwrap();
        std::fs::write(dir.join("installed.json"), "{}").unwrap();
    }
    let unknown = f.0.join("previous-20260925-0");
    std::fs::create_dir(&unknown).unwrap();
    std::fs::write(unknown.join(".release-backup"), env!("CARGO_PKG_NAME")).unwrap();
    std::fs::write(unknown.join("my-note.md"), "keep").unwrap();
    let unmarked = f.0.join("previous-20260924-0");
    std::fs::create_dir(&unmarked).unwrap();
    std::fs::write(unmarked.join("installed.json"), "keep").unwrap();
    maintenance::cleanup(&f.0, 30, 1).unwrap();
    assert!(unknown.join("my-note.md").exists());
    assert!(unmarked.exists());
    assert!(f.0.join("previous-20260925-3").exists());
    assert!(!f.0.join("previous-20260925-1").exists());
}

#[test]
fn settings_failure_preserves_the_last_good_file() {
    let f = Fixture::new();
    let settings = Settings::default();
    settings.save(&f.0).unwrap();
    let original = std::fs::read(f.0.join("settings.json")).unwrap();
    std::fs::create_dir(f.0.join("settings.json.bak")).unwrap();
    assert!(settings.save(&f.0).is_err());
    assert_eq!(std::fs::read(f.0.join("settings.json")).unwrap(), original);
}
#[test]
fn cancelled_render_does_not_return_stale_document_markup() {
    let mut renderer = Renderer::new();
    let token = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(2));
    renderer.cancel_token(token, 1);
    let rendered = renderer.render(
        None,
        "# Old result\n\nDiscard this.",
        &Settings::default(),
        &theme::find("dark"),
    );
    assert!(rendered.html.is_empty());
}

#[test]
fn trusted_startup_navigation_is_not_blocked_before_the_handshake() {
    assert!(navigation_allowed(false, ""));
    assert!(navigation_allowed(false, "about:blank"));
    assert!(navigation_allowed(true, "about:blank#doc-heading-text"));
    assert!(!navigation_allowed(true, "https://example.invalid/"));
    assert!(!navigation_allowed(true, "file:///unexpected.html"));
}

#[test]
fn image_files_open_without_text_decoding_or_editable_content() {
    let f = Fixture::new();
    let path = f.0.join("picture.PNG");
    let bytes = include_bytes!("../assets/brand.png");
    std::fs::write(&path, bytes).unwrap();
    let loaded = storage::read(&path, 1).unwrap();
    assert_eq!(loaded.image.as_ref().unwrap().format, "PNG");
    assert_eq!(loaded.image.as_ref().unwrap().bytes, bytes.len() as u64);
    assert!(loaded.source.is_empty() && loaded.read_only);
    let doc = Document::loaded(path.clone(), loaded);
    assert!(
        doc.image.is_some()
            && doc.read_only
            && !doc.dirty
            && !doc.editing
            && doc.fingerprint.is_none()
    );
    assert!(tree::list(&f.0, false)
        .unwrap()
        .iter()
        .any(|e| e.name == "picture.PNG" && e.openable));
    assert_eq!(std::fs::read(path).unwrap(), bytes);
    for ext in assets::IMAGE_EXTENSIONS {
        assert!(assets::image_type(&PathBuf::from(format!("test.{}", ext))).is_some());
    }
    assert!(assets::image_type(Path::new("notes.md")).is_none());
}
#[test]
fn oversized_images_are_rejected_without_loading_their_bytes() {
    let f = Fixture::new();
    let path = f.0.join("huge.jpg");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(assets::MAX_IMAGE_BYTES + 1)
        .unwrap();
    assert!(storage::read(&path, 32)
        .unwrap_err()
        .to_string()
        .contains("32 MB"));
    let svg = f.0.join("vector.svg");
    std::fs::write(
        &svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"80\" height=\"40\"></svg>",
    )
    .unwrap();
    let image = storage::read(&svg, 1).unwrap();
    assert_eq!(image.image.unwrap().format, "SVG");
    assert!(image.read_only && image.source.is_empty());
}
