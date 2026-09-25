// Documents keep their own editor nodes so switching tabs preserves native undo.
const editorNodes = new Map(), pendingViews = new Map();
let mapTimer, mapFrame, mapHeight = 0, helpFocus, selectedHelp = 'start';
let menuOrigin, choiceOrigin, choiceOptions = [], choiceValue, choiceChange, choicePreview, choiceTheme=false;
let themeFamily = "all";
let themePreviewTimer, themePreviewToken=0, readerFrame, dialogFrame;
let zoomTarget=null, zoomTimer=null, wheelRemainder=0, lastZoomWheel=0;
const cleanPath = path => normalizedPath(path || '');
const fileName = path => cleanPath(path).split('/').pop() || 'Untitled';

const MENU_PATHS={
 note:'<path d="M14 3H5v18h14V8zM14 3v5h5M8 13h8M12 10v7"/>',
 open:'<path d="M3 8V5h6l3 3h8v3M3 9h18l-3 11H2z"/>',
 workspace:'<path d="M3 5h6l3 3h9v12H3zM8 8v12M5 12h1M5 15h1"/>',
 save:'<path d="M4 3h13l4 4v14H3V3zM7 3v6h10V3M7 21v-8h10v8"/>',
 saveAs:'<path d="M4 3h12l3 3v5M7 3v6h8M3 3v18h8M13 18l6-6 3 3-6 6-4 1z"/>',
 reload:'<path d="M20 6v6h-6M20 12a8 8 0 1 0-2 6"/>',
 close:'<rect x="3" y="4" width="18" height="16" rx="2"/><path d="m9 9 6 6m0-6-6 6"/>',
 closeOthers:'<path d="M8 4h13v12M3 8h13v13H3zM7 12l5 5m0-5-5 5"/>',
 edit:'<path d="m4 16 12-12 4 4L8 20l-5 1zM14 6l4 4"/>',
 read:'<path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
 source:'<path d="m8 6-6 6 6 6m8-12 6 6-6 6M14 3l-4 18"/>',
 rendered:'<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 7h8M8 11h8M8 15h5"/>',
 map:'<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M15 3v18M17 7h2M17 10h2M17 13h2M17 16h2"/>',
 wrap:'<path d="M3 6h18M3 11h14a4 4 0 0 1 0 8h-5m3-3-3 3 3 3M3 17h5"/>',
 options:'<path d="M4 6h16M4 12h16M4 18h16M8 3v6M16 9v6M10 15v6"/>',
 exit:'<path d="M10 4H4v16h6M10 12h11m-4-4 4 4-4 4"/>',
 newTab:'<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M8 12h8M12 8v8"/>',
 enter:'<path d="M9 4H4v16h5M12 7l5 5-5 5M7 12h14"/>',
 copy:'<rect x="8" y="8" width="12" height="13" rx="2"/><path d="M15 5V3H3v13h2"/>',
 filename:'<path d="M14 3H5v18h14V8zM14 3v5h5M8 13h8M8 17h5"/>',
 collapse:'<path d="M8 3h13v13M3 8h13v13H3zM6 14h7"/>',
 check:'<path d="m5 12 4 4L19 6"/>'
};
function menuIcon(name){return '<svg viewBox="0 0 24 24" aria-hidden="true">'+(MENU_PATHS[name]||MENU_PATHS.note)+'</svg>';}

