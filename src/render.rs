use std::path::Path;

use crate::formatting::escape;
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use syntect::highlighting::Theme as CodeTheme;
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
    prepared: std::cell::RefCell<std::collections::HashMap<String, std::sync::Arc<CodeTheme>>>,
    cancellation: Option<(std::sync::Arc<std::sync::atomic::AtomicU64>, u64)>,
    themes: EmbeddedLazyThemeSet,
}

fn anchor_for(text: &str, used: &mut std::collections::HashMap<String, usize>) -> String {
    static CATEGORIES: std::sync::LazyLock<Vec<regex_syntax::hir::ClassUnicodeRange>> =
        std::sync::LazyLock::new(|| {
            match regex_syntax::Parser::new().parse(r"[\p{L}\p{M}\p{N}\p{Pc}]") {
                Ok(hir) => match hir.kind() {
                    regex_syntax::hir::HirKind::Class(regex_syntax::hir::Class::Unicode(class)) => {
                        class.ranges().to_vec()
                    }
                    _ => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        });
    let base: String = text
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                return Some('-');
            }
            if c == '-'
                || CATEGORIES
                    .binary_search_by(|range| {
                        if c < range.start() {
                            std::cmp::Ordering::Greater
                        } else if c > range.end() {
                            std::cmp::Ordering::Less
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    })
                    .is_ok()
            {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    let base = format!(
        "doc-heading-{}",
        if base.is_empty() { "section" } else { &base }
    );
    let mut candidate = base.clone();
    if let Some(previous) = used.get(&base).copied() {
        let mut suffix = previous + 1;
        loop {
            candidate = format!("{}-{}", base, suffix);
            if !used.contains_key(&candidate) {
                break;
            }
            suffix += 1;
        }
        used.insert(base, suffix);
    }
    used.entry(candidate.clone()).or_insert(0);
    candidate
}

fn footnote_id(
    name: &str,
    labels: &mut std::collections::HashMap<unicase::UniCase<String>, String>,
) -> String {
    let normalized = name.split_whitespace().collect::<Vec<_>>().join(" ");
    labels
        .entry(unicase::UniCase::new(normalized.clone()))
        .or_insert_with(|| format!("doc-footnote-{}", normalized.to_lowercase()))
        .clone()
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
            prepared: Default::default(),
            cancellation: None,
            themes: two_face::theme::extra(),
        }
    }

    pub fn cancel_token(
        &mut self,
        token: std::sync::Arc<std::sync::atomic::AtomicU64>,
        generation: u64,
    ) {
        self.cancellation = Some((token, generation));
    }
    fn cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(|(token, generation)| {
                token.load(std::sync::atomic::Ordering::Relaxed) != *generation
            })
    }
    fn code_theme(&self, theme: &Theme) -> std::sync::Arc<CodeTheme> {
        let key = format!(
            "{}:{}:{}:{}",
            theme.id, theme.text_contrast, theme.fg, theme.panel
        );
        if let Some(cached) = self.prepared.borrow().get(&key) {
            return cached.clone();
        }
        let base = self.themes.get(crate::theme::code_theme_name(&theme.id));
        let colour = |hex: &str| syntect::highlighting::Color {
            r: hex
                .get(1..3)
                .and_then(|c| u8::from_str_radix(c, 16).ok())
                .unwrap_or(0),
            g: hex
                .get(3..5)
                .and_then(|c| u8::from_str_radix(c, 16).ok())
                .unwrap_or(0),
            b: hex
                .get(5..7)
                .and_then(|c| u8::from_str_radix(c, 16).ok())
                .unwrap_or(0),
            a: 255,
        };
        let classic = matches!(
            theme.id.as_str(),
            "light"
                | "dark"
                | "dracula"
                | "solarized-dark"
                | "solarized-light"
                | "nord"
                | "gruvbox"
                | "monokai"
        );
        let background = if classic {
            base.settings
                .background
                .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
                .unwrap_or_else(|| theme.panel.clone())
        } else {
            theme.panel.clone()
        };
        let mut adjusted = base.clone();
        adjusted.settings.background = Some(colour(&background));
        adjusted.settings.foreground =
            Some(colour(&crate::theme::guard(&theme.fg, &background, 4.5)));
        for scope in &mut adjusted.scopes {
            if let Some(fg) = scope.style.foreground {
                let hex = format!("#{:02x}{:02x}{:02x}", fg.r, fg.g, fg.b);
                scope.style.foreground = Some(colour(&crate::theme::strengthen(
                    &crate::theme::guard(&hex, &background, 4.5),
                    &background,
                    theme.text_contrast,
                )));
            }
            if scope.style.background.is_some() {
                scope.style.background = Some(colour(&background));
            }
        }
        let adjusted = std::sync::Arc::new(adjusted);
        let mut cache = self.prepared.borrow_mut();
        if cache.len() > 48 {
            cache.clear();
        }
        cache.insert(key, adjusted.clone());
        adjusted
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
        if !colour || code.lines().any(|line| line.len() > 16 * 1024) {
            return format!("<pre class=\"plain\"><code>{}</code></pre>", escape(code));
        }
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
        let (mut output, background) = syntect::html::start_highlighted_html_snippet(theme);
        for line in syntect::util::LinesWithEndings::from(code) {
            if self.cancelled() {
                return String::new();
            }
            let rendered = highlighter
                .highlight_line(line, &self.syntaxes)
                .ok()
                .and_then(|regions| {
                    syntect::html::styled_line_to_highlighted_html(
                        &regions,
                        syntect::html::IncludeBackground::IfDifferent(background),
                    )
                    .ok()
                });
            let Some(rendered) = rendered else {
                return format!("<pre><code>{}</code></pre>", escape(code));
            };
            output.push_str(&rendered);
        }
        output.push_str("</pre>\n");
        output
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
        let first = text.lines().next().unwrap_or("");
        let first = first.strip_prefix('\u{feff}').unwrap_or(first).trim_end();
        if !["---", "+++"].contains(&first) {
            return (String::new(), text);
        }
        let start = text.find('\n').map(|i| i + 1).unwrap_or(text.len());
        if text[start..].lines().next().unwrap_or("").trim().is_empty() {
            return (String::new(), text);
        }
        let mut offset = start;
        for line in text[start..].split_inclusive('\n') {
            let delimiter = line.trim_end();
            if delimiter == first || first == "---" && delimiter == "..." {
                return (
                    text[start..offset].trim_end_matches(['\r', '\n']).into(),
                    &text[offset + line.len()..],
                );
            }
            offset += line.len();
        }
        (String::new(), text)
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
                html: self.colour_with(
                    text,
                    syntax,
                    &code_theme,
                    settings.syntax_colour
                        && text.len() <= settings.highlight_limit_kb as usize * 1024,
                ),
                outline: Vec::new(),
                note: if settings.syntax_colour
                    && text.len() > settings.highlight_limit_kb as usize * 1024
                {
                    "text view - highlight limit reached".into()
                } else if settings.syntax_colour {
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
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_GFM
            | Options::ENABLE_DEFINITION_LIST;
        let mut in_code = false;
        let code_bytes: usize = Parser::new_ext(body_text, options)
            .filter_map(|event| match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    in_code = true;
                    None
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code = false;
                    None
                }
                Event::Text(text) if in_code => Some(text.len()),
                _ => None,
            })
            .sum();
        let colour =
            settings.syntax_colour && code_bytes <= settings.highlight_limit_kb as usize * 1024;
        let parser = Parser::new_ext(body_text, options);
        let mut events: Vec<Event> = Vec::new();
        let mut outline: Vec<Heading> = Vec::new();
        let mut used_anchors = std::collections::HashMap::new();
        let mut footnotes = std::collections::HashMap::new();
        let mut html_block: Option<String> = None;
        let mut nested_images = 0usize;
        let sanitizer = crate::formatting::Sanitizer::new(base, settings.remote_images);

        let mut code = String::new();
        let mut language: Option<String> = None;
        let mut heading: Option<(u8, String, usize)> = None;
        let mut image: Option<(String, String, String)> = None; // src, title, alt

        for event in parser {
            if self.cancelled() {
                return Document {
                    html: String::new(),
                    outline: Vec::new(),
                    note: String::new(),
                    front_matter: String::new(),
                };
            }
            if sanitizer.hidden()
                && !matches!(
                    event,
                    Event::Html(_)
                        | Event::InlineHtml(_)
                        | Event::Start(Tag::HtmlBlock)
                        | Event::End(TagEnd::HtmlBlock)
                )
            {
                continue;
            }
            match event {
                Event::Start(Tag::HtmlBlock) => {
                    html_block = Some(String::new());
                }
                Event::End(TagEnd::HtmlBlock) => {
                    if let Some(raw) = html_block.take() {
                        events.push(Event::Html(sanitizer.push(&raw).into()));
                    }
                }
                Event::Html(raw) if html_block.is_some() => {
                    html_block.as_mut().unwrap().push_str(&raw);
                }
                Event::Start(Tag::BlockQuote(Some(kind))) => {
                    let label = match kind {
                        pulldown_cmark::BlockQuoteKind::Note => "Note",
                        pulldown_cmark::BlockQuoteKind::Tip => "Tip",
                        pulldown_cmark::BlockQuoteKind::Important => "Important",
                        pulldown_cmark::BlockQuoteKind::Warning => "Warning",
                        pulldown_cmark::BlockQuoteKind::Caution => "Caution",
                    };
                    events.push(Event::Html(
                        format!("<blockquote><p class=\"callout-label\">{label}</p>").into(),
                    ));
                }
                Event::Start(Tag::FootnoteDefinition(name)) => events.push(Event::Start(
                    Tag::FootnoteDefinition(footnote_id(&name, &mut footnotes).into()),
                )),
                Event::FootnoteReference(name) => {
                    if let Some((_, _, alt)) = image.as_mut() {
                        alt.push_str(&name);
                    } else {
                        events.push(Event::FootnoteReference(
                            footnote_id(&name, &mut footnotes).into(),
                        ));
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    language = Some(match kind {
                        CodeBlockKind::Fenced(name) => name
                            .split_ascii_whitespace()
                            .next()
                            .unwrap_or("")
                            .split(',')
                            .next()
                            .unwrap_or("")
                            .trim_matches(['{', '}'])
                            .trim_start_matches('.')
                            .to_string(),
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
                    if image.is_some() {
                        nested_images += 1;
                    } else {
                        image = Some((dest_url.to_string(), title.to_string(), String::new()));
                    }
                }
                Event::End(TagEnd::Image) => {
                    if nested_images > 0 {
                        nested_images -= 1;
                        continue;
                    }
                    if let Some((src, title, alt)) = image.take() {
                        // Relative paths are resolved here and served through
                        // the app's own scoped protocol; an HTML string has no
                        // base location of its own.
                        let Some(resolved) = assets::image_url(&src, base, settings.remote_images)
                        else {
                            events.push(Event::Text(alt.into()));
                            continue;
                        };
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
                        events.push(Event::Html(sanitizer.push(&raw).into()));
                    }
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    if image.is_none() {
                        let url = if link_type == pulldown_cmark::LinkType::Email {
                            format!("mailto:{}", dest_url)
                        } else {
                            dest_url.to_string()
                        };
                        events.push(Event::Html(assets::link_html(&url, &title, base).into()));
                    }
                }
                Event::End(TagEnd::Link) => {
                    if image.is_none() {
                        events.push(Event::Html("</a>".into()));
                    }
                }

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

        events.push(Event::Html(sanitizer.finish().into()));
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
