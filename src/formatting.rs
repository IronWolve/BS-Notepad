//! Permit document formatting without giving embedded HTML access to app chrome.
use crate::assets;
use html5ever::tokenizer::{
    BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use std::{cell::RefCell, path::Path};

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn colour(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() > 96 {
        return None;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return (matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .then_some(value);
    }
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Some(value);
    }
    for name in ["rgb(", "rgba(", "hsl(", "hsla("] {
        if let Some(body) = value.strip_prefix(name).and_then(|s| s.strip_suffix(')')) {
            if !body.is_empty()
                && body
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b" \t.,%+-/degturn".contains(&b))
            {
                return Some(value);
            }
        }
    }
    None
}

fn style(input: &str) -> String {
    let mut out = Vec::new();
    for declaration in input.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim().to_ascii_lowercase();
        let (property, clean) = match property.as_str() {
            "color" | "background-color" => (property.as_str(), colour(&value)),
            "background" => ("background-color", colour(&value)),
            "font-weight" => (
                "font-weight",
                [
                    "normal", "bold", "bolder", "lighter", "100", "200", "300", "400", "500",
                    "600", "700", "800", "900",
                ]
                .contains(&value.as_str())
                .then_some(value),
            ),
            "font-style" => (
                "font-style",
                ["normal", "italic", "oblique"]
                    .contains(&value.as_str())
                    .then_some(value),
            ),
            "text-align" => (
                "text-align",
                ["left", "center", "right", "justify", "start", "end"]
                    .contains(&value.as_str())
                    .then_some(value),
            ),
            "vertical-align" => (
                "vertical-align",
                [
                    "baseline",
                    "sub",
                    "super",
                    "middle",
                    "top",
                    "bottom",
                    "text-top",
                    "text-bottom",
                ]
                .contains(&value.as_str())
                .then_some(value),
            ),
            "text-decoration" => {
                let valid = !value.is_empty()
                    && value.split_whitespace().all(|word| {
                        ["none", "underline", "overline", "line-through"].contains(&word)
                    });
                ("text-decoration", valid.then_some(value))
            }
            _ => continue,
        };
        if let Some(value) = clean {
            out.push(format!("{}:{}", property, value));
        }
    }
    out.join(";")
}

struct Formatter<'a> {
    output: RefCell<String>,
    base: Option<&'a Path>,
    remote: bool,
}
impl TokenSink for Formatter<'_> {
    type Handle = ();
    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        let mut out = self.output.borrow_mut();
        match token {
            Token::CharacterTokens(text) => out.push_str(&escape(&text)),
            Token::NullCharacterToken => out.push('\u{fffd}'),
            Token::TagToken(tag) => {
                let name = tag.name.as_ref();
                let allowed = matches!(
                    name,
                    "span"
                        | "div"
                        | "p"
                        | "br"
                        | "hr"
                        | "strong"
                        | "b"
                        | "em"
                        | "i"
                        | "u"
                        | "s"
                        | "del"
                        | "ins"
                        | "sub"
                        | "sup"
                        | "small"
                        | "mark"
                        | "kbd"
                        | "samp"
                        | "var"
                        | "code"
                        | "pre"
                        | "blockquote"
                        | "ul"
                        | "ol"
                        | "li"
                        | "dl"
                        | "dt"
                        | "dd"
                        | "table"
                        | "caption"
                        | "thead"
                        | "tbody"
                        | "tfoot"
                        | "tr"
                        | "th"
                        | "td"
                        | "details"
                        | "summary"
                        | "a"
                        | "img"
                        | "input"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                );
                if !allowed {
                    out.push_str(&escape(&format!(
                        "<{}{}>",
                        if tag.kind == TagKind::EndTag { "/" } else { "" },
                        name
                    )));
                    return TokenSinkResult::Continue;
                }
                let void = matches!(name, "br" | "hr" | "img" | "input");
                if tag.kind == TagKind::EndTag {
                    if !void {
                        out.push_str(&format!("</{}>", name));
                    }
                    return TokenSinkResult::Continue;
                }
                // Only read-only task boxes are accepted as form controls.
                if name == "input"
                    && !tag.attrs.iter().any(|a| {
                        a.name.local.as_ref() == "type" && a.value.eq_ignore_ascii_case("checkbox")
                    })
                {
                    return TokenSinkResult::Continue;
                }
                out.push('<');
                out.push_str(name);
                if name == "input" {
                    out.push_str(" type=\"checkbox\" disabled");
                }
                for attr in tag.attrs {
                    if !attr.name.ns.is_empty() {
                        continue;
                    }
                    let key = attr.name.local.as_ref();
                    let value = attr.value.as_ref();
                    let mut attributes: Vec<(&str, String)> = Vec::new();
                    match key {
                        "style" => {
                            let value = style(value);
                            if !value.is_empty() {
                                attributes.push(("style", value));
                            }
                        }
                        "title" => attributes.push(("title", value.into())),
                        "alt" if name == "img" => attributes.push(("alt", value.into())),
                        "href" if name == "a" => {
                            if let Some(target) = assets::external(value) {
                                attributes.push(("href", target));
                                attributes.push(("data-external", "1".into()));
                            } else if value.starts_with('#') {
                                attributes.push(("href", format!("#{}", assets::fragment(value))));
                            } else if let Some((path, fragment)) =
                                assets::local_target(value, self.base)
                            {
                                attributes.push(("href", "#".into()));
                                attributes.push(("data-open", path));
                                attributes.push(("data-fragment", fragment));
                            }
                        }
                        "src" if name == "img" => {
                            if let Some(target) = assets::image_url(value, self.base, self.remote) {
                                attributes.push(("src", target));
                            }
                        }
                        "width" | "height" if name == "img" => {
                            if let Ok(n) = value.parse::<u32>() {
                                if (1..=8192).contains(&n) {
                                    attributes.push((key, n.to_string()));
                                }
                            }
                        }
                        "colspan" | "rowspan" if matches!(name, "td" | "th") => {
                            if let Ok(n) = value.parse::<u16>() {
                                if n <= 1000 {
                                    attributes.push((key, n.to_string()));
                                }
                            }
                        }
                        "align" if ["left", "center", "right", "justify"].contains(&value) => {
                            attributes.push(("align", value.into()))
                        }
                        "open" if name == "details" => out.push_str(" open"),
                        "checked" if name == "input" => out.push_str(" checked"),
                        _ => {}
                    }
                    for (key, value) in attributes {
                        out.push_str(&format!(" {}=\"{}\"", key, escape(&value)));
                    }
                }
                out.push('>');
                if tag.self_closing && !void {
                    out.push_str(&format!("</{}>", name));
                }
            }
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

#[cfg(test)]
pub fn html(fragment: &str, base: Option<&Path>) -> String {
    html_with_options(fragment, base, true)
}

pub fn html_with_options(fragment: &str, base: Option<&Path>, remote: bool) -> String {
    let sink = Formatter {
        output: RefCell::new(String::new()),
        base,
        remote,
    };
    let input = BufferQueue::default();
    input.push_back(fragment.into());
    let tokenizer = Tokenizer::new(sink, TokenizerOpts::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.output.into_inner()
}
