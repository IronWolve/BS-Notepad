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

#bar { flex:0 0 auto; display:flex; align-items:center; gap:6px; padding:5px 8px;
       background:var(--bar); border-bottom:1px solid var(--rule); }
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

#row { flex:1 1 auto; display:flex; min-height:0; }
#side { width:var(--side-w); flex:0 0 auto; background:var(--panel); display:flex;
        flex-direction:column; border-right:1px solid var(--rule); min-width:0; }
#side.hidden { display:none; }
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

#options { display:none; position:absolute; inset:0; background:rgba(0,0,0,.45); }
#options.show { display:flex; align-items:flex-start; justify-content:center; }
#panel { background:var(--panel); border:1px solid var(--rule); border-radius:8px;
         margin-top:40px; width:640px; max-height:80%; overflow:auto; padding:16px 18px; }
#panel h2 { margin:0 0 12px; font-size:16px; }
.set { display:flex; align-items:center; gap:8px; padding:5px 0;
       border-bottom:1px solid var(--rule); }
.set label { flex:0 0 190px; color:var(--dim); }
.set input, .set select { flex:1 1 auto; background:var(--bg); color:var(--fg);
       border:1px solid var(--rule); border-radius:4px; padding:3px 7px; font:inherit; }
.set .def { flex:0 0 auto; color:var(--dim); font-size:11px; }
.set button { background:transparent; color:var(--fg); border:1px solid var(--rule);
       border-radius:4px; padding:2px 8px; cursor:pointer; font:inherit; }
#panel .foot { display:flex; gap:8px; justify-content:flex-end; margin-top:14px; }
#warn { color:#e5c07b; padding:6px 0; }
</style></head><body>

<header id="bar">
  <button id="b-side" title="File tree (Ctrl+B)">&#9776;</button>
  <button id="b-open" title="Open (Ctrl+O)">Open</button>
  <button id="b-save" title="Save (Ctrl+S)">Save</button>
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
</header>

<div id="find">
  <input id="find-text" placeholder="Find in document" autocomplete="off">
  <button id="find-prev">Previous</button>
  <button id="find-next">Next</button>
  <span id="hits"></span>
  <button id="find-close">Close</button>
</div>

<div id="row">
  <nav id="side">
    <div id="tabs">
      <button data-pane="files" class="on">Files</button>
      <button data-pane="outline">Outline</button>
      <button data-pane="recent">Recent</button>
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
  <h2>Options</h2>
  <div id="warn"></div>
  <div id="sets"></div>
  <div class="foot">
    <button id="opt-reset">Reset everything</button>
    <button id="opt-close">Close</button>
  </div>
</div></div>

<script>
const $ = id => document.getElementById(id);
const send = o => window.ipc.postMessage(JSON.stringify(o));
let state = { settings:{}, defaults:{}, themes:[], fonts:[], path:"", dirty:false, editing:false };

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
    $("side").classList.toggle("hidden", !s.sidebar_visible);
    $("b-side").classList.toggle("on", s.sidebar_visible);
    $("b-theme").value = s.theme;
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
    if (open) app.buildOptions();
  },
  buildOptions() {
    const box = $("sets"); box.innerHTML = "";
    const fontOptions = kind => {
      const list = state.fonts.filter(f => kind === "any" ? true : f.monospace);
      return list.map(f => `<option value="${f.name}"${f.nerd ? ' data-nerd="1"' : ''}>${f.name}${f.nerd ? "  (patched)" : ""}</option>`).join("");
    };
    for (const [key, value] of Object.entries(state.settings)) {
      if (key === "recents" || key === "last_path" || key === "last_scroll") continue;
      const row = document.createElement("div"); row.className = "set";
      const def = state.defaults[key];
      let field;
      if (key === "theme") {
        field = `<select data-k="${key}">${state.themes.map(t => `<option value="${t.id}">${t.name}</option>`).join("")}</select>`;
      } else if (key.endsWith("_font")) {
        const mono = key === "code_font";
        field = `<select data-k="${key}"><option value="${value}">${value} (current)</option>${fontOptions(mono ? "mono" : "any")}</select>`;
      } else if (typeof value === "boolean") {
        field = `<input type="checkbox" data-k="${key}"${value ? " checked" : ""}>`;
      } else if (typeof value === "number") {
        field = `<input type="number" step="${Number.isInteger(value) ? 1 : 0.05}" data-k="${key}" value="${value}">`;
      } else {
        field = `<input type="text" data-k="${key}" value="${String(value).replace(/"/g, "&quot;")}">`;
      }
      row.innerHTML = `<label>${key.replace(/_/g, " ")}</label>${field}
        <span class="def">default: ${def}</span><button data-r="${key}">Reset</button>`;
      box.appendChild(row);
    }
    for (const el of box.querySelectorAll("[data-k]")) {
      if (el.tagName === "SELECT" && el.dataset.k === "theme") el.value = state.settings.theme;
      el.onchange = () => {
        const k = el.dataset.k;
        let v = el.type === "checkbox" ? el.checked : el.value;
        if (typeof state.defaults[k] === "number") v = Number(v);
        send({ cmd:"setting", key:k, value:v });
      };
    }
    for (const el of box.querySelectorAll("[data-r]"))
      el.onclick = () => send({ cmd:"setting", key:el.dataset.r, value:state.defaults[el.dataset.r] });
  },
  warn(t) { $("warn").textContent = t || ""; }
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

$("b-open").onclick = () => send({ cmd:"open" });
$("b-save").onclick = () => send({ cmd:"save", text:$("text").value });
$("b-edit").onclick = () => { app.toggleEdit(!state.editing); if (state.editing) send({ cmd:"wantSource" }); };
$("b-side").onclick = () => send({ cmd:"setting", key:"sidebar_visible", value:!state.settings.sidebar_visible });
$("b-theme").onchange = e => send({ cmd:"setting", key:"theme", value:e.target.value });
$("b-opts").onclick = () => app.options(true);
$("opt-close").onclick = () => app.options(false);
$("opt-reset").onclick = () => send({ cmd:"resetSettings" });
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
  else if (ctrl && (e.key === "=" || e.key === "+")) { e.preventDefault(); $("b-zoomin").onclick(); }
  else if (ctrl && e.key === "-") { e.preventDefault(); $("b-zoomout").onclick(); }
  else if (ctrl && e.key === "0") { e.preventDefault(); send({ cmd:"setting", key:"zoom", value:1 }); }
  else if (e.key === "Escape") { $("find-close").onclick(); app.options(false); }
  else if (e.key === "F3") step(e.shiftKey ? -1 : 1);
});
send({ cmd:"ready" });
</script></body></html>"##;
