(async()=>{
 const checks=[],wait=()=>new Promise(r=>setTimeout(r,100)),check=(value,label)=>{if(!value)throw Error(label);checks.push(label);};
 const textTab=state.activeTab,textSource=$('text').value,docZoom=state.settings.zoom;
 const svg='<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="1000"><defs><linearGradient id="g"><stop stop-color="#357a87"/><stop offset="1" stop-color="#172b45"/></linearGradient></defs><rect width="1600" height="1000" fill="url(#g)"/><circle cx="800" cy="500" r="240" fill="#e5bc73"/><path d="M0 900 700 250 1600 1000" fill="#d9e7db" opacity=".55"/></svg>';
 const url='data:image/svg+xml,'+encodeURIComponent(svg),image={url,format:'SVG',bytes:svg.length};
 const payload={tab:501,revision:0,generation:51,name:'landscape.svg',path:'/workspace/landscape.svg',editing:false,dirty:false,readOnly:true,image,themeId:state.settings.theme};
 app.setTabs({active:501,tabs:[...state.tabs,{id:501,name:'landscape.svg',path:payload.path,dirty:false}]});app.beginDocument(payload);app.setEditorText({tab:501,revision:0,text:''});app.finishDocument({tab:501,revision:0,generation:51,html:'',outline:[],renderKey:'image-501'});
 for(let i=0;i<40&&!imageContext?.loaded;i++)await wait();
 check(imageContext?.loaded,'An image loads inside the editor workspace');check(!$('image-viewer').hidden&&$('doc').style.display==='none','Image canvas replaces the reader without a new window');
 check(imageContext.width===1600&&imageContext.height===1000,'Natural image dimensions are shown');
 let rect=imageContext.node.getBoundingClientRect(),vp=$('image-viewport').getBoundingClientRect();check(rect.width<=vp.width&&rect.height<=vp.height,'Fit keeps the whole image visible');
 check($('b-edit').disabled&&$('b-save').disabled&&$('b-view').disabled&&$('b-find').disabled&&$('minimap').hidden,'Text-only actions and minimap stay out of image tabs');
 const before=imageContext.scale;$('image-plus').click();check(imageContext.scale>before&&!imageContext.fit,'Zoom-in control enlarges the image');check(state.settings.zoom===docZoom,'Image zoom leaves document zoom preferences unchanged');
 $('image-actual').click();check(imageContext.scale===1,'Actual size uses natural pixels');
 const scale=imageContext.scale;$('image-viewport').dispatchEvent(new WheelEvent('wheel',{ctrlKey:true,deltaY:-120,bubbles:true,cancelable:true}));check(imageContext.scale>scale,'Ctrl+wheel zooms the image');
 $('image-fit').click();check(imageContext.fit,'Fit restores the full-image view');
 $('image-magnify').click();rect=imageContext.node.getBoundingClientRect();imageContext.node.dispatchEvent(new PointerEvent('pointermove',{clientX:rect.left+rect.width/2,clientY:rect.top+rect.height/2,bubbles:true}));
 check(!$('image-lens').hidden&&$('image-magnify').getAttribute('aria-pressed')==='true','Magnifying glass enlarges the hovered area');
 imageContext.node.dispatchEvent(new PointerEvent('pointerleave'));check($('image-lens').hidden,'Lens disappears when the pointer leaves the picture');
 app.setImageZoom(2);const savedImage=imageContext;app.setImage(image,501);check(imageContext===savedImage&&imageContext.scale===2,'Theme redraws preserve image zoom and decoder state');
 app.setDocument({tab:textTab,revision:0,themeId:state.settings.theme,path:'/workspace/audit.md',editing:false,readOnly:false,html:'<p>Text document</p>'});
 check(!state.image&&$('image-viewer').hidden&&!$('b-edit').disabled&&!$('b-save').disabled,'Switching back restores text controls');check($('text').value===textSource,'Image viewing preserves the text tab contents');
 app.beginDocument(payload);app.finishDocument({tab:501,revision:0,generation:51,html:'',outline:[],renderKey:'image-501'});for(let i=0;i<40&&!imageContext?.loaded;i++)await wait();check(imageContext.scale===2&&!imageContext.fit,'Each image tab retains its own zoom');
 app.setImage({url:'data:image/png;base64,AAAA',format:'PNG',bytes:3},502);for(let i=0;i<30&&$('image-message').textContent==='Loading image…';i++)await wait();check($('image-message').textContent.includes('could not be displayed'),'Damaged images show a clear inline error');
 app.setImage(null,textTab);app.pruneImages(new Set([textTab]));check(!imageViews.has(501),'Closing an image tab releases its view state');
 return checks;
})()
