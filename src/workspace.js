// Documents keep their own editor nodes so switching tabs preserves native undo.
const editorNodes = new Map(), pendingViews = new Map();
let mapTimer, mapFrame, mapHeight = 0, helpFocus, selectedHelp = 'start';
let menuOrigin, choiceOrigin, choiceOptions = [], choiceValue, choiceChange;
const cleanPath = path => normalizedPath(path || '');
const fileName = path => cleanPath(path).split('/').pop() || 'Untitled';

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
        {label:'Close this tab',hint:'Ctrl+W',action:()=>send({cmd:'closeTab',id:tab.id})},
        {label:'Keep only this tab',disabled:payload.tabs.length<2,action:()=>send({cmd:'closeOtherTabs',id:tab.id})},
        null,{label:'Copy full path',disabled:!tab.path,action:()=>copyText(cleanPath(tab.path))}
      ],e);
    };
    wrap.append(button,close);scroller.appendChild(wrap);
  }
  $('content-row').setAttribute('aria-labelledby','tab-'+payload.active);
  app.documentChrome();
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
app.setDocument = document => {
  const id=document.tab ?? state.activeTab ?? 1;
  const changed=state.activeTab!==id || !editorNodes.has(id);
  editorFor(id);
  if(changed)pendingViews.set(id,document);
  baseDocument(document);
  app.toggleEdit(document.editing ?? state.editing,false);
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
    const icon=document.createElement('span');icon.className='menu-symbol';icon.textContent=item.symbol||'';icon.setAttribute('aria-hidden','true');
    const label=document.createElement('span');label.className='menu-label';label.textContent=item.label;
    const hint=document.createElement('span');hint.className='menu-hint';hint.textContent=item.hint||'';
    button.append(icon,label,hint);button.onclick=()=>{closeMenu();item.action();};menu.appendChild(button);
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
    {label:row.getAttribute('aria-expanded')==='true'?'Fold this folder':'Expand this folder',symbol:'›',action:()=>row.click()},
    {label:'Browse from this folder',action:()=>send({cmd:'workspacePath',path})}
  ]:[
    {label:'Open here',symbol:'↵',action:()=>send({cmd:'openPath',path})},
    {label:'Open in new tab',symbol:'+',action:()=>send({cmd:'openPath',path,newTab:true})}
  ];
  entries.push(null,
    {label:'Copy full path',action:()=>copyText(cleanPath(path))},
    {label:'Copy filename',action:()=>copyText(fileName(path))},
    {label:folder?'Open folder in file manager':'Open containing folder',action:()=>send({cmd:'showFolder',path})},
    null,{label:'Refresh file list',symbol:'↻',action:()=>send({cmd:'refreshTree'})});
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
function drawChoices() {
  const query=$('choice-search').value.toLowerCase(),list=$('choice-list');list.replaceChildren();
  const choices=choiceOptions.filter(o=>o.label.toLowerCase().includes(query));
  for(const option of choices) {
    const button=document.createElement('button');button.setAttribute('role','option');button.setAttribute('aria-selected',String(option.value===choiceValue));button.textContent=option.label;
    button.onclick=()=>{const callback=choiceChange;closeChoices();callback(option.value);};list.appendChild(button);
  }
  if(!choices.length){const empty=document.createElement('div');empty.className='choice-empty';empty.textContent='No matching choices';list.appendChild(empty);}
}
function openChoices(anchor,options,value,change) {
  closeMenu(false);closeChoices(false);choiceOrigin=anchor;choiceOptions=options;choiceValue=value;choiceChange=change;
  anchor.setAttribute('aria-expanded','true');$('choice-search').value='';$('choice-search').hidden=options.length<9;
  drawChoices();const popup=$('choice-popup');popup.style.width=Math.max(220,Math.min(400,anchor.getBoundingClientRect().width))+'px';popupPosition(popup,anchor);syncPopupState();
  if(!$('choice-search').hidden)$('choice-search').focus();
  else (popup.querySelector('[aria-selected=true]') || popup.querySelector('button'))?.focus();
  popup.querySelector('[aria-selected=true]')?.scrollIntoView({block:'nearest'});
}
$('choice-search').oninput=drawChoices;
app.setThemeControl=()=>{
  const name=state.themes.find(t=>t.id===state.settings.theme)?.name||'Theme';
  const arrow=document.createElement('span');arrow.className='select-arrow';arrow.textContent='▾';arrow.setAttribute('aria-hidden','true');
  $('b-theme').replaceChildren(arrow);$('b-theme').title='Theme: '+name;$('b-theme').setAttribute('aria-label','Theme: '+name);
};
app.chooseTheme=()=>openChoices($('b-theme'),state.themes.map(t=>({value:t.id,label:t.name})),state.settings.theme,value=>send({cmd:'setting',key:'theme',value}));

