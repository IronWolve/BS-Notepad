let quickSerial=0,quickTimer,quickIndex=-1,quickFiles=[],quickFocus;
function quickSelect(index){
 const rows=[...$('quick-list').children];quickIndex=Math.max(0,Math.min(index,rows.length-1));
 rows.forEach((row,i)=>row.setAttribute('aria-selected',String(i===quickIndex)));
 if(rows[quickIndex]){$('quick-input').setAttribute('aria-activedescendant',rows[quickIndex].id);rows[quickIndex].scrollIntoView({block:'nearest'});}
 else $('quick-input').removeAttribute('aria-activedescendant');
}
function quickSearch(){
 clearTimeout(quickTimer);quickFiles=[];quickIndex=-1;$('quick-list').replaceChildren();$('quick-input').removeAttribute('aria-activedescendant');$('quick-status').textContent='Searching workspace…';
 send({cmd:'quickSearch',request:++quickSerial,query:$('quick-input').value});
}
function closeQuick(){
 clearTimeout(quickTimer);++quickSerial;send({cmd:'cancelQuickSearch'});$('quick-dialog').close();quickFocus?.isConnected&&quickFocus.focus();
}
app.quickOpen=()=>{
 if($('quick-dialog').open){closeQuick();return;}
 if($('file-dialog').open||$('options').classList.contains('show')||$('help-overlay').classList.contains('show'))return;
 closeMenu(false);closeChoices(false);quickFocus=document.activeElement;$('quick-input').value='';$('quick-dialog').showModal();$('quick-input').focus();quickSearch();
};
app.quickMatches=payload=>{
 if(!$('quick-dialog').open||payload.request!==quickSerial)return;
 quickFiles=payload.matches||[];$('quick-list').replaceChildren();
 for(const [index,file] of quickFiles.entries()){
  const button=document.createElement('button');button.type='button';button.id='quick-match-'+index;button.setAttribute('role','option');button.tabIndex=-1;button.title=cleanPath(file.path);
  const label=document.createElement('span');label.className='quick-file-label';const name=document.createElement('strong'),folder=document.createElement('small');name.textContent=file.name;folder.textContent=file.folder||'Workspace';label.append(name,folder);button.append(fileIcon(file.name,false),label);
  button.onclick=()=>{closeQuick();send({cmd:'openPath',path:file.path,newTab:true});};$('quick-list').appendChild(button);
 }
 quickSelect(0);
 $('quick-status').textContent=(quickFiles.length?quickFiles.length+' matching file'+(quickFiles.length===1?'':'s')+'. Enter to open.':'No matching files.')+(payload.limited?' Search limited; narrow the filename or path.':'')+(payload.skipped?' Some folders could not be read.':'');
};
$('quick-input').oninput=()=>{clearTimeout(quickTimer);++quickSerial;quickFiles=[];$('quick-list').replaceChildren();$('quick-input').removeAttribute('aria-activedescendant');$('quick-status').textContent='Searching workspace…';quickTimer=setTimeout(quickSearch,140);};
$('quick-dialog').onkeydown=event=>{
 if((event.ctrlKey||event.metaKey)&&event.key.toLowerCase()==='p'){event.preventDefault();event.stopPropagation();closeQuick();return;}
 if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();quickSelect(quickIndex+(event.key==='ArrowDown'?1:-1));}
 if(event.key==='Enter'){event.preventDefault();$('quick-list').children[quickIndex]?.click();}
};
$('quick-close').onclick=closeQuick;$('quick-dialog').oncancel=event=>{event.preventDefault();closeQuick();};
let fileSerial=0,fileOperation=null,fileFocus;
app.fileAction=(kind,path,folder)=>{
 if(!path||$('file-dialog').open)return;
 closeMenu(false);closeChoices(false);fileFocus=document.activeElement;fileOperation={kind,path,parent:kind!=='rename'&&!folder,request:++fileSerial,busy:false};
 $('file-title').textContent=kind==='rename'?'Rename':'New folder';$('file-submit').textContent=kind==='rename'?'Rename':'Create';$('file-submit').disabled=$('file-cancel').disabled=$('file-name').disabled=false;$('file-error').textContent='';$('file-name').value=kind==='rename'?fileName(path):'';
 $('file-dialog').showModal();$('file-name').focus();const dot=$('file-name').value.lastIndexOf('.');$('file-name').setSelectionRange(0,kind==='rename'&&!folder&&dot>0?dot:$('file-name').value.length);
};
function closeFileAction(){if(fileOperation?.busy)return;$('file-dialog').close();fileOperation=null;fileFocus?.isConnected&&fileFocus.focus();}
$('file-cancel').onclick=closeFileAction;$('file-dialog').oncancel=event=>{event.preventDefault();closeFileAction();};
$('file-form').onsubmit=event=>{
 event.preventDefault();if(!fileOperation||fileOperation.busy||!$('file-form').reportValidity())return;
 fileOperation.busy=true;$('file-submit').disabled=$('file-cancel').disabled=$('file-name').disabled=true;$('file-error').textContent='Working…';send({cmd:'fileAction',...fileOperation,name:$('file-name').value});
};
app.fileOperation=payload=>{
 if(!fileOperation||payload.request!==fileOperation.request)return;
 fileOperation.busy=false;$('file-submit').disabled=$('file-cancel').disabled=$('file-name').disabled=false;
 if(payload.error){$('file-error').textContent=payload.error;$('file-name').focus();return;}
 if(payload.restoreTree){state.restoreTreeAfterFileAction=true;state.focusFilePath=payload.path;}
 closeFileAction();app.note('Folder or filename updated.');
};

app.focusPendingFile=()=>{
 if(!state.focusFilePath||state.restoringTree)return;
 const row=[...$('pane-files').querySelectorAll('[data-path]')].find(node=>samePath(node.dataset.path,state.focusFilePath));
 if(row){row.focus({preventScroll:true});row.scrollIntoView({block:'nearest'});state.focusFilePath=null;}
};
