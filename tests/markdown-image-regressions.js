(async()=>{
 const checks=[],wait=()=>new Promise(r=>setTimeout(r,100)),check=(v,label)=>{if(!v)throw Error(label);checks.push(label)};
 const tab=state.activeTab,source=$('text').value,zoom=state.settings.zoom,readonly=state.readOnly;
 let original=$('article').querySelector('img');check(!!original?.dataset.zoomable,'Rendered Markdown pictures are wired for zoom');
 check(original.tabIndex===0&&original.getAttribute('role')==='button','Embedded pictures are keyboard accessible');
 const large='data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800"><rect width="1200" height="800" fill="#39767d"/><circle cx="600" cy="400" r="230" fill="#e6bd78"/></svg>');
 original.src=large;const before=document.createElement('p'),after=document.createElement('p');before.style.height='900px';after.style.height='900px';original.before(before);original.after(after);
 for(let i=0;i<30&&(!original.complete||original.naturalWidth!==1200);i++)await wait();check(original.naturalWidth===1200,'Embedded image fixture finished decoding');$('doc').scrollTop=650;const scroll=$('doc').scrollTop;
 original.click();for(let i=0;i<30&&!imageContext?.loaded;i++)await wait();
 check(state.activeTab===tab&&state.embeddedImage?.tab===tab&&imageContext?.loaded,'Click opens the embedded image inside the same document tab');
 check(!$('image-back').hidden&&$('image-viewer').classList.contains('embedded-preview'),'Preview provides an in-app Back control');
 const scale=imageContext.scale;$('image-plus').click();check(imageContext.scale>scale,'Embedded image zoom enlarges the picture');
 $('image-magnify').click();const rect=imageContext.node.getBoundingClientRect();imageContext.node.dispatchEvent(new PointerEvent('pointermove',{clientX:rect.left+rect.width/2,clientY:rect.top+rect.height/2,bubbles:true}));check(!$('image-lens').hidden,'Embedded images support the magnifying glass');
 check(state.settings.zoom===zoom&&state.readOnly===readonly&&$('text').value===source,'Picture inspection preserves text, permissions and text zoom');
 const activeImage=imageContext;app.setDocument({tab,revision:0,path:'/workspace/audit.md',editing:false,readOnly:false,themeId:state.settings.theme,scroll:state.embeddedImage.fraction,html:'<p style="height:900px"></p><p><img src="'+large+'" alt="Embedded picture"></p><p style="height:900px"></p>'});
 check(state.embeddedImage&&imageContext===activeImage,'Same-document redraw preserves the image preview');original=$('article').querySelector('img');
 $('image-back').click();await wait();check(!state.image&&!state.embeddedImage&&$('image-viewer').hidden&&!$('doc').inert,'Back returns to the Markdown reader');
 check(Math.abs($('doc').scrollTop-scroll)<2&&document.activeElement===original,'Back restores the reading position and image focus');
 original.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}));check(!!state.embeddedImage,'Enter opens a focused Markdown picture');document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));check(!state.embeddedImage&&!state.image,'Escape returns to the document');
 const link=document.createElement('a');link.href='https://example.invalid/image-link';link.dataset.external='1';original.replaceWith(link);link.append(original);app.wireLinks();let followed=0;link.onclick=e=>{e.preventDefault();followed++;};
 original.click();check(followed===0&&!!state.embeddedImage,'Clicking a linked picture zooms without following the link');app.closeEmbeddedImage();original.dispatchEvent(new MouseEvent('click',{ctrlKey:true,bubbles:true,cancelable:true}));check(followed===1&&!state.embeddedImage,'Modified click preserves the original picture link action');
 original.click();app.setTabs({active:99,tabs:[...state.tabs,{id:99,name:'other.md',path:'/workspace/other.md',dirty:false}]});app.beginDocument({tab:99,revision:0,generation:7,themeId:state.settings.theme,path:'/workspace/other.md',editing:false,readOnly:false});app.setEditorText({tab:99,revision:0,text:'Other note'});app.finishDocument({tab:99,revision:0,generation:7,html:'<p>Other note</p>',outline:[]});
 check(!state.embeddedImage&&!state.image&&state.activeTab===99,'Switching tabs clears the temporary picture preview');check(editorNodes.get(tab).value===source,'Switching away preserves the original document buffer');
 return checks;
})()
