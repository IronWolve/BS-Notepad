// Images stay in the document workspace. Only their view state is kept between tabs.
const imageViews=new Map();let imageContext=null,imagePan=null;
const imageBytes=bytes=>bytes>=1048576?(bytes/1048576).toFixed(1)+' MB':bytes>=1024?Math.round(bytes/1024)+' KB':bytes+' B';
function rememberImage(){if(imageContext)imageViews.set(imageContext.tab,{scale:imageContext.scale,fit:imageContext.fit,loupe:imageContext.loupe,left:$('image-viewport').scrollLeft,top:$('image-viewport').scrollTop});}
function hideLens(){$('image-lens').hidden=true;}
app.pruneImages=live=>{for(const id of imageViews.keys())if(!live.has(id))imageViews.delete(id);if(imageContext&&!live.has(imageContext.tab)){imageContext=null;hideLens();}};
app.imageStatus=()=>{
 const ctx=imageContext;if(!ctx)return;
 $('document-status').hidden=!state.settings.status_bar;
 $('cursor-status').textContent=ctx.loaded?ctx.width.toLocaleString()+' × '+ctx.height.toLocaleString():'Image';
 $('cursor-status').disabled=true;$('cursor-status').title='Image dimensions';
 $('format-status').textContent=ctx.info.format+' · '+imageBytes(ctx.info.bytes)+' · View only';
 $('image-percent').textContent=ctx.loaded?Math.round(ctx.scale*100)+'%':'—';
 for(const id of ['image-minus','image-plus','image-fit','image-actual','image-magnify'])$(id).disabled=!ctx.loaded;
 $('image-fit').setAttribute('aria-pressed',String(ctx.fit));$('image-magnify').setAttribute('aria-pressed',String(ctx.loupe));
};
app.syncImageControls=()=>{
 const image=!!state.image;
 for(const id of ['b-view','b-find','b-map'])$(id).disabled=image;
 $('b-edit').disabled=!!state.readOnly;$('b-save').disabled=$('b-saveas').disabled=!!state.readOnly;
 $('replace-one').disabled=$('replace-all').disabled=!!state.readOnly;
 $('minimap').hidden=image||!state.settings.minimap;
 $('cursor-status').disabled=image;
 if(!image)$('cursor-status').title='Go to line ('+shortcutLabel('Ctrl+G')+')';
};
app.imageLayout=(keepCenter=false)=>{
 const ctx=imageContext,vp=$('image-viewport');if(!ctx?.loaded||!vp.clientWidth||!vp.clientHeight)return;
 hideLens();const viewport=vp.getBoundingClientRect(),old=ctx.node.getBoundingClientRect(),previous=old.width/ctx.width||ctx.scale||1;
 const x=(viewport.left+vp.clientWidth/2-old.left)/previous,y=(viewport.top+vp.clientHeight/2-old.top)/previous;
 if(ctx.fit)ctx.scale=Math.max(.001,Math.min(1,(vp.clientWidth-52)/ctx.width,(vp.clientHeight-52)/ctx.height));
 ctx.node.style.width=(ctx.width*ctx.scale)+'px';ctx.node.style.height=(ctx.height*ctx.scale)+'px';
 if(ctx.fit){vp.scrollLeft=0;vp.scrollTop=0;}
 else if(keepCenter){const next=ctx.node.getBoundingClientRect();vp.scrollLeft+=next.left+x*ctx.scale-(viewport.left+vp.clientWidth/2);vp.scrollTop+=next.top+y*ctx.scale-(viewport.top+vp.clientHeight/2);}
 app.imageStatus();rememberImage();
};
app.setImageZoom=value=>{if(!imageContext?.loaded||!Number.isFinite(value))return;const ctx=imageContext;ctx.fit=false;ctx.scale=Math.max(.01,Math.min(8,value));app.imageLayout(true);};
app.zoomImageStep=step=>{if(imageContext?.loaded)app.setImageZoom(imageContext.scale*Math.pow(1.25,step));};
app.fitImage=()=>{if(imageContext){imageContext.fit=true;app.imageLayout();}};
function showLens(event){
 const ctx=imageContext;if(!ctx?.loaded||!ctx.loupe||imagePan)return;
 const rect=ctx.node.getBoundingClientRect(),vp=$('image-viewport').getBoundingClientRect();
 if(event.clientX<rect.left||event.clientX>rect.right||event.clientY<rect.top||event.clientY>rect.bottom){hideLens();return;}
 const radius=Math.min(92,Math.max(24,(vp.width-24)/2),Math.max(24,(vp.height-24)/2)),factor=Math.max(1,ctx.scale*2.5);
 const x=(event.clientX-rect.left)/rect.width*ctx.width,y=(event.clientY-rect.top)/rect.height*ctx.height;
 const lens=$('image-lens');lens.style.width=lens.style.height=(radius*2)+'px';
 lens.style.left=Math.max(vp.left+8,Math.min(event.clientX-radius,vp.right-radius*2-8))+'px';lens.style.top=Math.max(vp.top+8,Math.min(event.clientY-radius,vp.bottom-radius*2-8))+'px';
 lens.style.backgroundImage='url('+JSON.stringify(ctx.node.src)+')';lens.style.backgroundSize=(ctx.width*factor)+'px '+(ctx.height*factor)+'px';lens.style.backgroundPosition=(radius-x*factor)+'px '+(radius-y*factor)+'px';lens.hidden=false;
}
app.toggleMagnifier=()=>{if(!imageContext?.loaded)return;imageContext.loupe=!imageContext.loupe;hideLens();app.imageStatus();rememberImage();};
app.setImage=(info,tab)=>{
 state.image=info||null;$('image-viewer').hidden=!info;app.syncImageControls();
 if(info&&imageContext?.tab===tab&&imageContext.info.url===info.url)return;
 rememberImage();hideLens();$('image-lens').style.backgroundImage='none';$('image-board').replaceChildren();imageContext=null;imagePan=null;
 if(!info)return;
 app.showFind(false);const saved=imageViews.get(tab),node=new Image(),paper=document.createElement('div');paper.className='image-paper';paper.appendChild(node);$('image-board').appendChild(paper);
 const ctx={tab,info,node,width:0,height:0,scale:saved?.scale||1,fit:saved?.fit??true,loupe:saved?.loupe??false,loaded:false};imageContext=ctx;
 node.alt=state.tabs?.find(t=>t.id===tab)?.name||'Open image';node.draggable=false;node.decoding='async';paper.hidden=true;
 $('image-message').hidden=false;$('image-message').textContent='Loading image…';app.imageStatus();
 node.onload=()=>{if(imageContext!==ctx)return;ctx.width=node.naturalWidth;ctx.height=node.naturalHeight;if(!ctx.width||!ctx.height){node.onerror();return;}ctx.loaded=true;paper.hidden=false;$('image-message').hidden=true;app.imageLayout();if(saved&&!ctx.fit){$('image-viewport').scrollLeft=saved.left;$('image-viewport').scrollTop=saved.top;}};
 node.onerror=()=>{if(imageContext!==ctx)return;ctx.loaded=false;paper.hidden=true;$('image-message').textContent='This image could not be displayed. It may be damaged or unsupported by this system.';$('image-message').hidden=false;app.imageStatus();};
 node.onpointermove=showLens;node.onpointerleave=hideLens;node.src=info.url;
};
$('image-minus').onclick=()=>app.zoomImageStep(-1);$('image-plus').onclick=()=>app.zoomImageStep(1);
$('image-fit').onclick=app.fitImage;$('image-actual').onclick=()=>app.setImageZoom(1);$('image-magnify').onclick=app.toggleMagnifier;
$('image-viewport').onscroll=()=>{hideLens();rememberImage();};
$('image-viewport').ondblclick=()=>{if(imageContext?.fit)app.setImageZoom(1);else app.fitImage();};
$('image-viewport').onkeydown=e=>{if(e.ctrlKey||e.metaKey||e.altKey)return;if(e.key.toLowerCase()==='f'){e.preventDefault();app.fitImage();}else if(e.key.toLowerCase()==='m'){e.preventDefault();app.toggleMagnifier();if(imageContext?.loupe){const rect=imageContext.node.getBoundingClientRect(),vp=$('image-viewport').getBoundingClientRect();showLens({clientX:Math.max(rect.left,Math.min(rect.right,vp.left+vp.width/2)),clientY:Math.max(rect.top,Math.min(rect.bottom,vp.top+vp.height/2))});}}};
$('image-viewport').onpointerdown=e=>{const vp=$('image-viewport');if(e.button!==0||!imageContext?.loaded||vp.scrollWidth<=vp.clientWidth&&vp.scrollHeight<=vp.clientHeight)return;e.preventDefault();vp.focus({preventScroll:true});imagePan={id:e.pointerId,x:e.clientX,y:e.clientY,left:vp.scrollLeft,top:vp.scrollTop};vp.setPointerCapture(e.pointerId);vp.classList.add('panning');hideLens();};
$('image-viewport').onpointermove=e=>{if(imagePan){$('image-viewport').scrollLeft=imagePan.left-(e.clientX-imagePan.x);$('image-viewport').scrollTop=imagePan.top-(e.clientY-imagePan.y);}};
$('image-viewport').onpointerup=$('image-viewport').onpointercancel=e=>{imagePan=null;$('image-viewport').classList.remove('panning');if($('image-viewport').hasPointerCapture(e.pointerId))$('image-viewport').releasePointerCapture(e.pointerId);};
new ResizeObserver(()=>{if(state.image)app.imageLayout();}).observe($('image-viewport'));
