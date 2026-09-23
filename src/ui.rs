/// The page shell. Loaded once; everything after that arrives over IPC, so the
/// document can change without rebuilding the window.
pub const SHELL: &str = r##"<!doctype html><html><head><meta charset="utf-8"><style>
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
#side { width:var(--side-w); flex:0 0 auto; background:var(--panel); display:flex;
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
</style></head><body>

<div id="hot"></div>
<header id="bar">
  <button id="b-side" title="File tree (Ctrl+B)">&#9776;</button>
  <button id="b-open" title="Open (Ctrl+O)">Open</button>
  <button id="b-save" title="Save (Ctrl+S)">Save</button>
  <button id="b-view" title="Rendered or plain text (Ctrl+U)">Text</button>
  <button id="b-edit" title="Edit the source (Ctrl+E)">Edit</button>
  <span class="sp"></span>
  <button id="b-find" title="Find (Ctrl+F)">Find</button>
  <button id="b-zoomout" title="Zoom out (Ctrl+-)">&minus;</button>
  <button id="b-zoomin" title="Zoom in (Ctrl+=)">+</button>
  <span class="sp"></span>
  <select id="b-theme" title="Theme"></select>
  <button id="b-opts" title="Options">Options</button>
  <span id="name">no file open</span>
  <span id="note"></span>
  <button id="b-pin" title="Keep this bar visible">&#9679;</button>
</header>
<div id="peek">&#9662;</div>

<div id="find">
  <input id="find-text" placeholder="Find in document" autocomplete="off">
  <button id="find-prev">Previous</button>
  <button id="find-next">Next</button>
  <span id="hits"></span>
  <button id="find-close">Close</button>
</div>

<div id="row">
  <div id="sidehot"></div>
  <nav id="side">
    <div id="grip" title="Drag to resize"></div>
    <div id="tabs">
      <button data-pane="files" class="on">Files</button>
      <button data-pane="outline">Outline</button>
      <button data-pane="recent">Recent</button>
      <button id="sidepin" title="Keep the pane open">&#9679;</button>
    </div>
    <div id="pane-files" class="pane on"></div>
    <div id="pane-outline" class="pane"></div>
    <div id="pane-recent" class="pane"></div>
  </nav>
  <div id="main">
    <div id="fm"></div>
    <div id="doc"><article id="article"></article></div>
    <div id="editor"><textarea id="text" spellcheck="false"></textarea></div>
  </div>
</div>

<div id="options"><div id="panel">
  <div id="phead"><h2>Options</h2><input id="search" placeholder="Search settings"></div>
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
let state = { settings:{}, defaults:{}, themes:[], fonts:[], path:"", dirty:false, editing:false };
// Grouped so each screen is short. Remembered state - window size, last file,
// scroll position - is not a setting and is deliberately not listed.
const GROUPS = {
  Appearance: ["theme", "chrome", "sidebar", "zoom"],
  Fonts: ["ui_font", "body_font", "code_font", "ui_size", "body_size",
          "code_size", "line_height", "ligatures"],
  Document: ["view_mode", "syntax_colour", "highlight_limit_kb",
             "plain_text_above_mb", "restore_last_file"],
};
const CHOICES = {
  view_mode: ["rendered", "source"],
  chrome: ["auto", "always"],
  sidebar: ["auto", "always", "off"],
  sidebar_tab: ["files", "outline", "recent"],
};
let activeGroup = "Appearance";