const HELP = [
 {id:'start',title:'Getting started',paragraphs:[
  'Choose a folder in Explorer to browse your workspace. Select a file to read it; use Edit when you want to change its text.',
  'Move the pointer over the left side of the app bar to reveal Find, Files and the theme arrow. The Files button shows or hides Explorer. Drag its divider to give the file list more room. Your workspace, theme and sizes are remembered.'
 ]},
 {id:'tabs',title:'Files & tabs',paragraphs:[
  'Right-click a file and choose Open in new tab, or middle-click it. An already-open file switches to its existing tab. New notes and files chosen from the Open dialog also get their own tabs.',
  'A single document shows a quiet title in the app bar. Open another document to reveal tabs. Each tab keeps its draft, editing mode, selection and scroll position. Close a tab with its × button or Ctrl+W. Modified tabs ask you to Save, Discard or Cancel.',
  'Right-click a tab to close it, keep only that tab, or copy its full path. Right-click a file or folder for path-copying and file-manager actions.'
 ]},
 {id:'editing',title:'Writing & saving',paragraphs:[
  'Edit document in the main menu, or Ctrl+E, switches between writing and reading. Save writes the current tab; Save a copy lets you choose another filename. A dot on a tab marks unsaved changes.',
  'If the file changed on disk, the app asks before overwriting it. Reload from the main menu reads the file again after checking your unsaved edits.',
  'Find works in both the reader and editor. Fonts, wrapping, tab width, line spacing and zoom are available in Options.'
 ]},
 {id:'map',title:'Document map',paragraphs:[
  'The map at the right is a small overview of your current document. Its outlined area shows the visible portion. Click or drag it to move through a long file.',
  'Use the Document map button in the toolbar, the main menu, or Options → Editor to show or hide it. With the map focused, arrow keys and Page Up / Page Down scroll the document.'
 ]},
 {id:'appearance',title:'Appearance',paragraphs:[
  'Choose a color theme from the toolbar or Options → Appearance. Mist and Sage are softer light palettes; Slate and Graphite offer medium contrast.',
  'The font pickers list installed families. Interface, reading and code fonts are independent. Dropdown choices use the selected app colors and include search for longer lists.',
  'Drag the blank space in the app bar to move the window. Double-click it to maximize or restore. The outer edges resize the window.'
 ]},
 {id:'shortcuts',title:'Keyboard shortcuts',shortcuts:[
  ['New tab','Ctrl+T / Ctrl+N'],['Open file','Ctrl+O'],['Open folder','Ctrl+Shift+O'],['Save','Ctrl+S'],['Save a copy','Ctrl+Shift+S'],['Close tab','Ctrl+W'],['Next / previous tab','Ctrl+Tab / Ctrl+Shift+Tab'],['Edit / preview','Ctrl+E'],['Find','Ctrl+F'],['Show / hide files','Ctrl+B'],['Reload file','F5'],['Options','Ctrl+,'],['Help','F1'],['Main menu','Alt+F'],['Quit','Ctrl+Q']
 ]}
];
app.drawHelp=()=>{
  const query=$('help-search').value.trim().toLowerCase();$('help-nav').replaceChildren();$('help-content').replaceChildren();
  for(const page of HELP){const b=document.createElement('button');b.textContent=page.title;b.classList.toggle('on',page.id===selectedHelp&&!query);b.onclick=()=>{selectedHelp=page.id;$('help-search').value='';app.drawHelp();};$('help-nav').appendChild(b);}
  const pages=query?HELP.filter(p=>(p.title+' '+(p.paragraphs||[]).join(' ')+' '+JSON.stringify(p.shortcuts||[])).toLowerCase().includes(query)):HELP.filter(p=>p.id===selectedHelp);
  for(const page of pages){const heading=document.createElement('h3');heading.textContent=page.title;$('help-content').appendChild(heading);
    for(const text of page.paragraphs||[]){const p=document.createElement('p');p.textContent=text;$('help-content').appendChild(p);}
    for(const [label,key] of page.shortcuts||[]){const row=document.createElement('div');row.className='shortcut';const name=document.createElement('span');name.textContent=label;const kbd=document.createElement('kbd');kbd.textContent=key;row.append(name,kbd);$('help-content').appendChild(row);}}
  if(!pages.length)$('help-content').textContent='No matching help topics.';
  $('help-content').scrollTop=0;
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
 {label:'New note',symbol:'+',hint:'Ctrl+N',action:()=>$('b-new').click()},
 {label:'Browse for a file…',symbol:'↗',hint:'Ctrl+O',action:()=>$('b-open').click()},
 {label:'Choose a workspace folder…',action:()=>$('folder-open').click()},null,
 {label:'Save edits',symbol:'✓',hint:'Ctrl+S',action:()=>$('b-save').click()},
 {label:'Save a copy…',hint:'Ctrl+Shift+S',action:()=>$('b-saveas').click()},
 {label:'Read again from disk',symbol:'↻',hint:'F5',disabled:!state.path,action:()=>send({cmd:'reload'})},
 {label:'Close this tab',hint:'Ctrl+W',action:()=>send({cmd:'closeTab',id:state.activeTab})},null,
 {label:state.editing?'Read document':'Edit document',hint:'Ctrl+E',action:()=>$('b-edit').click()},
 {label:state.settings.view_mode==='source'?'Rendered view':'Source view',hint:'Ctrl+U',action:()=>$('b-view').click()},
 {label:'Document map',symbol:state.settings.minimap?'✓':'',action:()=>$('b-map').click()},
 {label:'Wrap long lines',symbol:state.settings.word_wrap?'✓':'',action:()=>send({cmd:'setting',key:'word_wrap',value:!state.settings.word_wrap})},
 {label:'Options…',hint:'Ctrl+,',action:()=>app.options(true)},
 null,
 {label:'Quit '+(state.name||'BS Notepad'),hint:'Ctrl+Q',action:()=>send({cmd:'quit'})}
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
const originalSettings=app.applySettings;
app.applySettings=settings=>{originalSettings(settings);$('minimap').hidden=!settings.minimap;$('b-map').classList.toggle('on',!!settings.minimap);$('b-map').setAttribute('aria-pressed',String(!!settings.minimap));app.scheduleMap();};
const originalTheme=app.applyTheme;app.applyTheme=theme=>{originalTheme(theme);app.scheduleMap();};
new ResizeObserver(()=>{app.scheduleMap();closeChoices(false);closeMenu(false);}).observe($('content-row'));

document.addEventListener('pointerdown',e=>{if(!$('menu-popup').hidden&&!$('menu-popup').contains(e.target)&&e.target!==menuOrigin&&!menuOrigin?.contains(e.target))closeMenu(false);if(!$('choice-popup').hidden&&!$('choice-popup').contains(e.target)&&e.target!==choiceOrigin&&!choiceOrigin?.contains(e.target))closeChoices(false);});
document.addEventListener('keydown',e=>{
 const ctrl=e.ctrlKey||e.metaKey;
 const popup=!$('menu-popup').hidden?$('menu-popup'):!$('choice-popup').hidden?$('choice-popup'):null;
 if(popup){
   if(e.key==='Escape'||e.key==='Tab'){e.preventDefault();e.stopImmediatePropagation();popup===$('menu-popup')?closeMenu():closeChoices();return;}
   if(['ArrowDown','ArrowUp','Home','End'].includes(e.key)){
     e.preventDefault();e.stopImmediatePropagation();const buttons=[...popup.querySelectorAll('button:not(:disabled)')],index=buttons.indexOf(document.activeElement);
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
