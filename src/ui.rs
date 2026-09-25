/// The page shell. Loaded once; everything after that arrives over IPC, so the
/// document can change without rebuilding the window.
pub const SHELL: &str = r##"<!doctype html><html lang="en"><head><meta name="viewport" content="width=device-width, initial-scale=1"><title>Notes</title><meta charset="utf-8"><style>
:root {
  --bg:#2b303b; --fg:#c0c5ce; --panel:#232830; --bar:#1f242c;
  --rule:#4f5b66; --link:#8fa1b3; --dim:#7a8593; --accent:#8fa1b3;
  --ui-font:system-ui,sans-serif; --body-font:system-ui,sans-serif;
  --code-font:ui-monospace,monospace;
  --ui-size:13px; --body-size:16px; --code-size:14px; --line:1.65;
  --side-w:260px; --zoom:1;
}
* { box-sizing:border-box; }
html,body { margin:0; height:100%; background:var(--bg); color:var(--fg); }
body { display:flex; flex-direction:column; font:var(--ui-size)/1.4 var(--ui-font); }

/* The bar stays out of the way until the mouse reaches the top edge. */
#hot { position:absolute; top:0; left:0; right:0; height:10px; z-index:5; }
#bar { display:flex; align-items:center; gap:6px; padding:5px 8px;
       background:var(--bar); border-bottom:1px solid var(--rule); }
body.auto #bar { position:absolute; top:0; left:0; right:0; z-index:6;
       transform:translateY(-100%); transition:transform .14s ease; }
body.auto #bar.show, body.auto #bar.pinned { transform:none; }
#peek { position:absolute; top:0; left:50%; transform:translateX(-50%); z-index:4;
       background:var(--bar); color:var(--dim); border:1px solid var(--rule);
       border-top:0; border-radius:0 0 6px 6px; padding:0 12px; font-size:11px;
       display:none; }
body.auto #peek { display:block; }
body.auto #bar.show ~ #peek, body.auto #bar.pinned ~ #peek { opacity:0; }
#bar button, #bar select {
  background:transparent; color:var(--fg); border:1px solid var(--rule);
  border-radius:4px; padding:3px 9px; font:inherit; cursor:pointer;
}
#bar button:hover, #bar select:hover { background:var(--panel); }
#bar button.on { background:var(--panel); border-color:var(--accent); }
#name { color:var(--dim); margin-left:6px; overflow:hidden; text-overflow:ellipsis;
        white-space:nowrap; flex:1 1 auto; }
#note { color:var(--accent); margin-right:8px; }
.sp { flex:0 0 auto; width:1px; height:20px; background:var(--rule); margin:0 3px; }

#find { display:none; align-items:center; gap:6px; padding:5px 8px;
        background:var(--panel); border-bottom:1px solid var(--rule); }
#find.show { display:flex; }
#find input { background:var(--bg); color:var(--fg); border:1px solid var(--rule);
              border-radius:4px; padding:3px 8px; font:inherit; width:260px; }
#find button { background:transparent; color:var(--fg); border:1px solid var(--rule);
               border-radius:4px; padding:2px 8px; cursor:pointer; font:inherit; }
#hits { color:var(--dim); }

#row { flex:1 1 auto; display:flex; min-height:0; position:relative; }
#side { position:relative; width:var(--side-w); flex:0 0 auto; background:var(--panel); display:flex;
        flex-direction:column; border-right:1px solid var(--rule); min-width:0; }
#side.hidden { display:none; }
/* Same idea as the bar: out of the way until the mouse reaches the edge. */
#sidehot { position:absolute; top:0; bottom:0; left:0; width:10px; z-index:5; }
body.autoside #side { position:absolute; top:0; bottom:0; left:0; z-index:6;
        transform:translateX(-100%); transition:transform .14s ease; }
body.autoside #side.show, body.autoside #side.pinned { transform:none; }
body.autoside #grip { display:none; }
body.autoside #side.show #grip, body.autoside #side.pinned #grip { display:block; }
/* Drag to resize. */
#grip { position:absolute; top:0; right:-3px; width:6px; height:100%; cursor:col-resize;
        z-index:7; }
#grip:hover, #grip.dragging { background:var(--accent); opacity:.5; }
#sidepin { background:none; border:0; color:var(--dim); cursor:pointer; padding:0 7px;
        font:inherit; }
#sidepin.on { color:var(--accent); }
#tabs { display:flex; border-bottom:1px solid var(--rule); }
#tabs button { flex:1 1 0; background:transparent; color:var(--dim); border:0;
               border-bottom:2px solid transparent; padding:6px 4px; cursor:pointer;
               font:inherit; }
#tabs button.on { color:var(--fg); border-bottom-color:var(--accent); }
.pane { display:none; overflow:auto; padding:6px; flex:1 1 auto; }
.pane.on { display:block; }
.item { padding:3px 6px; border-radius:4px; cursor:pointer; white-space:nowrap;
        overflow:hidden; text-overflow:ellipsis; }
.item:hover { background:var(--bg); }
.item.dir { color:var(--accent); }
.item.current { background:var(--bg); color:var(--fg); }
.kids { margin-left:12px; }
.out-1 { font-weight:600; }
.out-2 { margin-left:10px; } .out-3 { margin-left:20px; }
.out-4 { margin-left:30px; } .out-5 { margin-left:40px; } .out-6 { margin-left:50px; }

#main { flex:1 1 auto; min-width:0; display:flex; flex-direction:column; }
#doc { flex:1 1 auto; overflow:auto; }
/* The window's full width is the text column. No max-width cap. */
article { width:100%; max-width:none; padding:24px 32px;
          font:calc(var(--body-size) * var(--zoom))/var(--line) var(--body-font); }
