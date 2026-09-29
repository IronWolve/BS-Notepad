// Pure edits keep offsets explicit; one native edit preserves the undo step.
const editingTools={
 indent(text,start,end,width,tabs,outdent){
  const unit=tabs?'\t':' '.repeat(width);
  if(start===end&&!outdent)return {start,end,text:unit,selectionStart:start+unit.length,selectionEnd:start+unit.length};
  const first=(start>0?text.lastIndexOf('\n',start-1)+1:0);
  const selectedEnd=end>start&&text[end-1]==='\n'?end-1:end;
  let last=text.indexOf('\n',selectedEnd);if(last<0)last=text.length;
  const lines=text.slice(first,last).split('\n'),changes=[];let position=first;
  const result=lines.map(line=>{
   const removed=outdent?(line.startsWith('\t')?1:(line.match(/^ */)?.[0].length||0)>0?Math.min(width,line.match(/^ */)[0].length):0):0;
   changes.push({at:position,remove:removed,add:outdent?0:unit.length});position+=line.length+1;
   return outdent?line.slice(removed):unit+line;
  }).join('\n');
  const map=offset=>{let delta=0;for(const change of changes){if(offset<change.at)break;delta+=change.add-Math.min(change.remove,offset-change.at);}return Math.max(first,offset+delta);};
  return {start:first,end:last,text:result,selectionStart:map(start),selectionEnd:map(end)};
 },
 newline(text,start,end,markdown){
  if(start!==end)return null;
  const first=(start>0?text.lastIndexOf('\n',start-1)+1:0),before=text.slice(first,start),indent=before.match(/^[\t ]*/)[0];
  if(markdown){
   let fence=null;
   for(const line of text.slice(0,first).split('\n')){const m=line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);if(m){if(!fence)fence={char:m[1][0],length:m[1].length};else if(m[1][0]===fence.char&&m[1].length>=fence.length&&!m[2].trim())fence=null;}}
   if(!fence){
    const item=before.match(/^([\t ]*)([-+*]|\d+[.)])([\t ]+)(?:\[([ xX])\]([\t ]+))?/);
    const quote=before.match(/^([\t ]*)(>)([\t ]?)/);
    const match=item||quote;
    if(match){
     const rest=before.slice(match[0].length),lineEnd=text.indexOf('\n',start),tail=text.slice(start,lineEnd<0?text.length:lineEnd);
     if(!rest.trim()&&!tail.trim())return {start:first,end:start,text:match[1],selectionStart:first+match[1].length,selectionEnd:first+match[1].length};
     let marker=match[2];if(/^\d/.test(marker)){const number=Number(marker.slice(0,-1));if(!Number.isSafeInteger(number)||number>=Number.MAX_SAFE_INTEGER)return null;marker=String(number+1)+marker.at(-1);}
     const prefix=match[1]+marker+(match[3]||' ')+(item&&match[4]!==undefined?'[ ]'+match[5]:'');
     return {start,end,text:'\n'+prefix,selectionStart:start+1+prefix.length,selectionEnd:start+1+prefix.length};
    }
   }
  }
  if(!indent)return null;
  return {start,end,text:'\n'+indent,selectionStart:start+1+indent.length,selectionEnd:start+1+indent.length};
 }
};
app.editingKey=(event,node)=>{
 if(primaryModifier(event)&&commandKey(event)==='m'&&(state.platform!=='macos'||event.shiftKey)){event.preventDefault();state.tabNavigation=!state.tabNavigation;app.note(state.tabNavigation?'Tab moves focus. Press the same shortcut to indent again.':'Tab indents text.');return;}
 if(event.key==='Tab'&&state.tabNavigation)return;
 if(event.isComposing||node.readOnly||event.ctrlKey||event.metaKey||event.altKey)return;
 let edit;
 if(event.key==='Tab')edit=editingTools.indent(node.value,node.selectionStart,node.selectionEnd,state.settings.tab_size||4,state.settings.tab_style==='tabs',event.shiftKey);
 else if(event.key==='Enter'&&!event.shiftKey){const markdown=state.settings.continue_lists!==false&&(!state.path||/\.(md|markdown|mdown|mkd|mkdn|mdx)$/i.test(state.path));edit=editingTools.newline(node.value,node.selectionStart,node.selectionEnd,markdown);}
 if(!edit)return;
 event.preventDefault();node.setSelectionRange(edit.start,edit.end);
 if(!document.execCommand('insertText',false,edit.text)){node.setRangeText(edit.text,edit.start,edit.end,'end');node.dispatchEvent(new Event('input',{bubbles:true}));}
 node.setSelectionRange(edit.selectionStart,edit.selectionEnd);app.updateStatus?.();
};