function wireEditor(node) {
  node.oninput = () => {
    const revision = Number(node.dataset.revision || 0) + 1;
    node.dataset.revision = revision;
    app.setDirty(true);
    send({cmd:'edit',text:node.value,revision});
    app.scheduleMap();
  };
  node.onkeydown = e => {
    if(e.key === 'Tab' && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      e.preventDefault(); document.execCommand('insertText',false,' '.repeat(state.settings.tab_size || 4));
    }
  };
  node.onscroll = () => { app.mapPosition(); };
}
wireEditor($('text'));
editorNodes.set(1,$('text'));
$('text').dataset.revision='0';
function editorFor(id) {
  let node=editorNodes.get(id);
  if(!node) {
    node=document.createElement('textarea');node.spellcheck=false;node.setAttribute('aria-label','Document editor');
    node.dataset.revision='0';wireEditor(node);editorNodes.set(id,node);$('editor').appendChild(node);
  }
  for(const [key,editor] of editorNodes) { editor.id=key===id?'text':'editor-'+key;editor.hidden=key!==id; }
  node.wrap=state.settings.word_wrap?'soft':'off';node.style.tabSize=state.settings.tab_size||4;
  state.activeTab=id;
  return node;
}
app.documentChrome = () => {
  const single=state.tabs?.length===1,tab=single?state.tabs[0]:null;
  $('document-head').hidden=single;
  $('single-title').textContent=tab ? tab.name + (tab.dirty ? ' •' : '') : '';
  $('single-title').setAttribute('aria-label',tab ? tab.name + (tab.dirty ? ', unsaved changes' : '') : '');
  $('content-row').setAttribute('role',single?'region':'tabpanel');
  if(single)$('content-row').setAttribute('aria-labelledby','single-title');
};
app.setTabs = payload => {
  state.tabs=payload.tabs;
  const live=new Set(payload.tabs.map(tab=>tab.id));
  for(const [id,node] of editorNodes) if(!live.has(id)) {node.remove();editorNodes.delete(id);pendingViews.delete(id);}
  const scroller=$('document-tabs'), scroll=scroller.scrollLeft, focus=document.activeElement?.dataset.tabId;
  scroller.replaceChildren();
  for(const tab of payload.tabs) {
    const wrap=document.createElement('div');wrap.className='document-tab'+(tab.id===payload.active?' active':'');wrap.dataset.id=tab.id;
    const button=document.createElement('button');button.className='tab-label';button.dataset.tabId=tab.id;button.setAttribute('role','tab');
    button.id='tab-'+tab.id;button.setAttribute('aria-controls','content-row');button.setAttribute('aria-selected',String(tab.id===payload.active));button.tabIndex=tab.id===payload.active?0:-1;
    button.title=tab.path || tab.name;
    const label=document.createElement('span');label.className='tab-name';label.textContent=tab.name;
    const dirty=document.createElement('span');dirty.className='tab-dirty';dirty.textContent='●';dirty.hidden=!tab.dirty;dirty.setAttribute('aria-label','Unsaved changes');
    button.append(fileIcon(tab.name),label,dirty);
    button.onclick=()=>send({cmd:'activateTab',id:tab.id});
    button.onkeydown=e=>{
      if(['ArrowLeft','ArrowRight','Home','End'].includes(e.key)) {
        e.preventDefault();const index=payload.tabs.findIndex(t=>t.id===tab.id);
        const next=e.key==='Home'?0:e.key==='End'?payload.tabs.length-1:(index+(e.key==='ArrowRight'?1:-1)+payload.tabs.length)%payload.tabs.length;
        send({cmd:'activateTab',id:payload.tabs[next].id});
      }
    };
    const close=document.createElement('button');close.className='tab-close';close.dataset.tabId=tab.id;close.textContent='×';close.title='Close '+tab.name;close.setAttribute('aria-label','Close '+tab.name);
    close.onclick=()=>send({cmd:'closeTab',id:tab.id});
    wrap.onauxclick=e=>{if(e.button===1){e.preventDefault();send({cmd:'closeTab',id:tab.id});}};
    wrap.oncontextmenu=e=>{
      e.preventDefault();showMenu(button,[
        {label:'Close this tab',icon:'close',hint:'Ctrl+W',action:()=>send({cmd:'closeTab',id:tab.id})},
        {label:'Keep only this tab',icon:'closeOthers',disabled:payload.tabs.length<2,action:()=>send({cmd:'closeOtherTabs',id:tab.id})},
        null,{label:'Copy full path',icon:'copy',disabled:!tab.path,action:()=>copyText(cleanPath(tab.path))}
      ],e);
    };
    wrap.append(button,close);scroller.appendChild(wrap);
  }
  $('content-row').setAttribute('aria-labelledby','tab-'+payload.active);
  app.documentChrome();
  app.fitTitle?.();
  scroller.scrollLeft=scroll;
  if(focus){if(payload.tabs.length===1)$('b-menu').focus();else scroller.querySelector(`.tab-label[data-tab-id="${payload.active}"]`)?.focus();}
  if(payload.tabs.length>1&&payload.active!==state.activeTab)scroller.querySelector('.document-tab.active')?.scrollIntoView({block:'nearest',inline:'nearest'});
};
app.updateTabDirty = dirty => {
  const tab=state.tabs?.find(t=>t.id===state.activeTab);if(tab)tab.dirty=dirty;
  app.documentChrome();
  const marker=$('document-tabs').querySelector(`[data-id="${state.activeTab}"] .tab-dirty`);if(marker)marker.hidden=!dirty;
};
const baseDocument=app.setDocument;
app.setDocument = payload => {
  if(payload.themeId && payload.themeId !== (state.previewTheme || state.settings.theme)) return;
  const id=payload.tab ?? state.activeTab ?? 1;
  const changed=state.activeTab!==id || !editorNodes.has(id);
  editorFor(id);
  if(changed)pendingViews.set(id,payload);
  baseDocument(payload);
  for(const table of $('article').querySelectorAll('table')) {
    const wrap=document.createElement('div');wrap.className='table-scroll';table.before(wrap);wrap.appendChild(table);
  }
  for(const image of $('article').querySelectorAll('img')) image.addEventListener('load',()=>app.scheduleReaderLayout(),{once:true});
  app.toggleEdit(payload.editing ?? state.editing,false);
  app.scheduleReaderLayout?.();
  app.scheduleMap();
};
app.setEditorText = payload => {
  const data=typeof payload==='string'?{tab:state.activeTab,text:payload,revision:0}:payload;
  const node=editorNodes.get(data.tab);if(!node)return;
  if((data.revision||0)<Number(node.dataset.revision||0))return;
  if(node.value!==data.text)node.value=data.text;
  node.dataset.revision=data.revision||0;
  const view=pendingViews.get(data.tab);
  if(view) {
    node.setSelectionRange(view.selectionStart||0,view.selectionEnd||0);node.scrollTop=view.editorScroll||0;
    pendingViews.delete(data.tab);
  }
  app.scheduleMap();
};
$('tab-new').onclick=()=>$('b-new').click();

