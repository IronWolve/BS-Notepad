/// The page shell. Loaded once; everything after that arrives over IPC, so the
/// document can change without rebuilding the window.
pub const SHELL: &str = r##"<!doctype html><html lang="en"><head><meta name="viewport" content="width=device-width, initial-scale=1"><title>Notes</title><meta charset="utf-8"><style>
:root {
 --bg:#1f1f1f; --fg:#d7d7d7; --panel:#181818; --bar:#252526;
 --rule:#343434; --link:#75beff; --dim:#a5a5a5; --accent:#3794ff;
 --ui-font:system-ui,sans-serif; --body-font:system-ui,sans-serif; --code-font:ui-monospace,monospace;
 --ui-size:13px; --body-size:16px; --code-size:14px; --line:1.65; --side-w:260px; --zoom:1;
 --hover:color-mix(in srgb,var(--fg) 7%,transparent);
 --selected:color-mix(in srgb,var(--accent) 18%,var(--panel));
}
* { box-sizing:border-box; }
html,body { margin:0; height:100%; background:var(--bg); color:var(--fg); }
body { display:flex; flex-direction:column; font:var(--ui-size)/1.45 var(--ui-font); }
button,input,select { font:inherit; }
button { color:inherit; cursor:pointer; }
button:disabled { opacity:.4; cursor:default; }
button:focus-visible,input:focus-visible,select:focus-visible,[tabindex]:focus-visible { outline:2px solid var(--accent); outline-offset:-2px; }
button { -webkit-tap-highlight-color:transparent; }
[hidden] { display:none!important; }
svg.ui-icon { width:16px; height:16px; flex:0 0 16px; fill:none; stroke:currentColor; stroke-width:1.5; stroke-linecap:round; stroke-linejoin:round; }
#hot { position:absolute; top:0; left:0; right:0; height:6px; z-index:5; }
#bar { min-height:48px; display:flex; align-items:center; gap:5px; padding:7px 12px; background:var(--bar); border-bottom:1px solid var(--rule); }
#brand { display:flex; align-items:center; gap:9px; margin-right:12px; font-size:13px; font-weight:650; white-space:nowrap; }
#brand svg { width:26px; height:26px; }
#bar button,#bar select { display:inline-flex; align-items:center; justify-content:center; gap:7px; min-height:32px; padding:5px 9px; color:var(--fg); background:transparent; border:1px solid transparent; border-radius:5px; white-space:nowrap; }
#bar button:hover,#bar select:hover { background:var(--hover); }
#bar button.on { background:var(--selected); }
#b-side.on { box-shadow:inset 0 -2px var(--accent); }
#bar .icon-only { width:30px; padding:5px; }
#bar .toolbar-group { display:flex; align-items:center; gap:2px; }
#bar .toolbar-end { display:flex; align-items:center; gap:5px; margin-left:auto; }
.sp { width:1px; height:20px; background:var(--rule); margin:0 6px; flex:0 0 1px; }
#bar #b-zoomreset { min-width:44px; font-size:12px; font-variant-numeric:tabular-nums; padding:4px; color:var(--dim); }
#b-save.on::after { content:""; width:5px; height:5px; border-radius:50%; background:var(--accent); }
body.auto #bar { position:absolute; top:0; left:0; right:0; z-index:10; transform:translateY(-100%); transition:transform .12s ease; }
body.auto #bar.show,body.auto #bar:focus-within { transform:none; }
#peek { display:none; position:absolute; top:0; left:50%; z-index:4; background:var(--bar); color:var(--dim); border:1px solid var(--rule); border-top:0; border-radius:0 0 5px 5px; padding:0 12px; }
body.auto #peek { display:block; }
#note { position:fixed; right:18px; bottom:18px; max-width:min(600px,90vw); padding:12px 16px; background:var(--panel); border:1px solid var(--rule); border-radius:6px; z-index:40; white-space:pre-wrap; box-shadow:0 6px 24px #0003; user-select:text; }
#note:empty { display:none; }
#find { display:none; align-items:center; gap:6px; padding:7px 12px; background:var(--panel); border-bottom:1px solid var(--rule); }
#find.show { display:flex; }
#find input { min-width:80px; width:260px; padding:5px 8px; border:1px solid var(--rule); border-radius:3px; background:var(--bg); color:var(--fg); }
#find button { border:0; border-radius:3px; padding:5px 8px; background:transparent; }
#find button:hover { background:var(--hover); }
#hits { color:var(--dim); font-size:12px; }
#row { flex:1; display:flex; min-height:0; position:relative; }
#side { position:relative; width:var(--side-w); max-width:70vw; flex:0 0 auto; background:var(--panel); display:flex; flex-direction:column; border-right:1px solid var(--rule); min-width:0; }
#side.hidden { display:none; }
#sidehot { display:none; position:absolute; inset:0 auto 0 0; width:6px; z-index:5; }
body.autoside #sidehot { display:block; }
body.autoside #side { position:absolute; inset:0 auto 0 0; z-index:6; transform:translateX(-100%); transition:transform .12s ease; box-shadow:8px 0 24px #0003; }
body.autoside #side.show,body.autoside #side:focus-within { transform:none; }
#grip { position:absolute; top:0; right:-3px; width:6px; height:100%; cursor:col-resize; z-index:7; }
#grip:hover,#grip.dragging { background:var(--accent); opacity:.6; }
#explorer-head { display:flex; align-items:center; min-height:40px; padding:0 10px 0 15px; gap:4px; }
#explorer-head strong { font-size:12px; font-weight:600; flex:1; }
.explorer-action { display:inline-flex; align-items:center; justify-content:center; width:26px; height:26px; padding:5px; background:transparent; color:var(--dim); border:0; border-radius:4px; }
.explorer-action:hover { color:var(--fg); background:var(--hover); }
#sidepin.on { color:var(--fg); background:var(--hover); }
#tabs { display:flex; padding:0 10px; gap:12px; border-bottom:1px solid var(--rule); }
#tabs button { display:flex; align-items:center; gap:5px; background:transparent; color:var(--dim); border:0; border-bottom:2px solid transparent; padding:7px 2px 8px; font-size:12px; }
#tabs button.on { color:var(--fg); border-bottom-color:var(--accent); }
#tabs .ui-icon { width:13px; height:13px; flex-basis:13px; }
#file-tools { padding:10px 12px 6px; }
#filter-wrap { position:relative; display:flex; align-items:center; }
#filter-wrap>.ui-icon { position:absolute; left:8px; width:13px; height:13px; color:var(--dim); pointer-events:none; }
#tree-filter { width:100%; min-width:0; height:29px; background:var(--bg); color:var(--fg); border:1px solid var(--rule); border-radius:4px; padding:4px 7px 4px 28px; font-size:12px; }
#tree-filter::placeholder { color:var(--dim); opacity:.75; }
#folder-tools { display:flex; align-items:center; min-height:34px; padding:2px 9px 2px 12px; gap:4px; }
#folder-tools .root-chevron { width:12px; color:var(--dim); }
#folder-path { overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-weight:600; flex:1; font-size:12px; user-select:text; }
#folder-tools .explorer-action { width:22px; height:24px; padding:3px; }
.pane { display:none; overflow:auto; min-height:0; flex:1; padding:0 0 12px; scrollbar-width:thin; scrollbar-color:var(--rule) transparent; }
.pane.on { display:block; }
.item { display:flex; align-items:center; gap:6px; width:100%; min-height:27px; padding:3px 12px 3px 10px; border:0; border-radius:0; background:transparent; color:var(--fg); text-align:left; white-space:nowrap; font:inherit; font-size:var(--ui-size); }
.item:hover { background:var(--hover); }
.item.current { background:var(--selected); box-shadow:inset 2px 0 var(--accent); }
.item:focus-visible { outline-offset:-1px; }
.item .chevron { width:12px; flex:0 0 12px; display:flex; align-items:center; justify-content:center; color:var(--dim); }
.item .chevron .ui-icon { width:12px; height:12px; flex-basis:12px; transition:transform .1s ease; }
.item[aria-expanded=true] .chevron .ui-icon { transform:rotate(90deg); }
.item .file-label { min-width:0; overflow:hidden; text-overflow:ellipsis; }
.file-icon { display:inline-flex; align-items:center; justify-content:center; width:16px; height:16px; flex:0 0 16px; }
.file-blue { color:#42b6f5; } .file-gold { color:#e6c34a; } .file-pink { color:#e784b5; }
.file-purple { color:#bf9aef; } .file-cyan { color:#6fcbd5; } .file-green { color:#9acb7c; }
.file-teal { color:#45b5a8; } .file-orange { color:#e7a075; } .file-muted { color:#96aab3; }
.light-icons .file-blue { color:#176fa3; } .light-icons .file-gold { color:#8a6500; } .light-icons .file-pink { color:#a33b74; }
.light-icons .file-purple { color:#7951a7; } .light-icons .file-cyan { color:#076e80; } .light-icons .file-green { color:#437521; }
.light-icons .file-teal { color:#087b6b; } .light-icons .file-orange { color:#a65823; } .light-icons .file-muted { color:#556b76; }
.item .file-icon { margin-right:2px; }
#document-icon,#root-icon { display:inline-flex; align-items:center; }
.set.theme-setting { grid-template-columns:minmax(120px,1fr) 26px; }
.theme-setting .lab { grid-column:1; }
.theme-setting .sw { grid-column:1 / -1; grid-row:2; padding:3px 0; gap:8px; }
.theme-setting .rst { grid-column:2; grid-row:1; }
.theme-setting .sw button { flex:1 0 100px; min-height:38px; }
.kids { margin-left:16px; border-left:1px solid color-mix(in srgb,var(--fg) 12%,transparent); }
.kids .item { padding-left:9px; }
.tree-message { padding:10px 16px; color:var(--dim); font-size:12px; line-height:1.6; white-space:normal; }
.tree-message strong { display:block; color:var(--fg); font-weight:500; }
#pane-recent .item { padding:7px 13px; gap:9px; }
.recent-label { display:flex; flex-direction:column; min-width:0; }
.recent-label .recent-path { display:block; color:var(--dim); font-size:11px; overflow:hidden; text-overflow:ellipsis; }
#pane-outline .item { padding:6px 14px; }
.out-1 { font-weight:600; } .out-2 { padding-left:26px!important; } .out-3 { padding-left:38px!important; }
.out-4,.out-5,.out-6 { padding-left:50px!important; }
.outline-symbol { font-family:var(--code-font); color:var(--dim); font-size:12px; }
#main { flex:1; min-width:0; display:flex; flex-direction:column; }
#document-head { display:flex; align-items:center; min-height:40px; background:var(--panel); border-bottom:1px solid var(--rule); }
#document-tab { min-width:0; flex:0 1 auto; display:flex; align-items:center; gap:8px; min-height:40px; max-width:65%; padding:0 16px; border-right:1px solid var(--rule); background:var(--bg); box-shadow:inset 0 2px var(--accent); }
#name { min-width:0; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; font-size:13px; }
#dirty-dot { font-size:10px; color:var(--fg); }
#document-actions { flex-shrink:0; margin-left:auto; display:flex; align-items:center; gap:3px; padding:0 9px; }
#document-actions button { display:flex; align-items:center; gap:6px; background:transparent; color:var(--dim); border:1px solid transparent; border-radius:4px; padding:4px 8px; font-size:12px; }
#document-actions button:hover { background:var(--hover); color:var(--fg); }
#document-actions button.on { background:var(--hover); color:var(--fg); border-color:var(--rule); }
#document-path { display:flex; align-items:center; gap:6px; min-height:27px; padding:4px 18px; color:var(--dim); font-size:11px; border-bottom:1px solid color-mix(in srgb,var(--rule) 40%,transparent); user-select:text; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; }
#document-path .crumb { overflow:hidden; text-overflow:ellipsis; }
#document-path .crumb-separator { color:var(--dim); opacity:.5; }
#doc { flex:1; overflow:auto; }
article { width:100%; padding:24px 32px; font:calc(var(--body-size) * var(--zoom))/var(--line) var(--body-font); }
article a { color:var(--link); }
article pre { overflow-x:auto; padding:14px 16px; border-radius:5px; position:relative; }
article code,article pre { font-family:var(--code-font); font-size:calc(var(--code-size) * var(--zoom)); }
article pre.plain { background:var(--panel); }
article table { border-collapse:collapse; }
article td,article th { border:1px solid var(--rule); padding:5px 10px; }
article img { max-width:100%; }
article h1,article h2,article h3 { line-height:1.3; }
article mark { background:var(--accent); color:var(--bg); }
article mark.on { outline:2px solid var(--fg); }
.copy { position:absolute; top:6px; right:6px; opacity:0; background:var(--panel); color:var(--fg); border:1px solid var(--rule); border-radius:4px; padding:2px 8px; font:var(--ui-size) var(--ui-font); }
article pre:hover .copy,.copy:focus-visible { opacity:1; }
#editor { flex:1; min-height:0; display:none; }
#editor.show { display:block; }
#text { width:100%; height:100%; resize:none; border:0; outline:0; background:var(--bg); color:var(--fg); padding:20px 28px; font-family:var(--code-font); font-size:calc(var(--code-size) * var(--zoom)); line-height:var(--line); tab-size:4; }
#fm { display:none; padding:8px 32px; max-height:20vh; overflow:auto; color:var(--dim); background:var(--panel); border-bottom:1px solid var(--rule); font-family:var(--code-font); font-size:var(--ui-size); white-space:pre-wrap; }
#fm.show { display:block; }
#options { display:none; position:absolute; inset:0; background:#0007; z-index:30; }
#options.show { display:flex; align-items:center; justify-content:center; }
#panel { width:min(900px,94vw); height:min(660px,90vh); min-width:480px; min-height:320px; resize:both; max-width:98vw; max-height:96vh; background:var(--bg); border:1px solid var(--rule); border-radius:8px; display:flex; flex-direction:column; overflow:hidden; box-shadow:0 18px 70px #0005; }
#phead { display:flex; align-items:center; gap:16px; padding:16px; border-bottom:1px solid var(--rule); background:var(--panel); }
#phead h2 { margin:0; font-size:18px; font-weight:600; }
#search { flex:1; min-width:0; background:var(--bg); border:1px solid var(--rule); color:var(--fg); border-radius:4px; padding:6px 10px; }
#phead button { background:transparent; color:var(--fg); border:0; font-size:20px; }
#pbody { flex:1; display:flex; min-height:0; }
#rail { width:170px; flex:0 0 auto; background:var(--panel); border-right:1px solid var(--rule); padding:8px 6px; }
#rail button { display:block; width:100%; text-align:left; background:none; border:0; color:var(--dim); padding:8px 10px; border-radius:4px; }
#rail button:hover { background:var(--hover); color:var(--fg); }
#rail button.on { background:var(--selected); color:var(--fg); box-shadow:inset 2px 0 var(--accent); }
#sets { flex:1; min-width:0; padding:8px 20px 20px; overflow:auto; }
.grp h4 { margin:12px 0 4px; font-size:13px; color:var(--dim); font-weight:600; }
.set { display:grid; grid-template-columns:minmax(120px,1fr) minmax(120px,220px) 26px; align-items:center; gap:12px; padding:13px 0; border-bottom:1px solid var(--rule); }
.set .lab { font-size:13px; }
.set .sub { display:block; color:var(--dim); font-size:12px; line-height:1.5; margin-top:2px; }
.set input,.set select { width:100%; background:var(--panel); color:var(--fg); border:1px solid var(--rule); border-radius:4px; padding:5px 8px; }
.set input[type=checkbox] { width:auto; accent-color:var(--accent); }
.set .rst { background:none; border:0; color:var(--dim); cursor:pointer; opacity:.7; font-size:14px; border-radius:4px; }
.set .rst:hover,.set .rst:focus { color:var(--fg); opacity:1; }
.sw { display:flex; gap:6px; flex-wrap:wrap; }
.sw button { min-width:88px; height:30px; border-radius:4px; font-size:11px; border:1px solid var(--rule); padding:0; }
.sw button.on { outline:2px solid var(--accent); outline-offset:1px; }
#pfoot { display:flex; align-items:center; gap:10px; padding:10px 15px; border-top:1px solid var(--rule); background:var(--panel); }
#pfoot .grow { flex:1; color:var(--dim); font-size:12px; }
#pfoot button { background:var(--bg); border:1px solid var(--rule); color:var(--fg); border-radius:4px; padding:5px 14px; }
@media (max-width:1080px) { #b-theme { display:none!important; } #brand span { display:none; } #brand { margin-right:5px; } }
@media (max-width:800px) { #bar { gap:2px; padding:6px 8px; } #bar button { padding:5px 7px; } #b-saveas .button-label,#b-new .button-label,#b-open .button-label,#b-find .button-label { display:none; } #b-pin,#zoom-group { display:none!important; } #bar .sp { margin:0 3px; } #rail { width:125px; } .set { grid-template-columns:minmax(100px,1fr) minmax(100px,150px) 24px; gap:7px; } #document-tab { padding:0 10px; } #document-actions { padding:0 5px; } #document-actions button { padding:4px 5px; } }
@media (prefers-reduced-motion:reduce) { * { transition:none!important; } }

/* Window, documents and popups share the selected palette. */
body { border:1px solid var(--rule); border-radius:10px; overflow:hidden; }
body.maximized { border-radius:0; }
body.auto.popup-open #bar { transform:none; }
#bar #brand #app-name { display:inline!important; }
#bar { min-height:42px; padding:4px 4px 4px 8px; gap:3px; }
#bar #brand { padding:3px 7px; margin-right:4px; border:0; gap:7px; }
#brand:hover #app-name { text-decoration:underline; text-underline-offset:4px; }
#brand svg { width:24px; height:24px; }
#bar #b-new,#bar #b-open,#bar #b-saveas,#bar #zoom-group,#bar #b-pin { display:none!important; }
#bar .sp { margin:0 3px; }
#bar .toolbar-end { margin-left:0; }
#bar #b-theme { min-width:90px; max-width:155px; }
#bar #drag-region { align-self:stretch; flex:1; min-width:24px; }
#window-controls { display:flex; align-items:center; gap:1px; margin-left:6px; }
#bar #window-controls button { width:33px; min-height:31px; padding:0; border-radius:4px; font:19px/1 system-ui,sans-serif; }
#bar #window-close:hover { background:#c42b32; color:white; }
#document-tabs { flex:1; min-width:0; display:flex; align-self:stretch; overflow-x:auto; scrollbar-width:thin; }
.document-tab { display:flex; align-items:center; gap:0; flex:0 0 auto; max-width:240px; min-width:90px; border-right:1px solid var(--rule); background:var(--panel); }
.document-tab.active { background:var(--bg); box-shadow:inset 0 2px var(--accent); }
.tab-label { display:flex; align-items:center; gap:7px; min-width:0; max-width:210px; flex:1; padding:10px 6px 10px 12px; color:var(--dim); border:0; background:transparent; font-size:var(--ui-size); }
.active .tab-label { color:var(--fg); }
.tab-label .tab-name { overflow:hidden; white-space:nowrap; text-overflow:ellipsis; }
.tab-dirty { font-size:9px; }
.tab-close { width:23px; height:24px; padding:0; margin-right:5px; border:0; border-radius:4px; background:transparent; color:var(--dim); font-size:17px; }
.tab-close:hover { background:var(--hover); color:var(--fg); }
#tab-new { flex:0 0 29px; border:0; background:transparent; color:var(--dim); align-self:stretch; font-size:20px; }
#tab-new:hover { background:var(--hover); }
#document-actions { padding-left:6px; border-left:1px solid var(--rule); }
#content-row { display:flex; flex:1; min-height:0; }
#content-main { flex:1; min-width:0; display:flex; flex-direction:column; }
#editor textarea { width:100%; height:100%; resize:none; border:0; outline:0; background:var(--bg); color:var(--fg); padding:20px 28px; font-family:var(--code-font); font-size:calc(var(--code-size) * var(--zoom)); line-height:var(--line); tab-size:4; }
#minimap { flex:0 0 108px; width:108px; position:relative; background:var(--panel); border-left:1px solid var(--rule); overflow:hidden; cursor:pointer; touch-action:none; user-select:none; }
#map-canvas { position:absolute; inset:0; width:100%; height:100%; pointer-events:none; }
#map-viewport { position:absolute; left:3px; right:3px; min-height:12px; background:color-mix(in srgb,var(--accent) 12%,transparent); border:1px solid color-mix(in srgb,var(--accent) 65%,transparent); border-radius:4px; cursor:grab; }
#map-viewport:active { cursor:grabbing; }
#b-map.on { background:var(--selected); }
.select-control { display:flex; align-items:center; justify-content:space-between; gap:8px; min-width:0; width:100%; text-align:left; padding:5px 8px; color:var(--fg); background:var(--panel); border:1px solid var(--rule); border-radius:4px; }
.select-control .selected-label { min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.select-control .select-arrow { color:var(--dim); font-size:11px; }
.popup { position:fixed; z-index:70; min-width:210px; max-width:min(420px,calc(100vw - 16px)); max-height:calc(100vh - 16px); overflow:auto; padding:5px; border:1px solid var(--rule); border-radius:8px; background:var(--panel); color:var(--fg); box-shadow:0 8px 32px #0005; }
.popup button { display:flex; align-items:center; gap:9px; width:100%; background:transparent; color:var(--fg); border:0; border-radius:4px; min-height:31px; padding:6px 10px; text-align:left; font-size:13px; }
.popup button:hover,.popup button:focus-visible { background:var(--hover); }
.popup button[aria-selected=true] { background:var(--selected); }
.popup .menu-label { flex:1; }
.popup .menu-hint { margin-left:16px; color:var(--dim); font-size:11px; white-space:nowrap; }
.popup .menu-symbol { width:17px; text-align:center; color:var(--dim); }
.popup hr { border:0; border-top:1px solid var(--rule); margin:5px 4px; }
#choice-search { width:100%; padding:7px 9px; border:1px solid var(--rule); border-radius:4px; margin:0 0 5px; color:var(--fg); background:var(--bg); }
#choice-list { max-height:300px; overflow:auto; }
#choice-list .choice-empty { padding:12px; color:var(--dim); }
#pfoot #footer-brand { border:0; background:transparent; color:var(--fg); padding:4px 0; font-weight:600; white-space:nowrap; }
#pfoot #footer-brand:hover { text-decoration:underline; }
#pfoot .grow { text-align:right; }
#help-overlay { display:none; position:absolute; inset:0; z-index:35; background:#0007; align-items:center; justify-content:center; }
#help-overlay.show { display:flex; }
#help-panel { width:min(840px,94vw); height:min(650px,90vh); min-width:440px; min-height:340px; max-width:98vw; max-height:96vh; display:flex; flex-direction:column; resize:both; overflow:hidden; background:var(--bg); border:1px solid var(--rule); border-radius:10px; box-shadow:0 20px 70px #0005; }
#help-head { display:flex; align-items:center; gap:17px; padding:19px 23px; border-bottom:1px solid var(--rule); background:var(--panel); }
#help-logo { width:84px; height:84px; object-fit:contain; }
#help-title { margin:0 0 5px; font-size:25px; }
#help-version { margin:0; color:var(--dim); font-size:12px; }
#help-close { margin-left:auto; align-self:flex-start; border:0; background:transparent; color:var(--dim); font-size:24px; }
#help-search-row { padding:12px 18px; border-bottom:1px solid var(--rule); }
#help-search { width:100%; border:1px solid var(--rule); border-radius:5px; background:var(--panel); color:var(--fg); padding:8px 10px; }
#help-layout { display:flex; flex:1; min-height:0; }
#help-nav { width:168px; flex:0 0 168px; padding:9px; border-right:1px solid var(--rule); background:var(--panel); }
#help-nav button { width:100%; text-align:left; border:0; border-radius:4px; padding:9px 10px; background:transparent; color:var(--dim); }
#help-nav button.on { background:var(--selected); color:var(--fg); }
#help-content { flex:1; min-width:0; padding:7px 22px 20px; overflow:auto; font-size:14px; line-height:1.7; user-select:text; }
#help-content h3 { margin:15px 0 8px; font-size:18px; }
#help-content p { margin:8px 0 15px; }
#help-content kbd { font:12px var(--code-font); border:1px solid var(--rule); background:var(--panel); padding:2px 5px; border-radius:4px; white-space:nowrap; }
#help-content .shortcut { display:flex; align-items:center; justify-content:space-between; gap:12px; padding:7px 0; border-bottom:1px solid var(--rule); }
#help-foot { display:flex; align-items:center; gap:14px; padding:12px 18px; border-top:1px solid var(--rule); background:var(--panel); }
#help-foot a { color:var(--link); text-decoration:none; }
#help-foot a:hover { text-decoration:underline; }
#help-foot span { margin-left:auto; color:var(--dim); font-size:12px; }
#help-foot button { color:var(--fg); border:1px solid var(--rule); background:var(--bg); padding:5px 15px; border-radius:4px; }
#resize-edges { position:fixed; inset:0; pointer-events:none; z-index:80; }
#resize-edges i { position:absolute; pointer-events:auto; }
#resize-edges [data-direction=n],#resize-edges [data-direction=s] { left:8px; right:8px; height:4px; cursor:ns-resize; }
#resize-edges [data-direction=n] { top:0; } #resize-edges [data-direction=s] { bottom:0; }
#resize-edges [data-direction=e],#resize-edges [data-direction=w] { top:8px; bottom:8px; width:4px; cursor:ew-resize; }
#resize-edges [data-direction=e] { right:0; } #resize-edges [data-direction=w] { left:0; }
#resize-edges [data-direction=nw],#resize-edges [data-direction=se] { width:9px; height:9px; cursor:nwse-resize; }
#resize-edges [data-direction=ne],#resize-edges [data-direction=sw] { width:9px; height:9px; cursor:nesw-resize; }
#resize-edges [data-direction=nw] { top:0; left:0; } #resize-edges [data-direction=ne] { top:0; right:0; }
#resize-edges [data-direction=sw] { bottom:0; left:0; } #resize-edges [data-direction=se] { bottom:0; right:0; }
body.maximized #resize-edges { display:none; }
@media (max-width:900px) { #bar #b-theme { display:none!important; } #brand span { display:inline!important; } #bar #b-opts .button-label,#bar #b-find .button-label { display:none; } }
@media (max-width:720px) { #brand #app-name { font-size:12px; } #bar #b-save .button-label { display:none; } #bar .sp { display:none; } #bar #b-side { padding:5px; } #minimap { width:72px; flex-basis:72px; } #pfoot .grow { display:none; } #pfoot #footer-brand { margin-right:auto; } #help-nav { width:130px; flex-basis:130px; } #help-head { padding:12px 16px; } }
</style></head><body>

<div id="hot"></div>
<header id="bar" aria-label="Toolbar">
  <button id="b-menu" class="icon-only" aria-label="Main menu" aria-haspopup="menu" title="Main menu (Alt+F)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button>
  <button id="brand" class="brand-help" title="Help &amp; about BS Notepad" aria-label="BS Notepad help"><svg viewBox="0 0 64 64" aria-hidden="true"><rect x="4" y="4" width="56" height="56" rx="9" fill="#223d58"/><path d="M13 10h32v44H13z" fill="#ddecf4"/><path d="M13 10h6v44h-6z" fill="#479bcd"/><path d="M24 21h15m-15 8h15m-15 8h15m-15 8h12" stroke="#738fa0" stroke-width="2"/><path d="m33 48 16-27 5 3-16 27-7 4z" fill="#f4b74d" stroke="#1c2c3e" stroke-width="2"/></svg><span id="app-name"></span></button>
  <button id="b-side" aria-label="Files" aria-controls="side" aria-pressed="false" title="Show or hide the file tree (Ctrl+B)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18M5.5 7h1M5.5 11h1M5.5 15h1"/></svg><span class="button-label">Files</span></button>
  <span class="sp"></span>
  <div class="toolbar-group">
    <button id="b-new" aria-label="New note" title="New note (Ctrl+N)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M8 14h8M12 10v8"/></svg><span class="button-label">New</span></button>
    <button id="b-open" aria-label="Open file" title="Open file (Ctrl+O)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 8V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v2M3 9h18l-3 11H2z"/></svg><span class="button-label">Open</span></button>
    <button id="b-save" aria-label="Save" title="Save (Ctrl+S)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 3h13l4 4v14H3V3zM7 3v6h10V3M7 21v-8h10v8"/></svg><span class="button-label">Save</span></button>
    <button id="b-saveas" aria-label="Save as" title="Save a copy (Ctrl+Shift+S)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 3h12l3 3v4M7 3v6h8M3 3v18h8M14 18l6-6 3 3-6 6-4 1z"/></svg><span class="button-label">Save as</span></button>
  </div>
  <span class="sp"></span>
  <button id="b-find" aria-label="Find" title="Find (Ctrl+F)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg><span class="button-label">Find</span></button>
  <div id="drag-region" title="Drag to move · Double-click to maximize"></div>
  <div class="toolbar-end">
    <div class="toolbar-group" id="zoom-group">
      <button id="b-zoomout" class="icon-only" aria-label="Zoom out" title="Zoom out (Ctrl+-)">−</button>
      <button id="b-zoomreset" title="Reset zoom (Ctrl+0)">100%</button>
      <button id="b-zoomin" class="icon-only" aria-label="Zoom in" title="Zoom in (Ctrl+=)">+</button>
    </div>
    <button id="b-map" class="icon-only" aria-label="Document map" aria-pressed="false" title="Show or hide document map"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M16 3v18M18 6h1M18 9h1M18 12h1M18 15h1"/></svg></button>
    <button id="b-theme" class="select-control" aria-label="Theme" aria-haspopup="listbox" aria-expanded="false" title="Theme"></button>
    <button id="b-opts" aria-label="Options" title="Options (Ctrl+,)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16M8 3v6M16 9v6M10 15v6"/></svg><span class="button-label">Options</span></button>
    <button id="b-pin" class="icon-only" aria-label="Toggle toolbar reveal" title="Keep this bar visible"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 3 6 0-1 6 4 4v2H6v-2l4-4zM12 15v7"/></svg></button>
  </div>
  <div id="window-controls">
    <button id="window-min" aria-label="Minimize" title="Minimize">−</button>
    <button id="window-max" aria-label="Maximize" title="Maximize">□</button>
    <button id="window-close" aria-label="Close window" title="Close window">×</button>
  </div>
  <span id="note" role="status" aria-live="polite"></span>
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
    <div id="explorer-head"><strong>Explorer</strong>
      <button id="folder-new" class="explorer-action" aria-label="New note" title="New note"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8zM14 2v6h6M8 14h8M12 10v8"/></svg></button>
      <button id="folder-open" class="explorer-action" aria-label="Open folder" title="Open folder (Ctrl+Shift+O)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 8V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v2M3 9h18l-3 11H2z"/></svg></button>
      <button id="folder-refresh" class="explorer-action" aria-label="Refresh folder" title="Refresh folder"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 7v5h-5M4 17v-5h5M5 8a8 8 0 0 1 13-4l2 3M4 17l2 3a8 8 0 0 0 13-4"/></svg></button>
      <button id="sidepin" class="explorer-action" aria-label="Pin sidebar" title="Keep Explorer visible"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 3 6 0-1 6 4 4v2H6v-2l4-4zM12 15v7"/></svg></button>
    </div>
    <div id="tabs" aria-label="Explorer view">
      <button data-pane="files" class="on"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18M5.5 7h1M5.5 11h1M5.5 15h1"/></svg>Files</button>
      <button data-pane="outline"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5h12M8 12h12M8 19h12M3 5h1M3 12h1M3 19h1"/></svg>Outline</button>
      <button data-pane="recent"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 11a9 9 0 1 1 2 7M3 4v7h7M12 7v5l4 2"/></svg>Recent</button>
    </div>
    <div id="file-tools"><div id="filter-wrap"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg><input id="tree-filter" aria-label="Filter loaded files" placeholder="Filter files…" title="Filter files in the folders you have expanded"></div></div>
    <div id="folder-tools"><span class="root-chevron" aria-hidden="true">⌄</span><span id="root-icon" aria-hidden="true"></span><span id="folder-path"></span>
      <button id="folder-up" class="explorer-action" title="Parent folder" aria-label="Parent folder"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 7-7 7 7M12 5v15"/></svg></button>
      <button id="folder-collapse" class="explorer-action" title="Collapse all folders" aria-label="Collapse all folders"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 3h12v12M4 7h12v14H4zM7 14h6"/></svg></button>
    </div>
    <div id="pane-files" class="pane on"></div>
    <div id="pane-outline" class="pane"></div>
    <div id="pane-recent" class="pane"></div>
  </nav>
  <div id="main">
    <div id="document-head">
      <div id="document-tabs" role="tablist" aria-label="Open documents"></div>
      <button id="tab-new" aria-label="New tab" title="New tab (Ctrl+T)">+</button>
      <div id="document-actions">
        <button id="b-view" title="Rendered or source text (Ctrl+U)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18"/></svg><span class="button-label">Source</span></button>
        <button id="b-edit" title="Edit the source (Ctrl+E)"><svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m4 16 12-12 4 4L8 20l-5 1zM14 6l4 4"/></svg><span class="button-label">Edit</span></button>
      </div>
    </div>
    <div id="document-path" title="Document location"></div>
    <div id="content-row" role="tabpanel"><div id="content-main">
    <div id="fm"></div>
    <div id="doc"><article id="article"></article></div>
    <div id="editor"><textarea id="text" aria-label="Document editor" spellcheck="false"></textarea></div>
    </div><aside id="minimap" aria-label="Document map"><canvas id="map-canvas" aria-hidden="true"></canvas><div id="map-viewport" role="scrollbar" tabindex="0" aria-label="Document map position" aria-orientation="vertical" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0" aria-controls="doc"></div></aside></div>
  </div>
</div>

<div id="options"><div id="panel" role="dialog" aria-modal="true" aria-labelledby="options-title">
  <div id="phead"><h2 id="options-title">Options</h2><input id="search" aria-label="Search settings" placeholder="Search settings"><button id="opt-x" aria-label="Close options">×</button></div>
  <div id="pbody"><div id="rail"></div><div id="sets"></div></div>
  <div id="pfoot">
    <button id="footer-brand" class="brand-help" title="Help &amp; about BS Notepad">BS Notepad</button>
    <span class="grow" id="pnote">Changes save automatically.</span>
    <button id="opt-reset">Reset all</button>
    <button class="pri" id="opt-close">Done</button>
  </div>
</div></div>

<div id="help-overlay"><section id="help-panel" role="dialog" aria-modal="true" aria-labelledby="help-title">
  <header id="help-head"><img id="help-logo" alt="BS" width="84" height="84"><div><h2 id="help-title">BS Notepad</h2><p id="help-version"></p></div><button id="help-close" aria-label="Close help">×</button></header>
  <div id="help-search-row"><input id="help-search" aria-label="Search help" placeholder="Find a command, shortcut or setting…"></div>
  <div id="help-layout"><nav id="help-nav" aria-label="Help topics"></nav><div id="help-content" tabindex="0"></div></div>
  <footer id="help-foot"><a id="help-github" href="#">GitHub · IronWolve ↗</a><span>F1 opens Help</span><button id="help-done">Done</button></footer>
</section></div>
<div id="menu-popup" class="popup" role="menu" hidden></div>
<div id="choice-popup" class="popup" hidden><input id="choice-search" aria-label="Filter choices" placeholder="Filter choices…"><div id="choice-list" role="listbox"></div></div>
<div id="resize-edges" aria-hidden="true"><i data-direction="n"></i><i data-direction="s"></i><i data-direction="e"></i><i data-direction="w"></i><i data-direction="nw"></i><i data-direction="ne"></i><i data-direction="sw"></i><i data-direction="se"></i></div>
<script>
const $ = id => document.getElementById(id);
const send = o => {
 const editor=$("text"), doc=$("doc");
 window.ipc.postMessage(JSON.stringify({...o,fromTab:state.activeTab,view:{editing:state.editing,scroll:doc.scrollTop/(doc.scrollHeight||1),editorScroll:editor?.scrollTop||0,selectionStart:editor?.selectionStart||0,selectionEnd:editor?.selectionEnd||0}}));
};
const ICONS = {"edit": "<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m4 16 12-12 4 4L8 20l-5 1zM14 6l4 4\"/></svg>", "preview": "<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/></svg>", "source": "<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18\"/></svg>", "chevron": "<svg class=\"ui-icon\" viewBox=\"0 0 24 24\" aria-hidden=\"true\"><path d=\"m9 5 7 7-7 7\"/></svg>"};
function setCommand(id, icon, label) { $(id).innerHTML = ICONS[icon] + '<span class="button-label">' + label + '</span>'; }
function fileAppearance(name, directory = false, expanded = false) {
 const lower = name.toLowerCase();
 if (directory) {
   const special = /^(assets?|images?|media)$/.test(lower) ? ['gold','image'] :
     /^(css|styles?|scss)$/.test(lower) ? ['blue','style'] :
     /^(docs?|notes?)$/.test(lower) ? ['blue','note'] :
     /^(src|source|lib)$/.test(lower) ? ['blue','code'] :
     /^(tests?|specs?)$/.test(lower) ? ['green','test'] :
     lower.startsWith('.') ? ['muted',''] : ['blue',''];
   return {color:special[0],kind:expanded?'folder-open':'folder',badge:special[1],type:'Folder'};
 }
 if (/^(dockerfile|containerfile)(\.|$)/.test(lower)) return {color:'blue',kind:'box',type:'Container build'};
 if (/^(makefile|cmakelists\.txt|build\.gradle)$/.test(lower)) return {color:'gold',kind:'gear',type:'Build file'};
 if (/^(\.env|\.gitignore|\.gitattributes|\.editorconfig|\.npmrc)/.test(lower)) return {color:'muted',kind:'gear',type:'Configuration'};
 if (lower.endsWith('.lock') || lower.endsWith('-lock.json')) return {color:'gold',kind:'lock',type:'Dependency lock'};
 const ext = lower.includes('.') ? lower.split('.').pop() : '';
 const groups = [
  ['md markdown mdown mkd mkdn mdx rst org','blue','markdown','Markdown / notes'],
  ['txt text log','muted','note','Text'], ['py pyw pyi ipynb','gold','python','Python'],
  ['rs','orange','gear','Rust'], ['js mjs cjs','gold','JS','JavaScript'],
  ['ts mts cts','blue','TS','TypeScript'], ['jsx tsx','cyan','atom','Component'],
  ['html htm xml','orange','code','Markup'], ['css','blue','style','Stylesheet'],
  ['scss sass less','pink','style','Stylesheet'], ['json jsonc json5','gold','braces','JSON'],
  ['toml yaml yml ini conf cfg properties','purple','gear','Configuration'],
  ['sh bash zsh fish','green','terminal','Shell script'], ['ps1 bat cmd','blue','terminal','Command script'],
  ['go','cyan','Go','Go'], ['java jar kt kts','orange','cup','Java / Kotlin'],
  ['c h cpp hpp cc cxx','blue','C','C / C++'], ['cs fs fsx','purple','C#','Managed source'],
  ['rb','pink','diamond','Ruby'], ['php','purple','php','PHP'], ['swift','orange','code','Swift'],
  ['sql db sqlite sqlite3','gold','database','Database'], ['csv tsv xls xlsx','green','table','Table'],
  ['png jpg jpeg webp gif bmp ico avif svg','teal','image','Image'],
  ['mp3 wav flac ogg m4a aac','purple','music','Audio'], ['mp4 mov webm mkv avi','pink','video','Video'],
  ['zip tar gz bz2 xz 7z rar','gold','box','Archive'], ['pdf','pink','PDF','PDF'],
  ['exe dll so dylib bin','muted','box','Binary'], ['woff woff2 ttf otf','orange','Aa','Font']
 ];
 for (const [extensions,color,kind,type] of groups) if (extensions.split(' ').includes(ext)) return {color,kind,type};
 return {color:'blue',kind:'file',type:'File'};
}
const FILE_SHAPES = {
 file:'<path d="M3.5 1.5h6l3 3v10h-9zM9.5 1.5v3h3"/>',
 note:'<path d="M3.5 1.5h6l3 3v10h-9zM9.5 1.5v3h3M6 8h4M6 11h4"/>',
 markdown:'<path d="M1 11V5l3 3 3-3v6M11 5v6m-2-2 2 2 2-2" stroke-width="1.7"/>',
 braces:'<path d="M6 2H4v4L2 8l2 2v4h2M10 2h2v4l2 2-2 2v4h-2"/>',
 code:'<path d="m5 4-4 4 4 4m6-8 4 4-4 4M9 2 7 14"/>',
 style:'<path d="M4 11c1 3 6 2 6 0s-6-1-6-4 6-5 8-3-6 6-9 4M8 10l-2 4" stroke-width="1.7"/>',
 image:'<path d="M3 1.5h7l3 3v10H3zM10 1.5v3h3M4.5 12l2.5-3 2 2 1.5-1.5 1.5 2.5"/><circle cx="6" cy="6" r="1" fill="currentColor" stroke="none"/>',
 gear:'<path d="m6 1-.5 2-2 .5L2 5l1 2-1 2 1.5 2 2 .5.5 2h3l.5-2 2-.5L13 9l-1-2 1-2-1.5-1.5-2-.5L9 1z"/><circle cx="7.5" cy="7" r="2"/>',
 lock:'<rect x="3" y="7" width="10" height="7" rx="1"/><path d="M5 7V4a3 3 0 0 1 6 0v3M8 10v2"/>',
 terminal:'<path d="m2 4 4 4-4 4M8 12h5" stroke-width="1.7"/>',
 python:'<path d="M8 1H5v6h6V1zM5 5H2v6h6v4h4V9H5" fill="currentColor" stroke="none"/><circle cx="8" cy="3" r=".65" fill="var(--panel)"/>',
 atom:'<ellipse cx="8" cy="8" rx="7" ry="2.8"/><ellipse cx="8" cy="8" rx="7" ry="2.8" transform="rotate(60 8 8)"/><ellipse cx="8" cy="8" rx="7" ry="2.8" transform="rotate(120 8 8)"/><circle cx="8" cy="8" r="1" fill="currentColor"/>',
 diamond:'<path d="m1 5 3-3h8l3 3-7 9zM1 5h14M4 2l4 12 4-12"/>',
 cup:'<path d="M3 7h8v4a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3zM11 8h2a2 2 0 0 1 0 4h-2M5 1c-3 3 4 2 1 5M9 1c-3 3 4 2 1 5"/>',
 database:'<ellipse cx="8" cy="3" rx="5.5" ry="2"/><path d="M2.5 3v10c0 2.5 11 2.5 11 0V3M2.5 8c0 2.5 11 2.5 11 0"/>',
 table:'<rect x="2" y="2" width="12" height="12" rx="1"/><path d="M2 6h12M2 10h12M6 6v8M10 6v8"/>',
 music:'<path d="M6 12V3l7-2v9M6 6l7-2"/><ellipse cx="3.8" cy="12" rx="2.2" ry="1.5" fill="currentColor"/><ellipse cx="10.8" cy="10" rx="2.2" ry="1.5" fill="currentColor"/>',
 video:'<rect x="1.5" y="3" width="9" height="10" rx="1"/><path d="m10.5 6 4-2v8l-4-2z"/>',
 box:'<path d="M2 4 8 1l6 3v8l-6 3-6-3zM2 4l6 3 6-3M8 7v8M5 2.5l6 3v4"/>',
 test:'<path d="M5 1h6M6 1v5l-4 6a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l-4-6V1M4 10h8"/>'
};
function fileIcon(name, directory = false, expanded = false) {
 const a=fileAppearance(name,directory,expanded), icon=document.createElement('span');
 icon.className='file-icon file-'+a.color; icon.title=a.type;icon.setAttribute('aria-hidden','true');
 let shape=FILE_SHAPES[a.kind];
 if(directory) {
   shape='<path d="M1 3h5l1.5 2H15v9H1z" fill="currentColor" stroke="none"/>';
   if(expanded) shape='<path d="M1 3h5l1.5 2H14v3H1z" fill="currentColor" opacity=".6" stroke="none"/><path d="M1 7h15l-3 7H1z" fill="currentColor" stroke="none"/>';
   if(a.badge) shape+='<g transform="translate(7.5 6) scale(.55)" style="color:var(--panel)" stroke-width="1.6">'+FILE_SHAPES[a.badge]+'</g>';
 } else if(!shape) {
   shape='<text x="8" y="12" fill="currentColor" stroke="none" text-anchor="middle" font-family="system-ui,sans-serif" font-weight="750" font-size="'+(a.kind.length>2?7:10)+'">'+a.kind+'</text>';
 }
 icon.innerHTML='<svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">'+shape+'</svg>';
 return icon;
}
function fillFileRow(el, name, directory, expanded = false) {
 const chevron=document.createElement('span'); chevron.className='chevron'; chevron.setAttribute('aria-hidden','true');
 if(directory) chevron.innerHTML=ICONS.chevron;
 const label=document.createElement('span');label.className='file-label';label.textContent=name;
 el.replaceChildren(chevron,fileIcon(name,directory,expanded),label);
}
function normalizedPath(path) {
 let value=path.replaceAll(String.fromCharCode(92),'/');
 if(value.startsWith('//?/UNC/')) value='//' + value.slice(8);
 else if(value.startsWith('//?/')) value=value.slice(4);
 return value;
}
function samePath(a,b) {
 const left=normalizedPath(a),right=normalizedPath(b);
 return /^(?:[a-z]:\/|\/\/)/i.test(left) ? left.toLowerCase()===right.toLowerCase() : left===right;
}

let requestId = 0, noteTimer, optionsFocus;
const pendingFolders = new Map();
let state = { activeTab:1, settings:{}, defaults:{}, themes:[], fonts:[], path:"", dirty:false, editing:false };
// Grouped so each screen is short. Remembered state - window size, last file,
// scroll position - is not a setting and is deliberately not listed.
const GROUPS = {
  Appearance: ["theme", "chrome", "zoom"],
  Workspace: ["sidebar", "sidebar_width", "sidebar_tab", "show_hidden", "restore_last_file", "close_to_tray"],
  Editor: ["word_wrap", "tab_size", "minimap"],
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
 minimap:["Document map","A small scrollable overview beside the document. Toggle it from the toolbar."],
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
    state.name = s.name; state.version = s.version; state.logoUrl = s.logoUrl; state.githubUrl = s.githubUrl; state.trayAvailable = s.trayAvailable;
    document.title = s.name; $("app-name").textContent = s.name;
    $("options-title").textContent = "Options";
    $("footer-brand").textContent = s.name + " · " + s.version;
    $("pnote").textContent = "Changes save automatically.";
    state.settings = s.settings; state.defaults = s.defaults;
    state.themes = s.themes; state.fonts = s.fonts;
    app.setThemeControl();
    app.applyTheme(s.theme);
    app.applySettings(s.settings);
    app.windowState(!!s.maximized);
    if (s.missingFonts && s.missingFonts.length)
      app.note("font not on this machine: " + s.missingFonts.join(", "));
    if ($("options").classList.contains("show")) app.drawOptions();
  },
  applyTheme(t) {
    const r = document.documentElement.style;
    document.body.classList.toggle("light-icons", !t.dark);
    r.setProperty("color-scheme",t.dark ? "dark" : "light");
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
    const filesDocked = s.sidebar === "always" && s.sidebar_tab === "files";
    $("b-side").classList.toggle("on", filesDocked);
    $("b-side").setAttribute("aria-pressed", String(filesDocked));
    $("b-side").title = (filesDocked ? "Hide" : "Show") + " the file tree (Ctrl+B)";
    app.setThemeControl();
    $("grip").setAttribute("aria-valuenow", s.sidebar_width);
    $("grip").setAttribute("aria-valuemin", 180);
    $("grip").setAttribute("aria-valuemax", 640);
    document.body.classList.toggle("auto", s.chrome === "auto");
    setCommand("b-view", s.view_mode === "source" ? "preview" : "source", s.view_mode === "source" ? "Rendered" : "Source");
    $("b-zoomreset").textContent = Math.round(s.zoom * 100) + "%";
    $("b-pin").classList.toggle("on", s.chrome === "always");
    $("b-view").classList.toggle("on", s.view_mode === "source");
    for (const b of document.querySelectorAll("#tabs button[data-pane]"))
      b.classList.toggle("on", b.dataset.pane === s.sidebar_tab);
    for (const p of ["files","outline","recent"])
      $("pane-" + p).classList.toggle("on", p === s.sidebar_tab);
    for (const id of ["folder-tools","file-tools"]) $(id).style.display = s.sidebar_tab === "files" ? "" : "none";
    if ($("options").classList.contains("show")) app.drawOptions();
  },
  setDocument(d) {
    $("article").innerHTML = d.html;
    state.path = d.path || "";
    app.breadcrumbs();
    app.note(d.note || "");
    $("fm").textContent = d.frontMatter || "";
    $("fm").classList.toggle("show", !!d.frontMatter);
    app.outline(d.outline || []);
    app.addCopyButtons();
    app.wireLinks();
    $("doc").scrollTop = (d.scroll || 0) * $("doc").scrollHeight;
    app.setDirty(!!d.dirty);
    for (const row of document.querySelectorAll("#pane-files [data-path], #pane-recent [data-path]")) { const current=samePath(row.dataset.path,state.path); row.classList.toggle("current",current); row.setAttribute("aria-current",String(current)); }
    if ($("find").classList.contains("show")) runFind($("find-text").value);
  },
  setEditorText(t) { if ($("text").value !== t) $("text").value = t; },
  note(t) { clearTimeout(noteTimer); $("note").textContent = t || ""; if (t) noteTimer = setTimeout(() => $("note").textContent = "", 9000); },
  setDirty(d) {
    state.dirty = d;
    $("b-save").classList.toggle("on", d);
    $("b-save").setAttribute("aria-label", d ? "Save unsaved changes" : "Save");
    app.updateTabDirty?.(d);
  },
  breadcrumbs() {
    const path = normalizedPath(state.path || "");
    const root = normalizedPath(state.workspace || "").replace(/\/$/,'');
    const relative = root && path.startsWith(root + '/') ? path.slice(root.length+1) : path;
    const parts = relative ? relative.split('/').filter(Boolean) : ["Unsaved note"];
    if(root && path.startsWith(root + '/')) parts.unshift(root.split('/').pop() || root);
    const bar=$("document-path"); bar.replaceChildren();bar.title=state.path || "Save this note to choose a location";
    for(const [i,part] of parts.entries()) {
      if(i) { const sep=document.createElement('span');sep.className='crumb-separator';sep.textContent='›';sep.setAttribute('aria-hidden','true');bar.appendChild(sep); }
      const text=document.createElement('span');text.className='crumb';text.textContent=part;bar.appendChild(text);
    }
  },
  outline(list) {
    const pane = $("pane-outline"); pane.innerHTML = "";
    if (!list.length) { pane.innerHTML = '<div class="item">No headings</div>'; return; }
    for (const h of list) {
      const el = document.createElement("button");
      el.className = "item out-" + h.level;
      const symbol=document.createElement("span");symbol.className="outline-symbol";symbol.textContent="#";symbol.setAttribute("aria-hidden","true");
      const label=document.createElement("span");label.className="file-label";label.textContent=h.text;el.append(symbol,label);
      el.onclick = () => {
        const target = document.getElementById(h.anchor);
        if (target) target.scrollIntoView({ block:"start" });
      };
      pane.appendChild(el);
    }
  },
  setTree(d) {
    pendingFolders.clear();
    state.workspace = d.dir;
    $("root-icon").replaceChildren(fileIcon(d.dir.split(/[\\/]/).pop() || "Folder",true,true));
    $("folder-path").textContent = d.dir.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || d.dir;
    app.breadcrumbs();
    $("folder-path").title = d.dir;
    $("folder-up").disabled = !d.parent;
    $("pane-files").replaceChildren(app.entries(d.entries || []));
    app.filterTree();
  },
  entries(list) {
    const box = document.createElement("div");
    if (!list.length) { const empty = document.createElement("div"); empty.className="tree-message"; empty.innerHTML="<strong>No files here</strong>Open another folder or create a new note."; box.appendChild(empty); }
    for (const e of list) {
      const branch = document.createElement("div"); branch.className="branch";
      const el = document.createElement("button");
      el.className = "item" + (e.dir ? " dir" : "") + (samePath(e.path,state.path) ? " current" : "");
      el.dataset.path = e.path; el.dataset.name = e.name; el.title = e.path;
      fillFileRow(el,e.name,e.dir);
      el.setAttribute("aria-label",e.name);
      el.setAttribute("aria-current",String(samePath(e.path,state.path)));
      el.title = fileAppearance(e.name,e.dir).type + " · " + e.path;
      if (e.dir) {
        el.setAttribute("aria-expanded", "false");
        el.onclick = () => {
          const open = el.getAttribute("aria-expanded") === "true";
          el.setAttribute("aria-expanded", String(!open));
          fillFileRow(el,e.name,true,!open);
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
      el.className = "item"; el.title = p; el.dataset.path=p;
      const name=p.split(/[\\/]/).pop();
      const label=document.createElement("span");label.className="recent-label";
      const filename=document.createElement("span");filename.className="file-label";filename.textContent=name;
      const path=document.createElement("span");path.className="recent-path";path.textContent=p;
      label.append(filename,path);el.append(fileIcon(name),label);
      el.classList.toggle("current",samePath(p,state.path));
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
  toggleEdit(on, notify = true) {
    state.editing = on;
    $("editor").classList.toggle("show", on);
    $("doc").style.display = on ? "none" : "";
    $("b-edit").classList.toggle("on", on);
    setCommand("b-edit",on ? "preview" : "edit",on ? "Preview" : "Edit");
    $("fm").hidden = on;
    if (on) $("text").focus();
    if (notify) send({cmd:"viewState"});
    app.scheduleMap?.();
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
    const focusTheme = document.activeElement?.dataset.theme;
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
    if (focusTheme) box.querySelector(`[data-theme="${focusTheme}"]`)?.focus();
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
      row.classList.add("theme-setting");
      control.setAttribute("role", "group");
      for (const t of state.themes) {
        const b = document.createElement("button");
        b.style.background = t.bg;
        b.style.borderColor = t.rule;
        b.style.color = t.fg;
        b.textContent = t.name;
        b.dataset.theme = t.id;
        b.title = t.name; b.setAttribute("aria-label", t.name); b.setAttribute("aria-pressed", String(t.id === value));
        b.classList.toggle("on", t.id === value);
        b.onclick = () => send({ cmd:"setting", key:"theme", value:t.id });
        control.appendChild(b);
      }
    } else if (CHOICES[key]) {
      control = app.selectControl(CHOICES[key].map(choice => ({value:choice,label:choiceLabel(choice)})),value,
        next => send({cmd:"setting",key,value:next}), LABELS[key]?.[0] || key);
    } else if (key.endsWith("_font")) {
      const list = state.fonts.filter(f => key !== "code_font" || f.monospace);
      const choices = [{value,label:value},...list.filter(f=>f.name!==value).map(f=>({value:f.name,label:f.name + (f.nerd ? " (patched)" : "")}))];
      control = app.selectControl(choices,value,next=>send({cmd:"setting",key,value:next}),LABELS[key]?.[0] || key);
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
    if (!["DIV","BUTTON"].includes(control.tagName)) {
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
$("folder-new").onclick = () => $("b-new").click();
$("folder-open").onclick = () => send({cmd:"openFolder"});
$("folder-refresh").onclick = () => send({cmd:"refreshTree"});
$("folder-up").onclick = () => send({cmd:"treeUp"});
$("tree-filter").oninput = () => app.filterTree();
$("folder-collapse").onclick = () => {
  pendingFolders.clear();
  for(const row of $("pane-files").querySelectorAll('.dir[aria-expanded="true"]')) { row.setAttribute('aria-expanded','false');fillFileRow(row,row.dataset.name,true); }
  for(const group of [...$("pane-files").querySelectorAll('.kids')]) group.remove();
  app.filterTree();
};
$("pane-files").onkeydown = e => {
  const row=e.target.closest('.item'); if(!row) return;
  const rows=[...$("pane-files").querySelectorAll('.item')].filter(el=>el.getClientRects().length);
  const index=rows.indexOf(row); let target;
  if(e.key==='ArrowDown') target=rows[Math.min(index+1,rows.length-1)];
  else if(e.key==='ArrowUp') target=rows[Math.max(0,index-1)];
  else if(e.key==='Home') target=rows[0];
  else if(e.key==='End') target=rows.at(-1);
  else if(e.key==='ArrowRight' && row.classList.contains('dir')) {
    if(row.getAttribute('aria-expanded')==='false') row.click();
    else target=row.parentElement.querySelector('.kids .item');
  } else if(e.key==='ArrowLeft') {
    if(row.getAttribute('aria-expanded')==='true') row.click();
    else target=row.closest('.kids')?.parentElement.querySelector(':scope > .item');
  } else return;
  e.preventDefault(); if(target) { target.focus();target.scrollIntoView({block:'nearest'}); }
};
$("b-view").onclick = () => send({ cmd:"setting", key:"view_mode",
  value: state.settings.view_mode === "source" ? "rendered" : "source" });
$("b-open").onclick = () => send({ cmd:"open" });
$("b-save").onclick = () => send({ cmd:"save", text:$("text").value });
$("b-edit").onclick = () => { app.toggleEdit(!state.editing); if (!state.editing) send({cmd:"preview"}); };
$("b-side").onclick = () => {
  const hide = state.settings.sidebar === "always" && state.settings.sidebar_tab === "files";
  if (!hide) send({ cmd:"setting", key:"sidebar_tab", value:"files" });
  send({ cmd:"setting", key:"sidebar", value:hide ? "off" : "always" });
};
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
$("b-theme").onclick = () => app.chooseTheme();
$("b-opts").onclick = () => app.options(true);
$("opt-x").onclick = () => app.options(false);
$("options").onclick = e => { if (e.target === $("options")) app.options(false); };
$("opt-close").onclick = () => app.options(false);
$("opt-reset").onclick = () => { if ($("opt-reset").dataset.confirm) { send({cmd:"resetSettings"}); $("opt-reset").textContent="Reset all"; delete $("opt-reset").dataset.confirm; } else { $("opt-reset").dataset.confirm="1"; $("opt-reset").textContent="Confirm reset"; } };
$("search").oninput = () => app.drawOptions();
$("b-zoomreset").onclick = () => send({cmd:"setting",key:"zoom",value:1});
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
/* WORKSPACE_UI */
send({ cmd:"ready" });
</script></body></html>"##;


pub fn shell() -> String {
    SHELL.replace("/* WORKSPACE_UI */", include_str!("workspace.js"))
}
