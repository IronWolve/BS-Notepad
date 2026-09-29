// Search uses original UTF-16 offsets, including when Unicode case mappings differ in length.
let marks = [], searchHits = [], at = -1, findTimer, statusFrame;
const FIND_LIMIT=5000, lineCache=new WeakMap();
function clearMarks() {
  clearTimeout(findTimer);findTimer=null;
  const parents=new Set();for (const mark of marks) if(mark.parentNode) { parents.add(mark.parentNode);mark.replaceWith(document.createTextNode(mark.textContent)); }
  for(const parent of parents)parent.normalize();
  marks=[];searchHits=[];at=-1;$('hits').textContent='';$('find-prev').disabled=$('find-next').disabled=true;
}
function matchesIn(text,query,limit=FIND_LIMIT) {
  if(!query)return [];
  const escaped=query.replace(/[.*+?^${}()|[\]\\]/g,'\\$&'), regex=new RegExp(escaped,$('find-case').getAttribute('aria-pressed')==='true'?'gu':'giu');
  const whole=$('find-word').getAttribute('aria-pressed')==='true', word=c=>!!c&&/[\p{L}\p{M}\p{N}_]/u.test(c),hits=[];
  for(const m of text.matchAll(regex)) {
    const start=m.index;let end=start+m[0].length;
    if(whole&&(word(Array.from(text.slice(Math.max(0,start-2),start)).at(-1))||word(String.fromCodePoint(text.codePointAt(end)||32))))continue;
    while(end<text.length&&/\p{M}/u.test(String.fromCodePoint(text.codePointAt(end))))end+=String.fromCodePoint(text.codePointAt(end)).length;
    if(!hits.length||start>=hits.at(-1).end)hits.push({start,end});
    if(hits.length>=limit)break;
  }
  return hits;
}
function runFind(query,navigate=true) {
  clearTimeout(findTimer);findTimer=null;const previous=at;clearMarks();if(!query)return;
  if(state.editing) {
    searchHits=matchesIn($('text').value,query);
  } else {
    const nodes=[],walker=document.createTreeWalker($('article'),NodeFilter.SHOW_TEXT,{acceptNode:node=>node.parentElement?.closest('button,script,style')?NodeFilter.FILTER_REJECT:NodeFilter.FILTER_ACCEPT});
    let text='',node,lastBlock;
    while((node=walker.nextNode())) {
      const block=node.parentElement.closest('p,pre,li,h1,h2,h3,h4,h5,h6,td,th,blockquote,div');
      if(lastBlock&&block!==lastBlock)text+='\n';lastBlock=block;
      nodes.push({node,start:text.length,end:text.length+node.length});text+=node.nodeValue;
    }
    searchHits=matchesIn(text,query).map(hit=>({...hit,marks:[]}));
    let first=0;
    for(const item of nodes) {
      while(first<searchHits.length&&searchHits[first].end<=item.start)first++;
      const overlaps=[];
      for(let index=first;index<searchHits.length&&searchHits[index].start<item.end;index++) {
        const hit=searchHits[index],start=Math.max(item.start,hit.start)-item.start,end=Math.min(item.end,hit.end)-item.start;
        if(end>start)overlaps.push({hit,start,end});
      }
      for(const x of overlaps.reverse()) {const range=document.createRange();range.setStart(item.node,x.start);range.setEnd(item.node,x.end);const mark=document.createElement('mark');range.surroundContents(mark);marks.push(mark);x.hit.marks.push(mark);}
    }
  }
  at=searchHits.length?Math.min(Math.max(0,navigate?0:previous),searchHits.length-1):-1;
  focusMark(navigate);
}
function revealSelection(editor,start,end,focus=true) {
 editor.setSelectionRange(start,end);if(focus)editor.focus({preventScroll:true});
 const style=getComputedStyle(editor),mirror=document.createElement('div');
 for(const key of ['fontFamily','fontSize','fontWeight','fontStyle','lineHeight','letterSpacing','wordSpacing','tabSize','paddingTop','paddingBottom','paddingLeft','paddingRight','textIndent','direction'])mirror.style[key]=style[key];
 Object.assign(mirror.style,{position:'fixed',left:'-100000px',top:'0',width:editor.clientWidth+'px',boxSizing:'border-box',whiteSpace:editor.wrap==='off'?'pre':'pre-wrap',overflowWrap:'break-word',visibility:'hidden'});
 mirror.textContent=editor.value.slice(0,start);const marker=document.createElement('span');marker.textContent='\u200b';mirror.appendChild(marker);document.body.appendChild(mirror);
 editor.scrollTop=Math.max(0,marker.offsetTop-editor.clientHeight/2);if(editor.wrap==='off')editor.scrollLeft=Math.max(0,marker.offsetLeft-editor.clientWidth/2);mirror.remove();
}
function focusMark(navigate=true,focusEditor=false) {
 $('find-prev').disabled=$('find-next').disabled=!searchHits.length;
 $('hits').textContent=searchHits.length?(at+1)+' of '+searchHits.length+(searchHits.length===FIND_LIMIT?'+ · narrow search':''):'No matches';
 if(state.editing) {const hit=searchHits[at];if(navigate&&hit)revealSelection($('text'),hit.start,hit.end,focusEditor);}
 else {for(const [index,hit] of searchHits.entries())for(const mark of hit.marks)mark.classList.toggle('on',index===at);if(navigate)searchHits[at]?.marks[0]?.scrollIntoView({block:'center'});}
}
function step(delta) {if(findTimer)runFind($('find-text').value,false);if(!searchHits.length)return;at=(at+delta+searchHits.length)%searchHits.length;focusMark(true,true);}
app.refreshFind=()=>{if($('find').classList.contains('show')){clearTimeout(findTimer);findTimer=setTimeout(()=>runFind($('find-text').value,false),150);}};
$('find-text').oninput=()=>{clearTimeout(findTimer);findTimer=setTimeout(()=>runFind($('find-text').value),150);};
app.replaceFound=all=>{
 const query=$('find-text').value;if(!query||state.readOnly||state.image)return;
 if(!state.editing){app.toggleEdit(true);runFind(query);app.note('Replace works in source text. Review these matches, then choose Replace.');return;}
 const editor=$('text'),hits=matchesIn(editor.value,query,Infinity);if(!hits.length)return;
 const replacement=$('replace-text').value;let start,end,value;
 if(all){start=hits[0].start;end=hits.at(-1).end;const parts=[];let prior=start;for(const hit of hits){parts.push(editor.value.slice(prior,hit.start),replacement);prior=hit.end;}value=parts.join('');}
 else {const hit=hits.find(hit=>hit.start===editor.selectionStart&&hit.end===editor.selectionEnd);
  if(!hit){const next=hits.find(hit=>hit.start>=editor.selectionStart)||hits[0];revealSelection(editor,next.start,next.end);runFind(query,false);at=searchHits.findIndex(hit=>hit.start===next.start);focusMark(false);app.note('Match selected. Choose Replace to change it.');return;}
  start=hit.start;end=hit.end;value=replacement;
 }
 editor.focus({preventScroll:true});editor.setSelectionRange(start,end);
 if(!document.execCommand('insertText',false,value)){editor.setRangeText(value,start,end,'end');editor.dispatchEvent(new Event('input'));}
 runFind(query,false);app.updateStatus?.();
};
for(const id of ['find-case','find-word'])$(id).onclick=()=>{$(id).setAttribute('aria-pressed',String($(id).getAttribute('aria-pressed')!=='true'));runFind($('find-text').value);};
$('find-replace').onclick=()=>{$('replace-row').hidden=!$('replace-row').hidden;$('find-replace').setAttribute('aria-expanded',String(!$('replace-row').hidden));if(!$('replace-row').hidden)$('replace-text').focus();};
$('replace-one').onclick=()=>app.replaceFound(false);$('replace-all').onclick=()=>app.replaceFound(true);
function patchLineStarts(starts,start,end,insert) {
 const upper=offset=>{let low=0,high=starts.length;while(low<high){const mid=(low+high)>>>1;if(starts[mid]<=offset)low=mid+1;else high=mid;}return low;};
 const first=upper(start),last=upper(end),delta=insert.length-(end-start),added=[];let pos=-1;
 while((pos=insert.indexOf('\n',pos+1))!==-1)added.push(start+pos+1);
 if(added.length>4096)return starts.slice(0,first).concat(added,starts.slice(last).map(offset=>offset+delta));
 starts.splice(first,last-first,...added);for(let index=first+added.length;index<starts.length;index++)starts[index]+=delta;return starts;
}
app.patchLines=(node,revision,start,end,insert,length)=>{const cached=lineCache.get(node);if(cached&&Number(cached.revision)===revision){cached.starts=patchLineStarts(cached.starts,start,end,insert);cached.revision=String(revision+1);cached.length=length;}};
app.invalidateLines=node=>lineCache.delete(node);
function editorLines(editor) {
 const revision=editor.dataset.revision,value=editor.value,cached=lineCache.get(editor);
 if(cached&&cached.revision===revision&&cached.length===value.length)return cached.starts;
 const starts=[0];let pos=0;while((pos=value.indexOf('\n',pos))!==-1)starts.push(++pos);
 lineCache.set(editor,{revision,length:value.length,starts});return starts;
}
function lineAt(starts,offset){let low=0,high=starts.length;while(low<high){const mid=(low+high)>>>1;if(starts[mid]<=offset)low=mid+1;else high=mid;}return Math.max(0,low-1);}
app.updateStatus=()=>{
 if(statusFrame)return;statusFrame=requestAnimationFrame(()=>{statusFrame=null;
  $('document-status').hidden=!state.settings.status_bar;if(!state.settings.status_bar)return;
  if(state.image){app.imageStatus?.();return;}const editor=$('text');if(!editor)return;
  const starts=editorLines(editor),line=lineAt(starts,editor.selectionStart),column=Array.from(editor.value.slice(starts[line],editor.selectionStart)).length+1;
  $('cursor-status').textContent=state.editing?'Ln '+(line+1)+', Col '+column:'Read mode';
  $('format-status').textContent=(state.encoding||'UTF-8')+' · '+(state.lineEnding||'LF')+(state.readOnly?' · Read-only':'');
 });
};
app.goToLine=()=>{if(state.readOnly||state.image){app.note('Line navigation is available for editable text documents.');return;}app.toggleEdit(true);const starts=editorLines($('text'));$('line-number').max=starts.length;$('line-number').value=lineAt(starts,$('text').selectionStart)+1;$('line-dialog').showModal();$('line-number').select();};
$('line-form').onsubmit=e=>{e.preventDefault();const starts=editorLines($('text')),line=Math.max(1,Math.min(Number($('line-number').value)||1,starts.length)),pos=starts[line-1];$('line-dialog').close();revealSelection($('text'),pos,pos);app.updateStatus();};
$('line-cancel').onclick=()=>$('line-dialog').close();$('cursor-status').onclick=()=>app.goToLine();
document.addEventListener('selectionchange',()=>{if(document.activeElement===$('text'))app.updateStatus();});