function popupPosition(popup,anchor,point) {
  popup.hidden=false;
  const rect=anchor.getBoundingClientRect(),width=popup.offsetWidth,height=popup.offsetHeight;
  const x=point?.clientX || rect.left,y=point?.clientY || rect.bottom+4;
  popup.style.left=Math.max(8,Math.min(x,innerWidth-width-8))+'px';
  popup.style.top=Math.max(8,Math.min(y,innerHeight-height-8))+'px';
}
function syncPopupState(){document.body.classList.toggle('popup-open',!$('menu-popup').hidden||!$('choice-popup').hidden);}
function closeMenu(focus=true) { $('menu-popup').hidden=true;syncPopupState();if(focus)menuOrigin?.focus(); }
function closeChoices(focus=true) {
  $('choice-popup').hidden=true;syncPopupState();choiceOrigin?.setAttribute('aria-expanded','false');if(focus)choiceOrigin?.focus();
}
function showMenu(anchor,items,point) {
  closeChoices(false);closeMenu(false);menuOrigin=anchor;
  const menu=$('menu-popup');menu.replaceChildren();
  for(const item of items) {
    if(!item){menu.appendChild(document.createElement('hr'));continue;}
    const button=document.createElement('button');button.setAttribute('role','menuitem');button.disabled=!!item.disabled;if(item.brand)button.classList.add('menu-brand');
    const icon=document.createElement('span');icon.className='menu-symbol';icon.setAttribute('aria-hidden','true');
    if(item.brand&&state.logoUrl){const image=document.createElement('img');image.src=state.logoUrl;image.alt='';icon.appendChild(image);}else icon.innerHTML=menuIcon(item.icon);
    const label=document.createElement('span');label.className='menu-label';label.textContent=item.label;
    const hint=document.createElement('span');hint.className='menu-hint';hint.textContent=item.hint||'';
    button.append(icon,label,hint);
    if(typeof item.checked==='boolean'){button.setAttribute('role','menuitemcheckbox');button.setAttribute('aria-checked',String(item.checked));const check=document.createElement('span');check.className='menu-check';check.setAttribute('aria-hidden','true');if(item.checked)check.innerHTML=menuIcon('check');button.appendChild(check);}
    button.onclick=()=>{closeMenu();item.action();};menu.appendChild(button);
  }
  popupPosition(menu,anchor,point);syncPopupState();menu.querySelector('button:not(:disabled)')?.focus();
}
async function copyText(text) {
  try { await navigator.clipboard.writeText(text); }
  catch {
    const focus=document.activeElement, temp=document.createElement('textarea');temp.value=text;temp.style.cssText='position:fixed;left:-9999px';document.body.appendChild(temp);temp.select();
    const ok=document.execCommand('copy');temp.remove();focus?.focus();if(!ok){app.note('Clipboard access is unavailable.');return;}
  }
  app.note('Copied');
}
function fileMenu(event,row) {
  event.preventDefault();const path=row.dataset.path,folder=row.classList.contains('dir');
  const entries=folder?[
    {label:row.getAttribute('aria-expanded')==='true'?'Fold this folder':'Expand this folder',icon:row.getAttribute('aria-expanded')==='true'?'collapse':'open',action:()=>row.click()},
    {label:'Browse from this folder',icon:'workspace',action:()=>send({cmd:'workspacePath',path})}
  ]:[
    {label:'Open here',icon:'enter',action:()=>send({cmd:'openPath',path})},
    {label:'Open in new tab',icon:'newTab',action:()=>send({cmd:'openPath',path,newTab:true})}
  ];
  entries.push(null,
    {label:'Copy full path',icon:'copy',action:()=>copyText(cleanPath(path))},
    {label:'Copy filename',icon:'filename',action:()=>copyText(fileName(path))},
    {label:folder?'Open folder in file manager':'Open containing folder',icon:'open',action:()=>send({cmd:'showFolder',path})},
    null,{label:'Refresh file list',icon:'reload',action:()=>send({cmd:'refreshTree'})});
  showMenu(row,entries,event);
}
for(const pane of [$('pane-files'),$('pane-recent')]) {
  pane.oncontextmenu=e=>{const row=e.target.closest('[data-path]');if(row)fileMenu(e,row);};
  pane.addEventListener('keydown',e=>{if(e.key==='F10'&&e.shiftKey){const row=e.target.closest('[data-path]');if(row)fileMenu(e,row);}});
  pane.onauxclick=e=>{const row=e.target.closest('[data-path]');if(e.button===1&&row&&!row.classList.contains('dir')){e.preventDefault();send({cmd:'openPath',path:row.dataset.path,newTab:true});}};
}
function labelChoice(button,label) {
  const text=document.createElement('span');text.className='selected-label';text.textContent=label;
  const arrow=document.createElement('span');arrow.className='select-arrow';arrow.textContent='▾';arrow.setAttribute('aria-hidden','true');button.replaceChildren(text,arrow);
}
app.selectControl = (options,value,change,label) => {
  const button=document.createElement('button');button.className='select-control';button.type='button';button.setAttribute('role','combobox');
  button.setAttribute('aria-label',label);button.setAttribute('aria-haspopup','listbox');button.setAttribute('aria-expanded','false');button.setAttribute('aria-controls','choice-list');
  labelChoice(button,options.find(o=>o.value===value)?.label||String(value));
  button.onclick=()=>openChoices(button,options,value,change);return button;
};
const softThemeIds = new Set(['rose-stone','warm-clay','cocoa','parchment','jade','lagoon','porcelain','lavender-clay','blossom','pewter','ink','marble']);
app.themeFamily = id => id.startsWith('bold-') ? 'bold' : softThemeIds.has(id) ? 'soft' : 'classic';
app.themeFilters = changed => {
  const bar=document.createElement('div');bar.className='theme-filters';bar.setAttribute('role','group');bar.setAttribute('aria-label','Theme collection');
  for(const [value,label] of [['all','All'],['bold','Bold'],['soft','Soft'],['classic','Classic']]) {
    const button=document.createElement('button');button.type='button';button.textContent=label;button.dataset.family=value;button.setAttribute('aria-pressed',String(themeFamily===value));
    button.onclick=()=>{themeFamily=value;for(const b of bar.children)b.setAttribute('aria-pressed',String(b.dataset.family===value));changed();};
    bar.appendChild(button);
  }
  return bar;
};
function layoutChoices() {
  const popup=$('choice-popup');if(popup.hidden||!choiceOrigin)return;
  const rect=choiceOrigin.getBoundingClientRect(),down=innerHeight-rect.bottom-14,up=rect.top-14;
  const above=down<180&&up>down,available=Math.max(80,above?up:down);
  popup.style.maxHeight=available+'px';
  $('choice-list').style.maxHeight=Math.max(40,available-12-($('choice-search').hidden?0:$('choice-search').offsetHeight+5)-($('choice-hint').hidden?0:$('choice-hint').offsetHeight+4)-($('choice-families').hidden?0:$('choice-families').offsetHeight+4))+'px';
  popup.style.left=Math.max(8,Math.min(rect.left,innerWidth-popup.offsetWidth-8))+'px';
  popup.style.top=Math.max(8,above?rect.top-popup.offsetHeight-6:rect.bottom+6)+'px';
}
function markChoice(value) {
  choiceValue=value;
  for(const button of $('choice-list').querySelectorAll('button'))button.setAttribute('aria-selected',String(button.dataset.value===String(value)));
}
function drawChoices() {
  const query=$('choice-search').value.toLowerCase(),list=$('choice-list');list.replaceChildren();
  const choices=choiceOptions.filter(o=>o.label.toLowerCase().includes(query)&&(!choiceTheme||query||themeFamily==='all'||app.themeFamily(o.value)===themeFamily));
  for(const option of choices) {
    const button=document.createElement('button');button.dataset.value=option.value;button.setAttribute('role','option');button.setAttribute('aria-selected',String(option.value===choiceValue));
    if(choiceTheme){
      const theme=state.themes.find(t=>t.id===option.value),swatch=document.createElement('span');swatch.className='theme-swatch';swatch.setAttribute('aria-hidden','true');swatch.textContent='Aa';
      if(theme){swatch.style.background=theme.bg;swatch.style.color=theme.fg;swatch.style.borderColor=theme.rule;}button.appendChild(swatch);
    }
    const label=document.createElement('span');label.className='choice-label';label.textContent=option.label;button.appendChild(label);
    if(choiceTheme){const saved=document.createElement('span');saved.className='choice-saved';saved.setAttribute('aria-hidden','true');if(option.value===state.settings.theme)saved.innerHTML=menuIcon('check');button.appendChild(saved);}
    if(choicePreview){const preview=()=>{markChoice(option.value);choicePreview?.(option.value);};button.onmouseenter=preview;button.onfocus=preview;}
    button.onclick=()=>{const callback=choiceChange;closeChoices();callback(option.value);};list.appendChild(button);
  }
  if(!choices.length){const empty=document.createElement('div');empty.className='choice-empty';empty.textContent='No matching choices';list.appendChild(empty);}
  requestAnimationFrame(layoutChoices);
}
function openChoices(anchor,options,value,change,preview=null) {
  closeMenu(false);closeChoices(false);choiceOrigin=anchor;choiceOptions=options;choiceValue=value;choiceChange=change;choicePreview=preview;choiceTheme=!!preview;
  $('choice-families').hidden=!choiceTheme;$('choice-families').replaceChildren();if(choiceTheme)$('choice-families').appendChild(app.themeFilters(drawChoices));
  anchor.setAttribute('aria-expanded','true');$('choice-search').value='';$('choice-search').hidden=options.length<(choiceTheme?19:9);$('choice-hint').hidden=!choiceTheme;
  const popup=$('choice-popup');popup.classList.toggle('theme-grid',choiceTheme);popup.style.width=Math.max(choiceTheme?440:220,Math.min(400,anchor.getBoundingClientRect().width))+'px';popup.hidden=false;
  drawChoices();layoutChoices();syncPopupState();
  if(!$('choice-search').hidden)$('choice-search').focus();
  else (popup.querySelector('[aria-selected=true]') || popup.querySelector('button'))?.focus();
  popup.querySelector('[aria-selected=true]')?.scrollIntoView({block:'nearest'});
}
$('choice-search').oninput=drawChoices;
app.setThemeControl=()=>{
  const current=state.previewTheme||state.settings.theme,name=state.themes.find(t=>t.id===current)?.name||'Theme';
  const arrow=document.createElement('span');arrow.className='select-arrow';arrow.textContent='▾';arrow.setAttribute('aria-hidden','true');
  $('b-theme').replaceChildren(arrow);$('b-theme').title=(state.previewTheme?'Preview: ':'Theme: ')+name;$('b-theme').setAttribute('aria-label','Theme: '+name);
};
app.clearThemePreview=()=>{clearTimeout(themePreviewTimer);themePreviewToken++;state.previewTheme=null;app.setThemeControl();};
app.previewTheme=id=>{
  if(!state.themes.some(theme=>theme.id===id))return;
  if(state.previewTheme===id||(!state.previewTheme&&state.settings.theme===id))return;
  clearTimeout(themePreviewTimer);state.previewTheme=id;const token=++themePreviewToken;app.setThemeControl();
  themePreviewTimer=setTimeout(()=>send({cmd:'previewTheme',theme:id,token}),55);
};
app.themePreview=payload=>{
  if(payload.token!==themePreviewToken||payload.id!==state.previewTheme)return;
  app.applyTheme(payload.theme);app.setThemeControl();
};
app.commitTheme=id=>{app.clearThemePreview();send({cmd:'setting',key:'theme',value:id});};
app.chooseTheme=()=>openChoices($('b-theme'),state.themes.map(t=>({value:t.id,label:t.name})),state.previewTheme||state.settings.theme,app.commitTheme,app.previewTheme);

