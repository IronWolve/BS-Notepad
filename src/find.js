// Search uses original UTF-16 offsets, including when Unicode case mappings differ in length.
let marks = [], searchHits = [], at = -1, editorHits = [];
function clearMarks() {
  for (const mark of marks) if(mark.parentNode) { const parent=mark.parentNode;mark.replaceWith(document.createTextNode(mark.textContent));parent.normalize(); }
  marks=[];searchHits=[];editorHits=[];at=-1;$('hits').textContent='';$('find-prev').disabled=$('find-next').disabled=true;
}
function matchesIn(text,query) {
  if(!query)return [];
  const escaped=query.replace(/[.*+?^${}()|[\]\\]/g,'\\$&'), regex=new RegExp(escaped,$('find-case').getAttribute('aria-pressed')==='true'?'gu':'giu');
  const whole=$('find-word').getAttribute('aria-pressed')==='true', word=c=>!!c&&/[\p{L}\p{N}_]/u.test(c),hits=[];
  for(const m of text.matchAll(regex)) {
    const start=m.index,end=start+m[0].length;
    if(whole&&(word(Array.from(text.slice(Math.max(0,start-2),start)).at(-1))||word(String.fromCodePoint(text.codePointAt(end)||32))))continue;
    hits.push({start,end});
  }
  return hits;
}
function runFind(query,navigate=true) {
  const previous=at;clearMarks();if(!query)return;
  if(state.editing) {
    searchHits=matchesIn($('text').value,query);editorHits=searchHits.map(hit=>hit.start);
  } else {
    const nodes=[],walker=document.createTreeWalker($('article'),NodeFilter.SHOW_TEXT,{acceptNode:node=>node.parentElement?.closest('button,script,style')?NodeFilter.FILTER_REJECT:NodeFilter.FILTER_ACCEPT});
    let text='',node,lastBlock;
    while((node=walker.nextNode())) {
      const block=node.parentElement.closest('p,pre,li,h1,h2,h3,h4,h5,h6,td,th,blockquote,div');
      if(lastBlock&&block!==lastBlock)text+='\n';lastBlock=block;
      nodes.push({node,start:text.length,end:text.length+node.length});text+=node.nodeValue;
    }
    searchHits=matchesIn(text,query).map(hit=>({...hit,marks:[]}));
    for(const item of nodes.reverse()) {
      const intersections=searchHits.map((hit,index)=>({hit,index,start:Math.max(item.start,hit.start)-item.start,end:Math.min(item.end,hit.end)-item.start})).filter(x=>x.end>x.start).reverse();
      for(const x of intersections) { const range=document.createRange();range.setStart(item.node,x.start);range.setEnd(item.node,x.end);const mark=document.createElement('mark');range.surroundContents(mark);marks.push(mark);x.hit.marks.push(mark); }
    }
  }
  at=searchHits.length?Math.min(Math.max(0,navigate?0:previous),searchHits.length-1):-1;
  focusMark(navigate);
}
function focusMark(navigate=true) {
  $('find-prev').disabled=$('find-next').disabled=!searchHits.length;
  $('hits').textContent=searchHits.length?(at+1)+' of '+searchHits.length:'No matches';
  if(state.editing) {
    const hit=searchHits[at];if(navigate&&hit){const editor=$('text');editor.setSelectionRange(hit.start,hit.end);editor.scrollTop=(editor.value.slice(0,hit.start).split('\n').length-1)*parseFloat(getComputedStyle(editor).lineHeight)-editor.clientHeight/2;}
  } else {for(const [index,hit] of searchHits.entries())for(const mark of hit.marks)mark.classList.toggle('on',index===at);if(navigate)searchHits[at]?.marks[0]?.scrollIntoView({block:'center'});}
}
function step(delta) {if(!searchHits.length)return;at=(at+delta+searchHits.length)%searchHits.length;focusMark();}
app.refreshFind=()=>{if($('find').classList.contains('show'))runFind($('find-text').value,false);};
app.replaceFound=all=>{
  const query=$('find-text').value;if(!query||state.readOnly)return;
  const editor=$('text'),hits=matchesIn(editor.value,query);if(!hits.length)return;
  const replacement=$('replace-text').value;
  let start,end,value;
  if(all){start=0;end=editor.value.length;value=editor.value;for(const hit of hits.slice().reverse())value=value.slice(0,hit.start)+replacement+value.slice(hit.end);}
  else {const hit=hits[state.editing?Math.max(0,Math.min(at,hits.length-1)):0];start=hit.start;end=hit.end;value=replacement;}
  app.toggleEdit(true);editor.focus();editor.setSelectionRange(start,end);
  if(!document.execCommand('insertText',false,value)){editor.setRangeText(value,start,end,'end');editor.dispatchEvent(new Event('input'));}
  runFind(query,false);app.updateStatus?.();
};
for(const id of ['find-case','find-word'])$(id).onclick=()=>{$(id).setAttribute('aria-pressed',String($(id).getAttribute('aria-pressed')!=='true'));runFind($('find-text').value);};
$('find-replace').onclick=()=>{$('replace-row').hidden=!$('replace-row').hidden;$('find-replace').setAttribute('aria-expanded',String(!$('replace-row').hidden));if(!$('replace-row').hidden)$('replace-text').focus();};
$('replace-one').onclick=()=>app.replaceFound(false);$('replace-all').onclick=()=>app.replaceFound(true);
app.updateStatus=()=>{
 const editor=$('text');if(!editor)return;
 const prefix=editor.value.slice(0,editor.selectionStart),line=prefix.split('\n').length,column=Array.from(prefix.slice(prefix.lastIndexOf('\n')+1)).length+1;
 $('document-status').hidden=!state.settings.status_bar;
 $('cursor-status').textContent=state.editing?'Ln '+line+', Col '+column:'Read mode';
 $('format-status').textContent=(state.encoding||'UTF-8')+' · '+(state.lineEnding||'LF')+(state.readOnly?' · Read-only preview':'');
};
app.goToLine=()=>{if(state.readOnly){app.note('This is a read-only large-file preview.');return;}app.toggleEdit(true);$('line-number').max=$('text').value.split('\n').length;$('line-number').value=1;$('line-dialog').showModal();$('line-number').select();};
$('line-form').onsubmit=e=>{e.preventDefault();const line=Math.max(1,Math.min(Number($('line-number').value)||1,Number($('line-number').max)));let pos=0;for(let i=1;i<line;i++)pos=$('text').value.indexOf('\n',pos)+1;$('line-dialog').close();const editor=$('text');editor.focus();editor.setSelectionRange(pos,pos);editor.scrollTop=(line-1)*parseFloat(getComputedStyle(editor).lineHeight)-editor.clientHeight/2;app.updateStatus();};
$('line-cancel').onclick=()=>$('line-dialog').close();$('cursor-status').onclick=()=>app.goToLine();
document.addEventListener('selectionchange',()=>{if(document.activeElement===$('text'))app.updateStatus();});
