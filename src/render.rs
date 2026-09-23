use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::settings::Settings;

pub struct Renderer {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
        }
    }

    /// Strip a leading `---` front matter block. Rendered as markdown it comes
    /// out as a rule and a mangled heading, which is never what was meant.
    fn strip_front_matter(text: &str) -> &str {
        let Some(rest) = text.strip_prefix("---\n") else { return text };
        match rest.find("\n---\n") {
            Some(end) => &rest[end + 5..],
            None => text,
        }
    }

    pub fn to_page(&self, markdown: &str, settings: &Settings, name: &str) -> String {
        let theme = self
            .themes
            .themes
            .get(&settings.theme)
            .unwrap_or_else(|| &self.themes.themes["base16-ocean.dark"]);

        let body_text = Self::strip_front_matter(markdown);
        let parser = Parser::new_ext(body_text, Options::all());
        let mut events = Vec::new();
        let mut language: Option<String> = None;
        let mut code = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    language = Some(match kind {
                        CodeBlockKind::Fenced(name) => name.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    });
                    code.clear();
                }
                Event::Text(text) if language.is_some() => code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => {
                    let name = language.take().unwrap_or_default();
                    let syntax = self
                        .syntaxes
                        .find_syntax_by_token(&name)
                        .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
                    // Inline styles, not CSS classes: measured smaller and faster.
                    let block = highlighted_html_for_string(&code, &self.syntaxes, syntax, theme)
                        .unwrap_or_else(|_| format!("<pre>{}</pre>", html_escape(&code)));
                    events.push(Event::Html(block.into()));
                }
                other => events.push(other),
            }
        }

        let mut body = String::new();
        html::push_html(&mut body, events.into_iter());
        shell(&body, settings, name)
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn shell(body: &str, s: &Settings, name: &str) -> String {
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><style>
:root {{
  --bg:#2b303b; --fg:#c0c5ce; --rule:#4f5b66; --link:#8fa1b3;
  --panel:#232830; --bar:#1f242c; --dim:#7a8593;
}}
html,body {{ margin:0; padding:0; height:100%; background:var(--bg); color:var(--fg); }}
body {{ font:{body_size}px/{line_height} {body_font}; display:flex; flex-direction:column; }}
#bar {{
  flex:0 0 auto; display:flex; align-items:center; gap:8px; padding:6px 10px;
  background:var(--bar); border-bottom:1px solid var(--rule); font-size:13px;
}}
#bar button {{
  background:transparent; color:var(--fg); border:1px solid var(--rule);
  border-radius:4px; padding:3px 10px; font:inherit; cursor:pointer;
}}
#bar button:hover {{ background:var(--panel); }}
#name {{ color:var(--dim); margin-left:4px; }}
#row {{ flex:1 1 auto; display:flex; min-height:0; }}
#tree {{
  width:{sidebar}px; flex:0 0 auto; background:var(--panel); overflow:auto;
  border-right:1px solid var(--rule); padding:12px; box-sizing:border-box;
  display:{tree_display}; color:var(--dim); font-size:13px;
}}
#doc {{ flex:1 1 auto; overflow:auto; }}
/* The whole window width is the text column. No max-width cap. */
main {{ width:100%; max-width:none; padding:24px 32px; box-sizing:border-box; }}
a {{ color:var(--link); }}
pre {{ overflow-x:auto; padding:12px 14px; border-radius:6px; }}
code, pre {{ font:{code_size}px/1.5 {code_font}; }}
table {{ border-collapse:collapse; }}
td,th {{ border:1px solid var(--rule); padding:4px 10px; }}
img {{ max-width:100%; }}
</style></head><body>
<header id="bar">
  <button id="toggle" title="Show or hide the file tree">&#9776;</button>
  <button id="open" title="Open a file (Ctrl+O)">Open</button>
  <span id="name">{name}</span>
</header>
<div id="row">
  <nav id="tree">File tree arrives in the next stage.</nav>
  <div id="doc"><main>{body}</main></div>
</div>
<script>
document.getElementById('open').onclick = () => window.ipc.postMessage('open');
document.getElementById('toggle').onclick = () => {{
  const t = document.getElementById('tree');
  t.style.display = (t.style.display === 'none') ? 'block' : 'none';
  window.ipc.postMessage('tree:' + (t.style.display === 'none' ? '0' : '1'));
}};
document.addEventListener('keydown', e => {{
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {{
    e.preventDefault(); window.ipc.postMessage('open');
  }}
}});
</script>
</body></html>"#,
        body_size = s.body_size,
        line_height = s.line_height,
        body_font = s.body_font,
        code_size = s.code_size,
        code_font = s.code_font,
        sidebar = s.sidebar_width,
        tree_display = if s.sidebar_visible { "block" } else { "none" },
        name = html_escape(name),
        body = body,
    )
}