const app = {
  init(s) {
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
    r.setProperty("--zoom", s.zoom);
    document.body.style.fontVariantLigatures = s.ligatures ? "normal" : "none";
    $("side").classList.toggle("hidden", s.sidebar === "off");
    document.body.classList.toggle("autoside", s.sidebar === "auto");
    $("side").classList.toggle("pinned", s.sidebar === "always");
    $("sidepin").classList.toggle("on", s.sidebar === "always");
    $("b-side").classList.toggle("on", s.sidebar !== "off");
    $("b-theme").value = s.theme;
    document.body.classList.toggle("auto", s.chrome === "auto");
    $("b-view").textContent = s.view_mode === "source" ? "Rendered" : "Text";
    $("b-view").classList.toggle("on", s.view_mode === "source");
    for (const b of document.querySelectorAll("#tabs button"))
      b.classList.toggle("on", b.dataset.pane === s.sidebar_tab);
    for (const p of ["files","outline","recent"])
      $("pane-" + p).classList.toggle("on", p === s.sidebar_tab);
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
    app.setDirty(false);
    if (state.editing) app.toggleEdit(false);
  },
  setEditorText(t) { $("text").value = t; },
  note(t) { $("note").textContent = t || ""; },
  setDirty(d) {
    state.dirty = d;
    $("b-save").classList.toggle("on", d);
    $("b-save").textContent = d ? "Save *" : "Save";
  },
  outline(list) {
    const pane = $("pane-outline"); pane.innerHTML = "";
    if (!list.length) { pane.innerHTML = '<div class="item">No headings</div>'; return; }
    for (const h of list) {
      const el = document.createElement("div");
      el.className = "item out-" + h.level; el.textContent = h.text;
      el.onclick = () => {
        const target = document.getElementById(h.anchor);
        if (target) target.scrollIntoView({ block:"start" });
      };
      pane.appendChild(el);
    }
  },
  setTree(d) {
    const pane = $("pane-files"); pane.innerHTML = "";
    const up = document.createElement("div");
    up.className = "item dir"; up.textContent = ".. " + (d.dir || "");
    up.onclick = () => send({ cmd:"treeUp" });
    pane.appendChild(up);
    pane.appendChild(app.entries(d.entries || []));
  },
  entries(list) {
    const box = document.createElement("div");
    for (const e of list) {
      const el = document.createElement("div");
      el.className = "item" + (e.dir ? " dir" : "") + (e.path === state.path ? " current" : "");
      el.textContent = (e.dir ? "▸ " : "") + e.name;
      if (e.dir) {
        el.onclick = () => {
          const next = el.nextElementSibling;
          if (next && next.classList.contains("kids")) { next.remove(); el.textContent = "▸ " + e.name; }
          else { el.textContent = "▾ " + e.name; send({ cmd:"expand", path:e.path }); window.__expandAfter = el; }
        };
      } else if (e.openable) {
        el.onclick = () => send({ cmd:"openPath", path:e.path });
      } else { el.style.color = "var(--dim)"; }
      box.appendChild(el);
    }
    return box;
  },
  setEntries(list) {
    const anchor = window.__expandAfter; if (!anchor) return;
    const kids = app.entries(list); kids.className = "kids";
    anchor.after(kids); window.__expandAfter = null;
  },
  setRecents(list) {
    const pane = $("pane-recent"); pane.innerHTML = "";
    if (!list.length) { pane.innerHTML = '<div class="item">Nothing yet</div>'; return; }
    for (const p of list) {
      const el = document.createElement("div");
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
    if (on) $("text").focus();
  },
  options(open) {
    $("options").classList.toggle("show", open);
    if (open) { $("search").value = ""; app.buildOptions(); }
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
    const query = ($("search").value || "").toLowerCase();
    for (const b of $("rail").children)
      b.classList.toggle("on", b.textContent === activeGroup && !query);
    const box = $("sets");
    box.innerHTML = "";
    const groups = query ? Object.keys(GROUPS) : [activeGroup];
    for (const group of groups) {
      const keys = GROUPS[group].filter(k =>
        !query || k.replace(/_/g, " ").includes(query));
      if (!keys.length) continue;
      const wrap = document.createElement("div");
      wrap.className = "grp";
      if (query) wrap.innerHTML = "<h4>" + group + "</h4>";
      for (const key of keys) wrap.appendChild(app.settingRow(key));
      box.appendChild(wrap);
    }
    if (!box.children.length)
      box.innerHTML = '<div class="grp" style="color:var(--dim)">Nothing matches.</div>';
  },
  settingRow(key) {
    const value = state.settings[key];
    const fallback = state.defaults[key];
    const row = document.createElement("div");
    row.className = "set";

    const label = document.createElement("div");
    label.className = "lab";
    label.innerHTML = key.replace(/_/g, " ") +
      '<span class="sub">default: ' + String(fallback) + "</span>";
    row.appendChild(label);

    let control;
    if (key === "theme") {
      control = document.createElement("div");
      control.className = "sw";
      for (const t of state.themes) {
        const b = document.createElement("button");
        b.style.background = t.bg;
        b.style.borderColor = t.rule;
        b.title = t.name;
        b.classList.toggle("on", t.id === value);
        b.onclick = () => send({ cmd:"setting", key:"theme", value:t.id });
        control.appendChild(b);
      }
    } else if (CHOICES[key]) {
      control = document.createElement("select");
      for (const choice of CHOICES[key]) {
        const o = document.createElement("option");
        o.value = choice; o.textContent = choice;
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
      control.step = Number.isInteger(fallback) ? 1 : 0.05;
      control.value = value;
    } else {
      control = document.createElement("input");
      control.type = "text";
      control.value = value;
    }
    if (control.tagName !== "DIV") {
      control.onchange = () => {
        let next = control.type === "checkbox" ? control.checked : control.value;
        if (typeof fallback === "number") next = Number(next);
        send({ cmd:"setting", key, value:next });
      };
    }
    row.appendChild(control);

    const reset = document.createElement("button");
    reset.className = "rst";
    reset.textContent = "\u21ba";
    reset.title = "Back to the default";
    reset.onclick = () => send({ cmd:"setting", key, value:fallback });
    row.appendChild(reset);
    return row;
  },
  warn(t) { $("pnote").textContent = t || "Changes apply as you make them."; }
};
window.app = app;

// find in page
let marks = [], at = -1;
function clearMarks() {
  for (const m of marks) { const p = m.parentNode; p.replaceChild(document.createTextNode(m.textContent), m); p.normalize(); }
  marks = []; at = -1; $("hits").textContent = "";
}
function runFind(text) {
  clearMarks();
  if (!text) return;
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
  marks.forEach((m, i) => m.classList.toggle("on", i === at));
  if (marks[at]) marks[at].scrollIntoView({ block:"center" });
  $("hits").textContent = (at + 1) + " of " + marks.length;
}
function step(d) { if (!marks.length) return; at = (at + d + marks.length) % marks.length; focusMark(); }

let pinned = false;
$("hot").onmouseenter = () => $("bar").classList.add("show");
$("bar").onmouseleave = () => { if (!pinned) setTimeout(() => $("bar").classList.remove("show"), 200); };
$("bar").onmouseenter = () => $("bar").classList.add("show");
$("b-pin").onclick = () => {
  pinned = !pinned;
  $("bar").classList.toggle("pinned", pinned);
  $("b-pin").classList.toggle("on", pinned);
};
$("b-view").onclick = () => send({ cmd:"setting", key:"view_mode",
  value: state.settings.view_mode === "source" ? "rendered" : "source" });
$("b-open").onclick = () => send({ cmd:"open" });
$("b-save").onclick = () => send({ cmd:"save", text:$("text").value });
$("b-edit").onclick = () => { app.toggleEdit(!state.editing); if (state.editing) send({ cmd:"wantSource" }); };
$("b-side").onclick = () => send({ cmd:"setting", key:"sidebar",
  value: state.settings.sidebar === "off" ? "auto" : "off" });
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
    const w = Math.min(640, Math.max(150, ev.clientX));
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
$("opt-close").onclick = () => app.options(false);
$("opt-reset").onclick = () => send({ cmd:"resetSettings" });
$("search").oninput = () => app.drawOptions();
$("b-zoomin").onclick = () => send({ cmd:"setting", key:"zoom", value:Math.min(3, (state.settings.zoom || 1) + 0.1) });
$("b-zoomout").onclick = () => send({ cmd:"setting", key:"zoom", value:Math.max(0.5, (state.settings.zoom || 1) - 0.1) });
$("b-find").onclick = () => { $("find").classList.add("show"); $("find-text").focus(); };
$("find-close").onclick = () => { $("find").classList.remove("show"); clearMarks(); };
$("find-next").onclick = () => step(1);
$("find-prev").onclick = () => step(-1);
$("find-text").oninput = e => runFind(e.target.value);
$("text").oninput = () => app.setDirty(true);
for (const b of document.querySelectorAll("#tabs button"))
  b.onclick = () => send({ cmd:"setting", key:"sidebar_tab", value:b.dataset.pane });

$("doc").onscroll = () => {
  const h = $("doc").scrollHeight || 1;
  send({ cmd:"scroll", value:$("doc").scrollTop / h });
};

document.addEventListener("keydown", e => {
  const ctrl = e.ctrlKey || e.metaKey;
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