const HELP = [
 {id:'start',title:'Getting started',paragraphs:[
  'Choose a folder in Explorer to browse your workspace. Select a file to read it; use Edit when you want to change its text.',
  'Files and the theme arrow sit beside the main menu. Find, Source, Edit and the document-map toggle sit on the right. The quiet controls brighten on hover or keyboard focus. Files shows or hides Explorer. Drag its divider to give the file list more room. Your workspace, theme and sizes are remembered.'
 ]},
 {id:'tabs',title:'Files & tabs',paragraphs:[
  'Right-click a file and choose Open in new tab, or middle-click it. An already-open file switches to its existing tab. New notes and files chosen from the Open dialog also get their own tabs.',
  'A single document shows a quiet title in the app bar. Open another document to reveal tabs. Each tab keeps its draft, editing mode, selection and scroll position. Close a tab with its × button or Ctrl+W. Modified tabs ask you to Save, Discard or Cancel.',
  'Right-click a tab to close it, keep only that tab, or copy its full path. Right-click a file or folder for path-copying and file-manager actions.'
 ]},
 {id:'editing',title:'Writing & saving',paragraphs:[
  'Edit document in the main menu, or Ctrl+E, switches between writing and reading. Save writes the current tab; Save a copy lets you choose another filename. A dot on a tab marks unsaved changes.',
  'If the file changed on disk, the app asks before overwriting it. Reload from the main menu reads the file again after checking your unsaved edits.',
  'Find works in both the reader and editor. Click the Find icon or press Ctrl+F again to close it. Hold Ctrl while scrolling over the document to zoom, or use Ctrl+Plus and Ctrl+Minus. Ctrl+0 returns to 100%. Ordinary scrolling still moves through the document. Fonts, wrapping, tab width and line spacing are available in Options.'
 ]},
 {id:'map',title:'Document map',paragraphs:[
  'The map at the right is a small overview of your current document. Its outlined area shows the visible portion. Click or drag it to move through a long file.',
  'Use the Document map button in the toolbar, the main menu, or Options → Editor to show or hide it. With the map focused, arrow keys and Page Up / Page Down scroll the document.'
 ]},
 {id:'appearance',title:'Appearance',paragraphs:[
  'Hover over a theme in the toolbar menu to preview it. Moving away keeps the preview; click a theme to save it. You can also choose a theme in Options → Appearance. Bold themes cover eight saturated colors, including yellow, orange, pink and purple. Use the Bold, Soft and Classic filters to browse the collections. Search looks through every collection. Text contrast in Options strengthens lettering without changing backgrounds or the quiet toolbar.',
  'Markdown uses a gently offset reading column. Wide tables, code and images use more of the available width and move the column toward the left. The font pickers list installed families. Interface, reading and code fonts are independent. Dropdown choices use the selected app colors and include search for longer lists.',
  'Drag the blank space in the app bar to move the window. Double-click it to maximize or restore. The outer edges resize the window.'
 ]},
 {id:'shortcuts',title:'Keyboard shortcuts',shortcuts:[
  ['New tab','Ctrl+T / Ctrl+N'],['Open file','Ctrl+O'],['Open folder','Ctrl+Shift+O'],['Save','Ctrl+S'],['Save a copy','Ctrl+Shift+S'],['Close tab','Ctrl+W'],['Next / previous tab','Ctrl+Tab / Ctrl+Shift+Tab'],['Edit / preview','Ctrl+E'],['Find','Ctrl+F'],['Zoom in','Ctrl+Plus'],['Zoom out','Ctrl+Minus'],['Reset zoom','Ctrl+0'],['Mouse zoom','Ctrl+wheel'],['Show / hide files','Ctrl+B'],['Reload file','F5'],['Options','Ctrl+,'],['Help','F1'],['Main menu','Alt+F'],['Quit','Ctrl+Q']
 ]}
];
app.drawHelp=()=>{
  const query=$('help-search').value.trim().toLowerCase();$('help-nav').replaceChildren();$('help-content').replaceChildren();
  for(const page of HELP){const b=document.createElement('button');b.textContent=page.title;b.classList.toggle('on',page.id===selectedHelp&&!query);b.onclick=()=>{selectedHelp=page.id;$('help-search').value='';app.drawHelp();};$('help-nav').appendChild(b);}
  const pages=query?HELP.filter(p=>(p.title+' '+(p.paragraphs||[]).join(' ')+' '+JSON.stringify(p.shortcuts||[])).toLowerCase().includes(query)):HELP.filter(p=>p.id===selectedHelp);
  for(const page of pages){const heading=document.createElement('h3');heading.textContent=page.title;$('help-content').appendChild(heading);
    for(const text of page.paragraphs||[]){const p=document.createElement('p');p.textContent=text;$('help-content').appendChild(p);}
    const shortcuts=document.createElement('div');shortcuts.className='shortcut-grid';
    for(const [label,key] of page.shortcuts||[]){const row=document.createElement('div');row.className='shortcut';const name=document.createElement('span');name.textContent=label;const kbd=document.createElement('kbd');kbd.textContent=key;row.append(name,kbd);shortcuts.appendChild(row);}if(shortcuts.children.length)$('help-content').appendChild(shortcuts);}
  if(!pages.length)$('help-content').textContent='No matching help topics.';
  $('help-content').scrollTop=0;app.scheduleDialogFit?.();
};
app.help=open=>{
  closeMenu(false);closeChoices(false);
  if(open){helpFocus=document.activeElement;if($('options').classList.contains('show'))app.options(false);}
  $('help-overlay').classList.toggle('show',open);
  for(const id of ['bar','row','find'])$(id).inert=open;
  if(open){$('help-logo').src=state.logoUrl||'';$('help-title').textContent=state.name;$('help-version').textContent='Version '+state.version+' · Help & shortcuts';$('help-github').href=state.githubUrl||'#';$('help-search').value='';app.drawHelp();$('help-search').focus();}
  else if(helpFocus?.isConnected&&helpFocus.offsetParent)helpFocus.focus();else $('b-menu').focus();
};
$('help-search').oninput=()=>app.drawHelp();
$('help-close').onclick=$('help-done').onclick=()=>app.help(false);
$('help-overlay').onclick=e=>{if(e.target===$('help-overlay'))app.help(false);};
$('help-github').onclick=e=>{e.preventDefault();if(state.githubUrl)send({cmd:'external',url:state.githubUrl});};
$('footer-brand').onclick=()=>app.help(true);
const originalOptions=app.options;
app.options=open=>{closeChoices(false);closeMenu(false);if(open&&$('help-overlay').classList.contains('show'))app.help(false);originalOptions(open);};