article a { color:var(--link); }
article pre { overflow-x:auto; padding:12px 14px; border-radius:6px; position:relative; }
article code, article pre { font-family:var(--code-font);
                            font-size:calc(var(--code-size) * var(--zoom)); }
article pre.plain { background:var(--panel); }
article table { border-collapse:collapse; }
article td, article th { border:1px solid var(--rule); padding:4px 10px; }
article img { max-width:100%; }
article h1,article h2,article h3 { line-height:1.25; }
article mark { background:var(--accent); color:var(--bg); }
article mark.on { outline:2px solid var(--fg); }
.copy { position:absolute; top:6px; right:6px; opacity:0; transition:opacity .12s;
        background:var(--panel); color:var(--fg); border:1px solid var(--rule);
        border-radius:4px; padding:1px 7px; cursor:pointer; font:var(--ui-size) var(--ui-font); }
article pre:hover .copy { opacity:1; }

#editor { flex:1 1 auto; display:none; }
#editor.show { display:block; }
#editor textarea { width:100%; height:100%; resize:none; border:0; outline:0;
  background:var(--bg); color:var(--fg); padding:20px 28px;
  font-family:var(--code-font); font-size:calc(var(--code-size) * var(--zoom));
  line-height:var(--line); }

#fm { display:none; padding:8px 32px; color:var(--dim); background:var(--panel);
      border-bottom:1px solid var(--rule); font-family:var(--code-font);
      font-size:var(--ui-size); white-space:pre-wrap; }
#fm.show { display:block; }

#options { display:none; position:absolute; inset:0; background:rgba(0,0,0,.45); z-index:30; }
#options.show { display:flex; align-items:center; justify-content:center; }
#panel { width:720px; height:440px; background:var(--bg); border:1px solid var(--rule);
         border-radius:12px; display:flex; flex-direction:column; overflow:hidden; }
#phead { display:flex; align-items:center; gap:10px; padding:11px 15px;
         border-bottom:1px solid var(--rule); background:var(--panel); }
#phead h2 { margin:0; font-size:15px; font-weight:600; }
#search { flex:1; background:var(--bg); border:1px solid var(--rule); color:var(--fg);
          border-radius:6px; padding:5px 10px; font:inherit; }
#pbody { flex:1; display:flex; min-height:0; }
#rail { width:160px; flex:0 0 auto; background:var(--panel);
        border-right:1px solid var(--rule); padding:8px 6px; }
#rail button { display:block; width:100%; text-align:left; background:none; border:0;
        color:var(--dim); padding:7px 10px; border-radius:6px; cursor:pointer; font:inherit; }
#rail button:hover { background:var(--bg); color:var(--fg); }
#rail button.on { background:var(--bg); color:var(--fg); box-shadow:inset 2px 0 0 var(--accent); }
#sets { flex:1; padding:12px 16px; overflow:auto; }
.grp h4 { margin:0 0 6px; font-size:11px; letter-spacing:.8px; text-transform:uppercase;
        color:var(--dim); font-weight:600; }
.set { display:grid; grid-template-columns:1fr 210px 24px; align-items:center; gap:12px;
        padding:7px 0; border-bottom:1px solid var(--rule); }
.set .lab { font-size:13px; }
.set .sub { display:block; color:var(--dim); font-size:11px; margin-top:1px; }
.set input, .set select { width:100%; background:var(--panel); color:var(--fg);
        border:1px solid var(--rule); border-radius:6px; padding:4px 8px; font:inherit; }
.set input[type=checkbox] { width:auto; }
.set .rst { background:none; border:0; color:var(--dim); cursor:pointer; opacity:0;
        font-size:14px; border-radius:4px; }
.set:hover .rst { opacity:1; }
.set .rst:hover { color:var(--accent); }
.sw { display:flex; gap:5px; flex-wrap:wrap; }
.sw button { width:28px; height:20px; border-radius:5px; border:1px solid var(--rule);
        cursor:pointer; padding:0; }
.sw button.on { outline:2px solid var(--accent); outline-offset:1px; }
#pfoot { display:flex; align-items:center; gap:10px; padding:9px 15px;
        border-top:1px solid var(--rule); background:var(--panel); }
#pfoot .grow { flex:1; color:var(--dim); font-size:12px; }
#pfoot button { background:var(--bg); border:1px solid var(--rule); color:var(--fg);
        border-radius:6px; padding:4px 14px; cursor:pointer; font:inherit; }
#pfoot button.pri { border-color:var(--accent); color:var(--accent); }
#warn { color:#e5c07b; padding:6px 0; }

