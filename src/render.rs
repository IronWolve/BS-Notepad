use std::path::Path;

use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use syntect::highlighting::Theme as CodeTheme;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use two_face::theme::EmbeddedLazyThemeSet;

use crate::assets;
use crate::settings::Settings;
use crate::theme::Theme;

#[derive(Serialize, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub anchor: String,
}

#[derive(Serialize, Clone)]
pub struct Document {
    pub html: String,
    pub outline: Vec<Heading>,
    /// Shown in the bar when the document was not rendered in full.
    pub note: String,
    pub front_matter: String,
}

pub struct Renderer {
    syntaxes: SyntaxSet,
    themes: EmbeddedLazyThemeSet,
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn anchor_for(text: &str, used: &mut Vec<String>) -> String {
    let base: String = text
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-').to_string();
    let base = if base.is_empty() {
        "section".into()
    } else {
        base
    };
    let mut candidate = base.clone();
    let mut n = 2;
    while used.contains(&candidate) {
        candidate = format!("{}-{}", base, n);
        n += 1;
    }
    used.push(candidate.clone());
    candidate
}

fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("md") | Some("markdown") | Some("mdown") | Some("mkd") | Some("mkdn") | Some("mdx")
    )
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            // The extended set: 200-odd languages, so PowerShell, Dockerfile,
            // batch files and the rest colour like everything else.
            syntaxes: two_face::syntax::extra_newlines(),
            themes: two_face::theme::extra(),
        }
    }

    fn code_theme(&self, theme: &Theme) -> std::borrow::Cow<'_, CodeTheme> {
        let base = self.themes.get(crate::theme::code_theme_name(&theme.id));
        if theme.text_contrast == 0 && matches!(theme.id.as_str(),
            "light" | "dark" | "dracula" | "solarized-dark" | "solarized-light" | "nord" | "gruvbox" | "monokai") {
            return std::borrow::Cow::Borrowed(base);
        }
        let colour = |hex: &str| syntect::highlighting::Color {
            r: u8::from_str_radix(&hex[1..3], 16).unwrap_or(0),
            g: u8::from_str_radix(&hex[3..5], 16).unwrap_or(0),
            b: u8::from_str_radix(&hex[5..7], 16).unwrap_or(0),
            a: 255,
        };
        let mut adjusted = base.clone();
        adjusted.settings.background = Some(colour(&theme.panel));
        adjusted.settings.foreground =
            Some(colour(&crate::theme::guard(&theme.fg, &theme.panel, 4.5)));
        for scope in &mut adjusted.scopes {
            if let Some(fg) = scope.style.foreground {
                let hex = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
                scope.style.foreground =
                    Some(colour(&crate::theme::strengthen(
                        &crate::theme::guard(&hex, &theme.panel, 4.5), &theme.panel, theme.text_contrast)));
            }
            if scope.style.background.is_some() {
                scope.style.background = Some(colour(&theme.panel));
            }
        }
        std::borrow::Cow::Owned(adjusted)
    }

    /// Language for a file the tree opened directly, falling back to the file
    /// name itself so things like Dockerfile and Makefile still colour.
    fn syntax_for(&self, path: &Path) -> &syntect::parsing::SyntaxReference {
        let by_extension = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(|e| self.syntaxes.find_syntax_by_token(e));
        let by_name = || {
            path.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| self.syntaxes.find_syntax_by_token(n))
        };
        by_extension
            .or_else(by_name)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text())
    }

    fn colour_with(
        &self,
        code: &str,
        syntax: &syntect::parsing::SyntaxReference,
        theme: &CodeTheme,
        colour: bool,
    ) -> String {
        if !colour {
            return format!("<pre class=\"plain\"><code>{}</code></pre>", escape(code));
        }
        // Inline styles rather than CSS classes: measured smaller and faster.
        highlighted_html_for_string(code, &self.syntaxes, syntax, theme)
            .unwrap_or_else(|_| format!("<pre>{}</pre>", escape(code)))
    }

    fn highlight(&self, code: &str, language: &str, theme: &CodeTheme, colour: bool) -> String {
        let syntax = self
            .syntaxes
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        self.colour_with(code, syntax, theme, colour)
    }

    /// A leading `---` block is metadata, not content. Rendered as markdown it
    /// turns into a rule and a mangled heading.
    fn split_front_matter(text: &str) -> (String, &str) {
        let Some(rest) = text.strip_prefix("---\n") else {
            return (String::new(), text);
        };
        match rest.find("\n---\n") {
            Some(end) => (rest[..end].to_string(), &rest[end + 5..]),
            None => (String::new(), text),
        }
    }

    pub fn render(
        &self,
        path: Option<&Path>,
        text: &str,
        settings: &Settings,
        theme: &Theme,
    ) -> Document {
        let code_theme = self.code_theme(theme);
        let base = path.and_then(|p| p.parent());

        // Very large files are opened as plain text on purpose, rather than
        // parsed and coloured until the window stops responding.
        let plain_limit = settings.plain_text_above_mb as usize * 1024 * 1024;
        if text.len() > plain_limit {
            return Document {
                html: format!("<pre class=\"plain\"><code>{}</code></pre>", escape(text)),
                outline: Vec::new(),
                note: format!(
                    "opened as plain text - over {} MB",
                    settings.plain_text_above_mb
                ),
                front_matter: String::new(),
            };
        }

        // Anything that is not markdown, and markdown itself when the source
        // view is asked for, is shown as its own text - coloured in whatever
        // language it is.
        let as_source =
            settings.view_mode == "source" || path.map(|p| !is_markdown(p)).unwrap_or(false);
        if as_source {
            let syntax = match path {
                Some(p) => self.syntax_for(p),
                None => self.syntaxes.find_syntax_plain_text(),
            };
            let name = syntax.name.clone();
            return Document {
                html: self.colour_with(text, syntax, &code_theme, settings.syntax_colour),
                outline: Vec::new(),
                note: if settings.syntax_colour {
                    format!("text view - {}", name)
                } else {
                    "text view - colouring off".into()
                },
                front_matter: String::new(),
            };
        }

        let (front_matter, body_text) = Self::split_front_matter(text);

        // Colouring every block up front is what makes a huge document slow,
        // so past a threshold the code is shown plain and says so.
        let code_bytes: usize = body_text
            .split("\n```")
            .skip(1)
            .step_by(2)
            .map(|chunk| chunk.len())
            .sum();
        let colour =
            settings.syntax_colour && code_bytes <= settings.highlight_limit_kb as usize * 1024;

        let parser = Parser::new_ext(body_text, Options::all());
        let mut events: Vec<Event> = Vec::new();
        let mut outline: Vec<Heading> = Vec::new();
        let mut used_anchors: Vec<String> = Vec::new();

        let mut code = String::new();
        let mut language: Option<String> = None;
        let mut heading: Option<(u8, String, usize)> = None;
        let mut image: Option<(String, String, String)> = None; // src, title, alt

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    language = Some(match kind {
                        CodeBlockKind::Fenced(name) => name.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    });
                    code.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    let name = language.take().unwrap_or_default();
                    let block = self.highlight(&code, &name, &code_theme, colour);
                    events.push(Event::Html(block.into()));
                }

                Event::Start(Tag::Heading { level, .. }) => {
                    heading = Some((level_number(level), String::new(), events.len()));
                    events.push(Event::Html(String::new().into()));
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some((level, text, start)) = heading.take() {
                        let anchor = anchor_for(&text, &mut used_anchors);
                        outline.push(Heading {
                            level,
                            text: text.clone(),
                            anchor: anchor.clone(),
                        });
                        events[start] =
                            Event::Html(format!("<h{} id=\"{}\">", level, anchor).into());
                        events.push(Event::Html(format!("</h{}>", level).into()));
                    }
                }

                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    image = Some((dest_url.to_string(), title.to_string(), String::new()));
                }
                Event::End(TagEnd::Image) => {
                    if let Some((src, title, alt)) = image.take() {
                        // Relative paths are resolved here and served through
                        // the app's own scoped protocol; an HTML string has no
                        // base location of its own.
                        let resolved = assets::url_for(&src, base);
                        events.push(Event::Html(
                            format!(
                                "<img src=\"{}\" alt=\"{}\" title=\"{}\">",
                                escape(&resolved),
                                escape(&alt),
                                escape(&title)
                            )
                            .into(),
                        ));
                    }
                }

                Event::Html(raw) | Event::InlineHtml(raw) => {
                    if image.is_none() {
                        events.push(Event::Html(crate::formatting::html(&raw, base).into()));
                    }
                }
                Event::Start(Tag::Link {
                    dest_url, title, ..
                }) => {
                    let target = dest_url.to_string();
                    let tag = if target.starts_with("http://") || target.starts_with("https://") {
                        // Opened in the system browser, never in this window.
                        format!(
                            "<a href=\"{}\" data-external=\"1\" title=\"{}\">",
                            escape(&target),
                            escape(&title)
                        )
                    } else if target.starts_with('#') {
                        format!("<a href=\"{}\">", escape(&target))
                    } else {
                        // Another file beside this one: opened in the app.
                        let absolute = assets::resolve(&target, base);
                        format!(
                            "<a href=\"#\" data-open=\"{}\" title=\"{}\">",
                            escape(&absolute),
                            escape(&title)
                        )
                    };
                    events.push(Event::Html(tag.into()));
                }
                Event::End(TagEnd::Link) => events.push(Event::Html("</a>".into())),

                Event::Text(text) => {
                    if language.is_some() {
                        code.push_str(&text);
                    } else {
                        if let Some((_, buffer, _)) = heading.as_mut() {
                            buffer.push_str(&text);
                        }
                        if let Some((_, _, alt)) = image.as_mut() {
                            alt.push_str(&text);
                        } else {
                            events.push(Event::Text(text));
                        }
                    }
                }
                Event::Code(text) => {
                    if let Some((_, buffer, _)) = heading.as_mut() {
                        buffer.push_str(&text);
                    }
                    if let Some((_, _, alt)) = image.as_mut() {
                        alt.push_str(&text);
                    } else {
                        events.push(Event::Code(text));
                    }
                }
                other => {
                    if matches!(other, Event::SoftBreak | Event::HardBreak) {
                        if let Some((_, buffer, _)) = heading.as_mut() {
                            buffer.push(' ');
                        }
                    }
                    if image.is_none() {
                        events.push(other);
                    }
                }
            }
        }

        let mut body = String::new();
        html::push_html(&mut body, events.into_iter());

        let note = if colour || !settings.syntax_colour {
            String::new()
        } else {
            format!(
                "code shown plain - over {} KB of it in one file",
                settings.highlight_limit_kb
            )
        };

        Document {
            html: body,
            outline,
            note,
            front_matter,
        }
    }
}