function mainMenu(){showMenu($('b-menu'),[
 {label:state.name||'BS Notepad',brand:true,hint:'Help · F1',action:()=>app.help(true)},null,
 {label:'New note',icon:'note',hint:'Ctrl+N',action:()=>$('b-new').click()},
 {label:'Browse for a file…',icon:'open',hint:'Ctrl+O',action:()=>$('b-open').click()},
 {label:'Choose a workspace folder…',icon:'workspace',action:()=>$('folder-open').click()},null,
 {label:'Save edits',icon:'save',hint:'Ctrl+S',action:()=>$('b-save').click()},
 {label:'Save a copy…',icon:'saveAs',hint:'Ctrl+Shift+S',action:()=>$('b-saveas').click()},
 {label:'Read again from disk',icon:'reload',hint:'F5',disabled:!state.path,action:()=>send({cmd:'reload'})},
 {label:'Close this tab',icon:'close',hint:'Ctrl+W',action:()=>send({cmd:'closeTab',id:state.activeTab})},null,
 {label:state.editing?'Read document':'Edit document',icon:state.editing?'read':'edit',hint:'Ctrl+E',action:()=>$('b-edit').click()},
 {label:state.settings.view_mode==='source'?'Rendered view':'Source view',icon:state.settings.view_mode==='source'?'rendered':'source',hint:'Ctrl+U',action:()=>$('b-view').click()},
 {label:'Document map',icon:'map',checked:!!state.settings.minimap,action:()=>$('b-map').click()},
 {label:'Wrap long lines',icon:'wrap',checked:!!state.settings.word_wrap,action:()=>send({cmd:'setting',key:'word_wrap',value:!state.settings.word_wrap})},
 {label:'Options…',icon:'options',hint:'Ctrl+,',action:()=>app.options(true)},null,
 {label:'Quit '+(state.name||'BS Notepad'),icon:'exit',hint:'Ctrl+Q',action:()=>send({cmd:'quit'})}
]);}
$('b-menu').onclick=mainMenu;
$('b-map').onclick=()=>send({cmd:'setting',key:'minimap',value:!state.settings.minimap});
$('window-min').onclick=()=>send({cmd:'windowMinimize'});
$('window-max').onclick=()=>send({cmd:'windowMaximize'});
$('window-close').onclick=()=>send({cmd:'closeWindow'});
$('drag-region').onmousedown=e=>{if(e.button===0&&e.detail===1)send({cmd:'windowDrag'});};
$('drag-region').ondblclick=()=>send({cmd:'windowMaximize'});
for(const edge of $('resize-edges').children)edge.onmousedown=e=>{if(e.button===0){e.preventDefault();send({cmd:'windowResize',direction:edge.dataset.direction});}};
app.windowState=maximized=>{document.body.classList.toggle('maximized',maximized);$('window-max').textContent=maximized?'❐':'□';$('window-max').title=maximized?'Restore':'Maximize';$('window-max').setAttribute('aria-label',maximized?'Restore':'Maximize');};