button, select, input { font:inherit; }
button { cursor:pointer; }
button:focus-visible, select:focus-visible, input:focus-visible, .item:focus-visible { outline:2px solid var(--accent); outline-offset:2px; }
#bar { min-height:48px; gap:5px; flex-wrap:wrap; }
#bar button { min-height:30px; border-color:transparent; }
#bar button:hover { border-color:var(--rule); }
#brand { display:flex; align-items:center; gap:8px; font-weight:650; margin:0 10px 0 2px; white-space:nowrap; }
#brand svg { width:27px; height:27px; }
#name { min-width:80px; }
#note { position:fixed; right:18px; bottom:18px; max-width:min(600px,90vw); padding:10px 16px; background:var(--panel); border:1px solid var(--rule); border-radius:6px; z-index:40; white-space:pre-wrap; user-select:text; }
#note:empty { display:none; }
#side { max-width:70vw; }
#sidehot { display:none; }
body.autoside #sidehot { display:block; }
#folder-tools { display:flex; gap:4px; padding:8px; border-bottom:1px solid var(--rule); }
#folder-tools button { border:1px solid var(--rule); background:transparent; color:var(--fg); border-radius:4px; padding:5px 8px; }
#folder-open { flex:1; text-align:left; }
#folder-path { padding:8px 12px; color:var(--dim); overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-size:12px; }
#tree-filter { margin:0 8px 6px; width:calc(100% - 16px); background:var(--bg); color:var(--fg); border:1px solid var(--rule); border-radius:4px; padding:6px 8px; }
.item { display:block; width:100%; border:0; text-align:left; background:transparent; color:var(--fg); font:inherit; padding:5px 7px; min-height:28px; }
.item.dir { color:var(--fg); }
.item.current { box-shadow:inset 2px 0 var(--accent); }
.tree-message { color:var(--dim); padding:10px; white-space:normal; }
#editor { min-height:0; }
#text { tab-size:4; }
#panel { width:min(900px,94vw); height:min(660px,90vh); min-width:480px; min-height:320px; resize:both; max-width:98vw; max-height:96vh; border-radius:8px; }
#phead h2 { font-size:18px; }
#phead { padding:16px; }
#rail { width:170px; }
#sets { padding:8px 20px 20px; }
.set { grid-template-columns:minmax(120px,1fr) minmax(120px,220px) 26px; padding:13px 0; }
.set .sub { font-size:12px; line-height:1.5; }
.set .rst { opacity:.65; }
.set .rst:focus { opacity:1; }
.sw button { width:34px; height:28px; }
#pfoot .pri { color:var(--fg); border-color:var(--rule); }
#phead button { background:transparent; color:var(--fg); border:0; font-size:20px; }
#fm { max-height:20vh; overflow:auto; }
@media (max-width:800px) { #brand span, #b-theme, #b-pin, #bar .sp { display:none; } #bar { gap:2px; } #bar button { padding:4px 6px; } #rail { width:125px; } .set { grid-template-columns:minmax(100px,1fr) minmax(100px,150px) 24px; gap:7px; } }
@media (prefers-reduced-motion:reduce) { * { transition:none!important; } }
</style></head><body>

<div id="hot"></div>
<header id="bar" aria-label="Toolbar">
  <div id="brand"><svg viewBox="0 0 64 64" aria-hidden="true"><rect x="4" y="4" width="56" height="56" rx="9" fill="#223d58"/><path d="M13 10h32v44H13z" fill="#ddecf4"/><path d="M13 10h6v44h-6z" fill="#479bcd"/><path d="M24 21h15m-15 8h15m-15 8h15m-15 8h12" stroke="#738fa0" stroke-width="2"/><path d="m33 48 16-27 5 3-16 27-7 4z" fill="#f4b74d" stroke="#1c2c3e" stroke-width="2"/></svg><span id="app-name"></span></div>
  <button id="b-new" title="New note (Ctrl+N)">New</button>
  <button id="b-side" aria-label="Toggle sidebar" title="File tree (Ctrl+B)">&#9776;</button>
  <button id="b-open" title="Open (Ctrl+O)">Open</button>
  <button id="b-saveas" title="Save a copy (Ctrl+Shift+S)">Save as</button>
  <button id="b-save" title="Save (Ctrl+S)">Save</button>
  <button id="b-view" title="Rendered or plain text (Ctrl+U)">Text</button>
  <button id="b-edit" title="Edit the source (Ctrl+E)">Edit</button>
  <span class="sp"></span>
  <button id="b-find" title="Find (Ctrl+F)">Find</button>
  <button id="b-zoomout" title="Zoom out (Ctrl+-)">&minus;</button>
  <button id="b-zoomin" title="Zoom in (Ctrl+=)">+</button>
  <span class="sp"></span>
  <select id="b-theme" title="Theme"></select>
  <button id="b-opts" title="Options (Ctrl+,)">⚙ Options</button>
  <span id="name">no file open</span>
  <span id="note" role="status" aria-live="polite"></span>
  <button id="b-pin" aria-label="Toggle toolbar reveal" title="Keep this bar visible">&#9679;</button>
</header>
<div id="peek">&#9662;</div>

<div id="find">
  <input id="find-text" aria-label="Find in document" placeholder="Find in document" autocomplete="off">
  <button id="find-prev">Previous</button>
  <button id="find-next">Next</button>
  <span id="hits"></span>
  <button id="find-close">Close</button>
</div>

<div id="row">
  <div id="sidehot"></div>
  <nav id="side" aria-label="Workspace">
    <div id="grip" role="separator" aria-label="Sidebar width" aria-orientation="vertical" tabindex="0" title="Drag or use arrow keys to resize"></div>
    <div id="tabs">
      <button data-pane="files" class="on">Files</button>
      <button data-pane="outline">Outline</button>
      <button data-pane="recent">Recent</button>
      <button id="sidepin" aria-label="Pin sidebar" title="Keep the pane open">&#9679;</button>
    </div>
    <div id="folder-tools"><button id="folder-open" title="Open folder (Ctrl+Shift+O)">Open folder…</button><button id="folder-up" title="Parent folder" aria-label="Parent folder">↑</button><button id="folder-refresh" title="Refresh folder" aria-label="Refresh folder">↻</button></div>
    <div id="folder-path"></div>
    <input id="tree-filter" aria-label="Filter loaded files" placeholder="Filter loaded files…">
    <div id="pane-files" class="pane on"></div>
    <div id="pane-outline" class="pane"></div>
    <div id="pane-recent" class="pane"></div>
  </nav>
  <div id="main">
    <div id="fm"></div>
    <div id="doc"><article id="article"></article></div>
    <div id="editor"><textarea id="text" aria-label="Document editor" spellcheck="false"></textarea></div>
  </div>
</div>

<div id="options"><div id="panel" role="dialog" aria-modal="true" aria-labelledby="options-title">
  <div id="phead"><h2 id="options-title">Options</h2><input id="search" aria-label="Search settings" placeholder="Search settings"><button id="opt-x" aria-label="Close options">×</button></div>
  <div id="pbody"><div id="rail"></div><div id="sets"></div></div>
  <div id="pfoot">
    <span class="grow" id="pnote">Changes apply as you make them.</span>
    <button id="opt-reset">Reset all</button>
    <button class="pri" id="opt-close">Done</button>
  </div>
</div></div>

<script>
const $ = id => document.getElementById(id);
const send = o => window.ipc.postMessage(JSON.stringify(o));
let requestId = 0, noteTimer, optionsFocus;
const pendingFolders = new Map();
let state = { settings:{}, defaults:{}, themes:[], fonts:[], path:"", dirty:false, editing:false };
// Grouped so each screen is short. Remembered state - window size, last file,
// scroll position - is not a setting and is deliberately not listed.
const GROUPS = {
  Appearance: ["theme", "chrome", "zoom"],
  Workspace: ["sidebar", "sidebar_width", "sidebar_tab", "show_hidden", "restore_last_file", "close_to_tray"],
  Editor: ["word_wrap", "tab_size"],
  Fonts: ["ui_font", "body_font", "code_font", "ui_size", "body_size",
          "code_size", "line_height", "ligatures"],
  Document: ["view_mode", "syntax_colour", "highlight_limit_kb",
             "plain_text_above_mb"],
};
const CHOICES = {
  view_mode: ["rendered", "source"],
  chrome: ["auto", "always"],
  sidebar: ["auto", "always", "off"],
  sidebar_tab: ["files", "outline", "recent"],
};
const LABELS = {
 theme:["Color theme","Colors for the editor, reader and workspace."],
 chrome:["Toolbar","Keep controls visible or reveal them at the top edge."],
 sidebar:["Sidebar","Dock the browser, reveal it from the left edge, or hide it."],
 sidebar_width:["Sidebar width","Width in pixels; drag the divider to resize."],
 sidebar_tab:["Sidebar tab","Choose Files, Outline or Recent."],
 show_hidden:["Show hidden files","Include files and folders beginning with a dot."],
 restore_last_file:["Reopen last file","Continue with your last document at startup."],
 close_to_tray:["Close to system tray","Keep the note open in the Windows tray. Use Quit to exit."],
 word_wrap:["Word wrap","Wrap long lines in the editor."], tab_size:["Tab width","Spaces inserted by Tab, from 1 to 8."],
 ui_font:["Interface font","Toolbar, file browser and options."], body_font:["Reading font","Rendered Markdown paragraphs and headings."], code_font:["Code font","Editor and code blocks; monospace fonts."],
 ui_size:["Interface size","Pixels."], body_size:["Reading size","Pixels before zoom."], code_size:["Code size","Pixels before zoom."],
 line_height:["Line spacing","Line height as a multiple of the font size."], ligatures:["Font ligatures","Allow the font to join character combinations."],
 zoom:["Document zoom","1 is 100%; Ctrl+0 resets it."], view_mode:["Reading mode","Rendered Markdown or syntax-colored source."],
 syntax_colour:["Syntax highlighting","Color source files and fenced code blocks."],
 highlight_limit_kb:["Highlight limit","Skip syntax highlighting above this size in KB."],
 plain_text_above_mb:["Large file threshold","Show files above this size in MB as plain text."]
};
const RANGES = { ui_size:[10,28,1], body_size:[10,48,1], code_size:[10,40,1], line_height:[1,2.5,.05], zoom:[.5,3,.1], sidebar_width:[180,640,10], tab_size:[1,8,1], highlight_limit_kb:[1,4096,1], plain_text_above_mb:[1,100,1] };
const choiceLabel = x => ({always:"Always visible",auto:"Reveal at edge",off:"Hidden",source:"Source text",rendered:"Rendered Markdown",files:"Files",outline:"Outline",recent:"Recent"}[x] || x);
let activeGroup = "Appearance";

const app = {
  init(s) {
    state.name = s.name; state.trayAvailable = s.trayAvailable;
    document.title = s.name; $("app-name").textContent = s.name;
    $("options-title").textContent = "Options";
    $("pnote").textContent = s.name + " " + s.version + " · Changes save automatically.";
    state.settings = s.settings; state.defaults = s.defaults;
    state.themes = s.themes; state.fonts = s.fonts;
    const sel = $("b-theme");
    sel.innerHTML = "";
    for (const t of s.themes) {
      const o = document.createElement("option");
      o.value = t.id; o.textContent = t.name; sel.appendChild(o);
    }
    sel.value = state.settings.theme;
    app.applyTheme(s.theme);
    app.applySettings(s.settings);
    if (s.missingFonts && s.missingFonts.length)
      app.note("font not on this machine: " + s.missingFonts.join(", "));
    if ($("options").classList.contains("show")) app.drawOptions();
  },
  applyTheme(t) {
    const r = document.documentElement.style;
    r.setProperty("--bg", t.bg); r.setProperty("--fg", t.fg);
    r.setProperty("--panel", t.panel); r.setProperty("--bar", t.bar);
    r.setProperty("--rule", t.rule); r.setProperty("--link", t.link);
    r.setProperty("--dim", t.dim); r.setProperty("--accent", t.accent);
  },
  applySettings(s) {
    state.settings = s;
    const r = document.documentElement.style;
    r.setProperty("--ui-font", s.ui_font); r.setProperty("--body-font", s.body_font);
    r.setProperty("--code-font", s.code_font);
    r.setProperty("--ui-size", s.ui_size + "px");
    r.setProperty("--body-size", s.body_size + "px");
    r.setProperty("--code-size", s.code_size + "px");
    r.setProperty("--line", s.line_height);
    r.setProperty("--side-w", s.sidebar_width + "px");
    $("text").wrap = s.word_wrap ? "soft" : "off";
    $("text").style.tabSize = s.tab_size;
    $("sidehot").hidden = s.sidebar !== "auto";
    r.setProperty("--zoom", s.zoom);
    document.body.style.fontVariantLigatures = s.ligatures ? "normal" : "none";
    $("side").classList.toggle("hidden", s.sidebar === "off");
    document.body.classList.toggle("autoside", s.sidebar === "auto");
    $("side").classList.toggle("pinned", s.sidebar === "always");
    $("sidepin").classList.toggle("on", s.sidebar === "always");
    $("b-side").classList.toggle("on", s.sidebar !== "off");
    $("b-theme").value = s.theme;
    $("grip").setAttribute("aria-valuenow", s.sidebar_width);
    $("grip").setAttribute("aria-valuemin", 180);
    $("grip").setAttribute("aria-valuemax", 640);
    document.body.classList.toggle("auto", s.chrome === "auto");
    $("b-view").textContent = s.view_mode === "source" ? "Rendered" : "Text";
    $("b-view").classList.toggle("on", s.view_mode === "source");
    for (const b of document.querySelectorAll("#tabs button[data-pane]"))
      b.classList.toggle("on", b.dataset.pane === s.sidebar_tab);
    for (const p of ["files","outline","recent"])
      $("pane-" + p).classList.toggle("on", p === s.sidebar_tab);
    for (const id of ["folder-tools","folder-path","tree-filter"]) $(id).style.display = s.sidebar_tab === "files" ? "" : "none";
    if ($("options").classList.contains("show")) app.drawOptions();
  },
  setDocument(d) {
    $("article").innerHTML = d.html;
    $("name").textContent = d.name || "no file open";
    state.path = d.path || "";
    app.note(d.note || "");
    $("fm").textContent = d.frontMatter || "";
    $("fm").classList.toggle("show", !!d.frontMatter);
    app.outline(d.outline || []);
    app.addCopyButtons();
    app.wireLinks();
    $("doc").scrollTop = (d.scroll || 0) * $("doc").scrollHeight;
    app.setDirty(!!d.dirty);
    for (const row of document.querySelectorAll("#pane-files [data-path]")) row.classList.toggle("current", row.dataset.path === state.path);
    if ($("find").classList.contains("show")) runFind($("find-text").value);
  },
  setEditorText(t) { if ($("text").value !== t) $("text").value = t; },
  note(t) { clearTimeout(noteTimer); $("note").textContent = t || ""; if (t) noteTimer = setTimeout(() => $("note").textContent = "", 9000); },
  setDirty(d) {
    state.dirty = d;
    $("b-save").classList.toggle("on", d);
    $("b-save").textContent = d ? "Save *" : "Save";
  },
  outline(list) {
    const pane = $("pane-outline"); pane.innerHTML = "";
    if (!list.length) { pane.innerHTML = '<div class="item">No headings</div>'; return; }
    for (const h of list) {
      const el = document.createElement("button");
      el.className = "item out-" + h.level; el.textContent = h.text;
      el.onclick = () => {
        const target = document.getElementById(h.anchor);
        if (target) target.scrollIntoView({ block:"start" });
      };
      pane.appendChild(el);
    }
  },
  setTree(d) {
    pendingFolders.clear();
    $("folder-path").textContent = d.dir;
    $("folder-path").title = d.dir;
    $("folder-up").disabled = !d.parent;
    $("pane-files").replaceChildren(app.entries(d.entries || []));
    app.filterTree();
  },
  entries(list) {
    const box = document.createElement("div");
    if (!list.length) { const empty = document.createElement("div"); empty.className="tree-message"; empty.textContent="This folder is empty."; box.appendChild(empty); }
    for (const e of list) {
      const branch = document.createElement("div"); branch.className="branch";
      const el = document.createElement("button");
      el.className = "item" + (e.dir ? " dir" : "") + (e.path === state.path ? " current" : "");
      el.dataset.path = e.path; el.dataset.name = e.name; el.title = e.path;
      el.textContent = (e.dir ? "▸  " : "·  ") + e.name;
      if (e.dir) {
        el.setAttribute("aria-expanded", "false");
        el.onclick = () => {
          const open = el.getAttribute("aria-expanded") === "true";
          el.setAttribute("aria-expanded", String(!open));
          el.textContent = (open ? "▸  " : "▾  ") + e.name;
          const kids = branch.querySelector(":scope > .kids");
          if (kids) kids.remove();
          pendingFolders.delete(e.path);
          if (!open) {
            const request = ++requestId;
            pendingFolders.set(e.path, { el, branch, request });
            const loading = document.createElement("div"); loading.className="kids tree-message"; loading.textContent="Loading…"; branch.appendChild(loading);
            send({ cmd:"expand", path:e.path, request });
          }
        };
      } else {
        el.onclick = () => send({ cmd:"openPath", path:e.path });
      }
      branch.appendChild(el); box.appendChild(branch);
    }
    return box;
  },
  setEntries(d) {
    const pending = pendingFolders.get(d.path);
    if (!pending || pending.request !== d.request || !pending.el.isConnected) return;
    pendingFolders.delete(d.path);
    pending.branch.querySelector(":scope > .kids")?.remove();
    const kids = app.entries(d.entries || []); kids.className="kids";
    if (d.error) { kids.textContent = "Cannot read folder: " + d.error; kids.classList.add("tree-message"); }
    pending.branch.appendChild(kids);
    app.filterTree();
  },
  filterTree() {
    const query = $("tree-filter").value.toLowerCase();
    const visit = parent => {
      let visible = false;
      for (const branch of parent.children) {
        if (!branch.classList.contains("branch")) continue;
        const row = branch.firstElementChild;
        const kids = branch.querySelector(":scope > .kids");
        const childMatch = kids ? visit(kids) : false;
        const match = row.dataset.name.toLowerCase().includes(query);
        branch.hidden = !!query && !match && !childMatch && !row.classList.contains("dir");
        visible ||= !branch.hidden;
      }
      return visible;
    };
    const root = $("pane-files").firstElementChild;
    if (root) visit(root);
  },
  setRecents(list) {
    const pane = $("pane-recent"); pane.innerHTML = "";
    if (!list.length) { pane.innerHTML = '<div class="item">Nothing yet</div>'; return; }
    for (const p of list) {
      const el = document.createElement("button");
      el.className = "item"; el.textContent = p.split(/[\\/]/).pop(); el.title = p;
      el.onclick = () => send({ cmd:"openPath", path:p });
      pane.appendChild(el);
    }
  },
  addCopyButtons() {
    for (const pre of document.querySelectorAll("#article pre")) {
      const b = document.createElement("button");
      b.className = "copy"; b.textContent = "Copy";
      b.onclick = () => {
        navigator.clipboard.writeText(pre.innerText.replace(/\nCopy$/, ""));
        b.textContent = "Copied"; setTimeout(() => b.textContent = "Copy", 1200);
      };
      pre.appendChild(b);
    }
  },
  wireLinks() {
    for (const a of document.querySelectorAll("#article a")) {
      if (a.dataset.external) a.onclick = e => { e.preventDefault(); send({ cmd:"external", url:a.getAttribute("href") }); };
      else if (a.dataset.open) a.onclick = e => { e.preventDefault(); send({ cmd:"openPath", path:a.dataset.open }); };
    }
  },
  toggleEdit(on) {
    state.editing = on;
    $("editor").classList.toggle("show", on);
    $("doc").style.display = on ? "none" : "";
    $("b-edit").classList.toggle("on", on);
    $("b-edit").textContent = on ? "Preview" : "Edit";
    $("fm").hidden = on;
    if (on) $("text").focus();
    if ($("find").classList.contains("show")) runFind($("find-text").value);
  },
  options(open) {
    $("options").classList.toggle("show", open);
    $("row").inert = open; $("bar").inert = open; $("find").inert = open;
    if (open) { $("opt-reset").textContent="Reset all"; delete $("opt-reset").dataset.confirm; optionsFocus = document.activeElement; $("search").value = ""; app.buildOptions(); $("search").focus(); }
    else optionsFocus?.focus();
  },
  buildOptions() {
    const rail = $("rail");
    if (!rail.dataset.built) {
      for (const group of Object.keys(GROUPS)) {
        const b = document.createElement("button");
        b.textContent = group;
        b.onclick = () => { activeGroup = group; app.drawOptions(); };
        rail.appendChild(b);
      }
      rail.dataset.built = "1";
    }
    app.drawOptions();
  },
  drawOptions() {
    const focusKey = document.activeElement?.dataset.setting;
    const query = ($("search").value || "").toLowerCase();
    for (const b of $("rail").children)
      b.classList.toggle("on", b.textContent === activeGroup && !query);
    const box = $("sets");
    box.innerHTML = "";
    const groups = query ? Object.keys(GROUPS) : [activeGroup];
    for (const group of groups) {
      const keys = GROUPS[group].filter(k =>
        !query || (k + " " + (LABELS[k] || []).join(" ")).toLowerCase().includes(query));
      if (!keys.length) continue;
      const wrap = document.createElement("div");
      wrap.className = "grp";
      if (query) wrap.innerHTML = "<h4>" + group + "</h4>";
      for (const key of keys) wrap.appendChild(app.settingRow(key));
      box.appendChild(wrap);
    }
    if (!box.children.length)
      box.innerHTML = '<div class="grp" style="color:var(--dim)">Nothing matches.</div>';
    if (focusKey) box.querySelector(`[data-setting="${focusKey}"]`)?.focus();
  },
  settingRow(key) {
    const value = state.settings[key];
    const fallback = state.defaults[key];
    const row = document.createElement("div");
    row.className = "set";

    const label = document.createElement("div");
    label.className = "lab";
    label.textContent = LABELS[key]?.[0] || key.replace(/_/g, " ");
    label.id = "label-" + key;
    const help = document.createElement("span"); help.className="sub";
    help.textContent = (LABELS[key]?.[1] || "") + " Default: " + choiceLabel(String(fallback)) + ".";
    label.appendChild(help);
    row.appendChild(label);

    let control;
    if (key === "theme") {
      control = document.createElement("div");
      control.className = "sw";
      control.setAttribute("role", "group");
      for (const t of state.themes) {
        const b = document.createElement("button");
        b.style.background = t.bg;
        b.style.borderColor = t.rule;
        b.title = t.name; b.setAttribute("aria-label", t.name); b.setAttribute("aria-pressed", String(t.id === value));
        b.classList.toggle("on", t.id === value);
        b.onclick = () => send({ cmd:"setting", key:"theme", value:t.id });
        control.appendChild(b);
      }
    } else if (CHOICES[key]) {
      control = document.createElement("select");
      for (const choice of CHOICES[key]) {
        const o = document.createElement("option");
        o.value = choice; o.textContent = choiceLabel(choice);
        o.selected = choice === value;
        control.appendChild(o);
      }
    } else if (key.endsWith("_font")) {
      control = document.createElement("select");
      const mono = key === "code_font";
      const list = state.fonts.filter(f => !mono || f.monospace);
      const current = document.createElement("option");
      current.value = value; current.textContent = value; current.selected = true;
      control.appendChild(current);
      for (const f of list) {
        const o = document.createElement("option");
        o.value = f.name;
        o.textContent = f.name + (f.nerd ? "   (patched)" : "");
        control.appendChild(o);
      }
    } else if (typeof value === "boolean") {
      control = document.createElement("input");
      control.type = "checkbox";
      control.checked = value;
    } else if (typeof value === "number") {
      control = document.createElement("input");
      control.type = "number";
      const range = RANGES[key] || [1,100,1];
      [control.min, control.max, control.step] = range;
      control.value = value;
    } else {
      control = document.createElement("input");
      control.type = "text";
      control.value = value;
    }
    control.setAttribute("aria-labelledby", label.id);
    control.dataset.setting = key;
    if (key === "close_to_tray" && !state.trayAvailable) { control.disabled = true; help.textContent = "System tray is available in the Windows build."; }
    if (control.tagName !== "DIV") {
      control.onchange = () => {
        let next = control.type === "checkbox" ? control.checked : control.value;
        if (typeof fallback === "number") { if (!control.value || !control.checkValidity()) return; next = Number(next); }
        send({ cmd:"setting", key, value:next });
      };
    }
    row.appendChild(control);

    const reset = document.createElement("button");
    reset.className = "rst";
    reset.textContent = "\u21ba";
    reset.title = "Reset " + (LABELS[key]?.[0] || key);
    reset.setAttribute("aria-label", reset.title);
    reset.onclick = () => send({ cmd:"setting", key, value:fallback });
    row.appendChild(reset);
    return row;
  },
  warn(t) { $("pnote").textContent = t || "Changes apply as you make them."; }
};
window.app = app;

// find in page
let marks = [], at = -1, editorHits = [];
function clearMarks() {
  for (const m of marks) { const p = m.parentNode; p.replaceChild(document.createTextNode(m.textContent), m); p.normalize(); }
  marks = []; editorHits = []; at = -1; $("hits").textContent = "";
}
function runFind(text) {
  clearMarks();
  if (!text) return;
  if (state.editing) {
    const haystack = $("text").value.toLowerCase(), needle = text.toLowerCase();
    let pos = 0;
    while ((pos = haystack.indexOf(needle, pos)) !== -1) { editorHits.push(pos); pos += needle.length; }
    at = editorHits.length ? 0 : -1;
    focusMark(); return;
  }
  const walker = document.createTreeWalker($("article"), NodeFilter.SHOW_TEXT);
  const targets = []; let node;
  while ((node = walker.nextNode())) if (node.nodeValue.toLowerCase().includes(text.toLowerCase())) targets.push(node);
  for (const t of targets) {
    const parts = t.nodeValue.split(new RegExp("(" + text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") + ")", "ig"));
    const frag = document.createDocumentFragment();
    for (const p of parts) {
      if (p.toLowerCase() === text.toLowerCase() && p) { const m = document.createElement("mark"); m.textContent = p; marks.push(m); frag.appendChild(m); }
      else frag.appendChild(document.createTextNode(p));
    }
    t.parentNode.replaceChild(frag, t);
  }
  $("hits").textContent = marks.length ? "1 of " + marks.length : "no matches";
  if (marks.length) { at = 0; focusMark(); }
}
function focusMark() {
  if (state.editing) {
    $("hits").textContent = editorHits.length ? (at + 1) + " of " + editorHits.length : "No matches";
    if (at >= 0) { const text = $("text"); text.setSelectionRange(editorHits[at], editorHits[at] + $("find-text").value.length); text.scrollTop = text.value.slice(0, editorHits[at]).split("\n").length * parseFloat(getComputedStyle(text).lineHeight) - text.clientHeight/2; }
    return;
  }
  marks.forEach((m, i) => m.classList.toggle("on", i === at));
  if (marks[at]) marks[at].scrollIntoView({ block:"center" });
  $("hits").textContent = (at + 1) + " of " + marks.length;
}
function step(d) { const n = state.editing ? editorHits.length : marks.length; if (!n) return; at = (at + d + n) % n; focusMark(); }

let pinned = false;
$("hot").onmouseenter = () => $("bar").classList.add("show");
$("bar").onmouseleave = () => { if (!pinned) setTimeout(() => $("bar").classList.remove("show"), 200); };
$("bar").onmouseenter = () => $("bar").classList.add("show");
$("b-pin").onclick = () => send({cmd:"setting", key:"chrome", value:state.settings.chrome === "always" ? "auto" : "always"});
$("b-new").onclick = () => send({cmd:"new"});
$("b-saveas").onclick = () => send({cmd:"saveAs", text:$("text").value});
$("folder-open").onclick = () => send({cmd:"openFolder"});
$("folder-refresh").onclick = () => send({cmd:"refreshTree"});
$("folder-up").onclick = () => send({cmd:"treeUp"});
$("tree-filter").oninput = () => app.filterTree();
$("b-view").onclick = () => send({ cmd:"setting", key:"view_mode",
  value: state.settings.view_mode === "source" ? "rendered" : "source" });
$("b-open").onclick = () => send({ cmd:"open" });
$("b-save").onclick = () => send({ cmd:"save", text:$("text").value });
$("b-edit").onclick = () => { app.toggleEdit(!state.editing); if (!state.editing) send({cmd:"preview"}); };
$("b-side").onclick = () => send({ cmd:"setting", key:"sidebar",
  value: state.settings.sidebar === "off" ? "always" : "off" });
$("sidepin").onclick = () => send({ cmd:"setting", key:"sidebar",
  value: state.settings.sidebar === "always" ? "auto" : "always" });
$("sidehot").onmouseenter = () => $("side").classList.add("show");
$("side").onmouseleave = () => {
  if (state.settings.sidebar === "auto" && !dragging)
    setTimeout(() => { if (!dragging) $("side").classList.remove("show"); }, 200);
};
$("side").onmouseenter = () => $("side").classList.add("show");

let dragging = false;
$("grip").onmousedown = e => {
  e.preventDefault();
  dragging = true;
  $("grip").classList.add("dragging");
  const move = ev => {
    const w = Math.min(640, Math.max(180, ev.clientX));
    document.documentElement.style.setProperty("--side-w", w + "px");
  };
  const up = () => {
    dragging = false;
    $("grip").classList.remove("dragging");
    document.removeEventListener("mousemove", move);
    document.removeEventListener("mouseup", up);
    const w = parseInt(getComputedStyle(document.documentElement)
      .getPropertyValue("--side-w"), 10) || 260;
    send({ cmd:"setting", key:"sidebar_width", value:w });
  };
  document.addEventListener("mousemove", move);
  document.addEventListener("mouseup", up);
};
$("b-theme").onchange = e => send({ cmd:"setting", key:"theme", value:e.target.value });
$("b-opts").onclick = () => app.options(true);
$("opt-x").onclick = () => app.options(false);
$("options").onclick = e => { if (e.target === $("options")) app.options(false); };
$("opt-close").onclick = () => app.options(false);
$("opt-reset").onclick = () => { if ($("opt-reset").dataset.confirm) { send({cmd:"resetSettings"}); $("opt-reset").textContent="Reset all"; delete $("opt-reset").dataset.confirm; } else { $("opt-reset").dataset.confirm="1"; $("opt-reset").textContent="Confirm reset"; } };
$("search").oninput = () => app.drawOptions();
$("b-zoomin").onclick = () => send({ cmd:"setting", key:"zoom", value:Math.min(3, (state.settings.zoom || 1) + 0.1) });
$("b-zoomout").onclick = () => send({ cmd:"setting", key:"zoom", value:Math.max(0.5, (state.settings.zoom || 1) - 0.1) });
$("b-find").onclick = () => { $("find").classList.add("show"); $("find-text").focus(); };
$("find-close").onclick = () => { $("find").classList.remove("show"); clearMarks(); };
$("find-next").onclick = () => step(1);
$("find-prev").onclick = () => step(-1);
$("find-text").oninput = e => runFind(e.target.value);
$("text").oninput = () => { app.setDirty(true); send({cmd:"edit", text:$("text").value}); };
$("text").onkeydown = e => {
  if (e.key === "Tab" && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
    e.preventDefault(); document.execCommand("insertText", false, " ".repeat(state.settings.tab_size || 4));
  }
};
$("grip").onkeydown = e => { if (["ArrowLeft","ArrowRight"].includes(e.key)) { e.preventDefault(); send({cmd:"setting",key:"sidebar_width",value:state.settings.sidebar_width + (e.key === "ArrowRight" ? 20 : -20)}); } };
$("find-text").onkeydown = e => { if (e.key === "Enter") { e.preventDefault(); step(e.shiftKey ? -1 : 1); } };
for (const b of document.querySelectorAll("#tabs button[data-pane]"))
  b.onclick = () => send({ cmd:"setting", key:"sidebar_tab", value:b.dataset.pane });

$("doc").onscroll = () => {
  const h = $("doc").scrollHeight || 1;
  send({ cmd:"scroll", value:$("doc").scrollTop / h });
};

document.addEventListener("keydown", e => {
  const ctrl = e.ctrlKey || e.metaKey;
  if ($("options").classList.contains("show")) {
    if (e.key === "Escape") { e.preventDefault(); app.options(false); }
    if (e.key === "Tab") {
      const controls = [...$("panel").querySelectorAll("button,input,select")].filter(el => !el.disabled && el.offsetParent);
      const first = controls[0], last = controls[controls.length-1];
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    }
    return;
  }
  if (ctrl && e.key === ",") { e.preventDefault(); app.options(true); return; }
  if (ctrl && e.key.toLowerCase() === "n") { e.preventDefault(); send({cmd:"new"}); return; }
  if (ctrl && e.shiftKey && e.key.toLowerCase() === "o") { e.preventDefault(); send({cmd:"openFolder"}); return; }
  if (ctrl && e.shiftKey && e.key.toLowerCase() === "s") { e.preventDefault(); $("b-saveas").onclick(); return; }
  if (ctrl && e.key.toLowerCase() === "o") { e.preventDefault(); send({ cmd:"open" }); }
  else if (ctrl && e.key.toLowerCase() === "s") { e.preventDefault(); send({ cmd:"save", text:$("text").value }); }
  else if (ctrl && e.key.toLowerCase() === "e") { e.preventDefault(); $("b-edit").onclick(); }
  else if (ctrl && e.key.toLowerCase() === "b") { e.preventDefault(); $("b-side").onclick(); }
  else if (ctrl && e.key.toLowerCase() === "f") { e.preventDefault(); $("b-find").onclick(); }
  else if (ctrl && e.key.toLowerCase() === "u") { e.preventDefault(); $("b-view").onclick(); }
  else if (ctrl && (e.key === "=" || e.key === "+")) { e.preventDefault(); $("b-zoomin").onclick(); }
  else if (ctrl && e.key === "-") { e.preventDefault(); $("b-zoomout").onclick(); }
  else if (ctrl && e.key === "0") { e.preventDefault(); send({ cmd:"setting", key:"zoom", value:1 }); }
  else if (e.key === "Escape") { $("find-close").onclick(); app.options(false); }
  else if (e.key === "F3") step(e.shiftKey ? -1 : 1);
});
send({ cmd:"ready" });
</script></body></html>"##;