function scrollTarget(){return state.editing?$('text'):$('doc');}
app.mapPosition=()=>{
 if(!$('minimap')||$('minimap').hidden)return;
 const target=scrollTarget();if(!target)return;
 const total=Math.max(target.scrollHeight,1),visible=Math.min(1,target.clientHeight/total),height=Math.min(mapHeight||$('minimap').clientHeight,$('minimap').clientHeight);
 const viewport=$('map-viewport');viewport.style.height=Math.max(12,height*visible)+'px';
 viewport.style.top=Math.min(Math.max(0,height-12),height*target.scrollTop/total)+'px';
 viewport.setAttribute('aria-valuenow',Math.round(target.scrollTop/Math.max(1,total-target.clientHeight)*100));
 viewport.setAttribute('aria-controls',state.editing?'text':'doc');
};
app.drawMap=()=>{
 if(!$('minimap')||$('minimap').hidden)return;
 const canvas=$('map-canvas'),ctx=canvas.getContext('2d');if(!ctx)return;
 const width=$('minimap').clientWidth,height=$('minimap').clientHeight;if(!width||!height)return;
 const dpr=devicePixelRatio||1;canvas.width=Math.round(width*dpr);canvas.height=Math.round(height*dpr);ctx.scale(dpr,dpr);ctx.clearRect(0,0,width,height);
 const content=state.editing?$('text').value:$('article').innerText;
 const lines=content.split(/\r?\n/),count=Math.min(lines.length,1600),step=lines.length/Math.max(count,1);
 mapHeight=Math.min(height,Math.max(24,lines.length*3.2+12));
 const lineHeight=Math.min(3.2,(mapHeight-12)/Math.max(count,1)),font=Math.min(2.6,Math.max(.8,lineHeight*.8));
 const style=getComputedStyle(document.documentElement),fg=style.getPropertyValue('--fg'),dim=style.getPropertyValue('--dim'),accent=style.getPropertyValue('--accent'),link=style.getPropertyValue('--link');
 ctx.font=font+'px '+(state.settings.code_font||'monospace');ctx.textBaseline='top';ctx.globalAlpha=.7;
 for(let i=0;i<count;i++){const line=lines[Math.floor(i*step)];ctx.fillStyle=/^\s*(#|\/\/|\/\*)/.test(line)?accent:line.includes('"')||line.includes("'")?link:/^\s*$/.test(line)?dim:fg;ctx.fillText(line.slice(0,180),6,6+i*lineHeight,width-12);}
 app.mapPosition();
};
app.scheduleMap=()=>{clearTimeout(mapTimer);mapTimer=setTimeout(()=>{cancelAnimationFrame(mapFrame);mapFrame=requestAnimationFrame(app.drawMap);},75);};
let mapDrag=null;
$('minimap').onpointerdown=e=>{
 if(e.button!==0)return;e.preventDefault();const target=scrollTarget(),rect=$('minimap').getBoundingClientRect(),view=$('map-viewport').getBoundingClientRect();
 mapDrag={pointer:e.pointerId,offset:e.target===$('map-viewport')?e.clientY-view.top:view.height/2};$('minimap').setPointerCapture(e.pointerId);
 target.scrollTop=Math.max(0,(e.clientY-rect.top-mapDrag.offset)/(mapHeight||rect.height)*target.scrollHeight);app.mapPosition();
};
$('minimap').onpointermove=e=>{if(mapDrag){const target=scrollTarget(),rect=$('minimap').getBoundingClientRect();target.scrollTop=Math.max(0,(e.clientY-rect.top-mapDrag.offset)/(mapHeight||rect.height)*target.scrollHeight);app.mapPosition();}};
$('minimap').onpointerup=$('minimap').onpointercancel=()=>{mapDrag=null;};
$('map-viewport').onkeydown=e=>{const target=scrollTarget();let delta=0;if(e.key==='ArrowDown')delta=40;else if(e.key==='ArrowUp')delta=-40;else if(e.key==='PageDown')delta=target.clientHeight;else if(e.key==='PageUp')delta=-target.clientHeight;else if(e.key==='Home')target.scrollTop=0;else if(e.key==='End')target.scrollTop=target.scrollHeight;else return;e.preventDefault();target.scrollTop+=delta;app.mapPosition();};
const originalScroll=$('doc').onscroll;$('doc').onscroll=e=>{originalScroll?.(e);app.mapPosition();};
app.clearPendingZoom=()=>{clearTimeout(zoomTimer);zoomTimer=null;zoomTarget=null;wheelRemainder=0;};
app.flushZoom=()=>{
  if(zoomTimer===null||zoomTarget===null)return;
  clearTimeout(zoomTimer);zoomTimer=null;
  send({cmd:'setting',key:'zoom',value:zoomTarget});
};
app.setZoom=value=>{
  if(!Number.isFinite(value))return;
  const next=Math.round(Math.max(.5,Math.min(3,value))*100)/100;
  if(Math.abs(next-(zoomTarget??state.settings.zoom??1))<.0001)return;
  zoomTarget=next;clearTimeout(zoomTimer);
  zoomTimer=setTimeout(app.flushZoom,120);
  app.applySettings({...state.settings,zoom:next});
};
app.adjustZoom=step=>app.setZoom((Math.round((zoomTarget??state.settings.zoom??1)*100)+step*10)/100);
function zoomBlocked(){return $('options').classList.contains('show')||$('help-overlay').classList.contains('show')||!$('menu-popup').hidden||!$('choice-popup').hidden;}
document.addEventListener('wheel',event=>{
  if(!event.ctrlKey&&!event.metaKey)return;
  event.preventDefault();
  if(event.altKey||zoomBlocked()||!(event.target instanceof Element)||!event.target.closest('#content-row')||!event.deltaY)return;
  const delta=event.deltaY*(event.deltaMode===1?16:event.deltaMode===2?scrollTarget().clientHeight:1),now=performance.now();
  if(now-lastZoomWheel>180||Math.sign(delta)!==Math.sign(wheelRemainder))wheelRemainder=0;
  lastZoomWheel=now;
  if(Math.abs(delta)>=40){wheelRemainder=0;app.adjustZoom(delta<0?1:-1);return;}
  wheelRemainder+=delta;
  if(Math.abs(wheelRemainder)>=40){app.adjustZoom(wheelRemainder<0?1:-1);wheelRemainder=0;}
},{passive:false});
const originalSettings=app.applySettings;
app.applySettings=settings=>{
 if(zoomTarget!==null){
   if(zoomTimer===null&&Math.abs(settings.zoom-zoomTarget)<.0001)zoomTarget=null;
   else settings={...settings,zoom:zoomTarget};
 }
 settings={...settings,zoom:Math.round(settings.zoom*100)/100};
 originalSettings(settings);app.scheduleReaderLayout?.();app.scheduleDialogFit?.();$('minimap').hidden=!settings.minimap;$('b-map').classList.toggle('on',!!settings.minimap);$('b-map').setAttribute('aria-pressed',String(!!settings.minimap));app.scheduleMap();};
const originalTheme=app.applyTheme;app.applyTheme=theme=>{if(theme.id&&theme.id!==(state.previewTheme||state.settings.theme))return;originalTheme(theme);app.scheduleMap();};
new ResizeObserver(()=>{app.scheduleMap();app.scheduleReaderLayout?.();closeChoices(false);closeMenu(false);}).observe($('content-row'));

app.fitTitle=()=>{
  const bar=$('bar').getBoundingClientRect(),left=$('left-tools').getBoundingClientRect(),right=$('document-actions').getBoundingClientRect();
  $('bar').style.setProperty('--title-clearance',Math.ceil(2*(Math.max(left.right-bar.left,bar.right-right.left)+12))+'px');
};
new ResizeObserver(()=>app.fitTitle()).observe($('bar'));
const readerMeasure=document.createElement('canvas').getContext('2d');
app.layoutReader=()=>{
  const article=$('article'),doc=$('doc'),available=doc.clientWidth;if(!available)return;
  const markdown=!state.path||/\.(md|markdown|mdown|mkd|mkdn|mdx)$/i.test(state.path);
  if(!markdown||state.settings.view_mode==='source'){
    article.style.setProperty('--reader-width','100%');article.style.setProperty('--reader-offset','0px');return;
  }
  const style=getComputedStyle(article),padding=parseFloat(style.paddingLeft)+parseFloat(style.paddingRight),size=parseFloat(style.fontSize);
  if(readerMeasure)readerMeasure.font=size+'px '+style.fontFamily;
  const prose=Math.max(680,Math.min(960,(readerMeasure?readerMeasure.measureText('0'.repeat(82)).width:82*size*.55)+padding));
  const blocks=[...article.querySelectorAll('table,pre')].map(element=>({element,width:element.style.getPropertyValue('width'),max:element.style.getPropertyValue('max-width')}));
  for(const block of blocks){block.element.style.width='max-content';block.element.style.maxWidth='none';}
  let widest=prose;
  for(const block of blocks)widest=Math.max(widest,block.element.scrollWidth+padding);
  for(const block of blocks){block.element.style.width=block.width;block.element.style.maxWidth=block.max;}
  for(const image of article.querySelectorAll('img'))widest=Math.max(widest,(Number(image.getAttribute('width'))||image.naturalWidth||0)+padding);
  const width=Math.min(available,Math.ceil(widest)),offset=Math.max(0,Math.floor((available-width)*.4));
  article.style.setProperty('--reader-width',width+'px');article.style.setProperty('--reader-offset',offset+'px');
  app.scheduleMap();
};
app.scheduleReaderLayout=()=>{cancelAnimationFrame(readerFrame);readerFrame=requestAnimationFrame(app.layoutReader);};
function fitDialog(panel,content){
  if(!panel.offsetParent)return;
  const available=innerHeight-32,current=panel.getBoundingClientRect().height;
  const overflow=content.scrollHeight-content.clientHeight;
  const needed=Math.min(available,current+Math.max(0,overflow)+2);
  if((overflow>1||current>available)&&Math.abs(needed-current)>1)panel.style.height=needed+'px';
}
app.scheduleDialogFit=()=>{cancelAnimationFrame(dialogFrame);dialogFrame=requestAnimationFrame(()=>{fitDialog($('panel'),$('sets'));fitDialog($('help-panel'),$('help-content'));});};
const originalDrawOptions=app.drawOptions;app.drawOptions=()=>{originalDrawOptions();app.scheduleDialogFit();};
window.addEventListener('resize',()=>{app.fitTitle();app.scheduleReaderLayout();app.scheduleDialogFit();});
document.fonts?.ready.then(()=>app.scheduleReaderLayout());

document.addEventListener('pointerdown',e=>{if(!$('menu-popup').hidden&&!$('menu-popup').contains(e.target)&&e.target!==menuOrigin&&!menuOrigin?.contains(e.target))closeMenu(false);if(!$('choice-popup').hidden&&!$('choice-popup').contains(e.target)&&e.target!==choiceOrigin&&!choiceOrigin?.contains(e.target))closeChoices(false);});
document.addEventListener('keydown',e=>{
 const ctrl=e.ctrlKey||e.metaKey;
 const popup=!$('menu-popup').hidden?$('menu-popup'):!$('choice-popup').hidden?$('choice-popup'):null;
 if(popup){
   if(e.key==='Escape'||e.key==='Tab'){e.preventDefault();e.stopImmediatePropagation();popup===$('menu-popup')?closeMenu():closeChoices();return;}
   if(['ArrowDown','ArrowUp','Home','End'].includes(e.key)){
     e.preventDefault();e.stopImmediatePropagation();const buttons=[...popup.querySelectorAll(popup.id==='choice-popup'?'#choice-list button:not(:disabled)':'button:not(:disabled)')],index=buttons.indexOf(document.activeElement);
     const next=e.key==='Home'?0:e.key==='End'?buttons.length-1:(index+(e.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length;buttons[next]?.focus();return;
   }
   if(e.key==='Enter'&&document.activeElement===$('choice-search')){e.preventDefault();e.stopImmediatePropagation();$('choice-list').querySelector('button')?.click();return;}
   if(ctrl){e.stopImmediatePropagation();return;}
 }
 if($('help-overlay').classList.contains('show')){
   if(e.key==='Escape'){e.preventDefault();app.help(false);}
   if(e.key==='Tab'){const controls=[...$('help-panel').querySelectorAll('button,input,a,[tabindex="0"]')].filter(el=>el.offsetParent);const first=controls[0],last=controls.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first.focus();}}
   e.stopImmediatePropagation();return;
 }
 if(e.key==='F1'){e.preventDefault();e.stopImmediatePropagation();app.help(true);return;}
 if($('options').classList.contains('show'))return;
 if(e.altKey&&e.key.toLowerCase()==='f'){e.preventDefault();e.stopImmediatePropagation();mainMenu();return;}
 if(ctrl&&e.key.toLowerCase()==='t'){e.preventDefault();e.stopImmediatePropagation();send({cmd:'new'});}
 else if(ctrl&&e.key.toLowerCase()==='w'){e.preventDefault();e.stopImmediatePropagation();send({cmd:'closeTab',id:state.activeTab});}
 else if(ctrl&&e.key.toLowerCase()==='q'){e.preventDefault();e.stopImmediatePropagation();send({cmd:'quit'});}
 else if(ctrl&&e.key==='Tab'){e.preventDefault();e.stopImmediatePropagation();const tabs=state.tabs||[],index=tabs.findIndex(t=>t.id===state.activeTab);if(tabs.length)send({cmd:'activateTab',id:tabs[(index+(e.shiftKey?-1:1)+tabs.length)%tabs.length].id});}
 else if(e.key==='F5'){e.preventDefault();e.stopImmediatePropagation();send({cmd:'reload'});}
},true);
