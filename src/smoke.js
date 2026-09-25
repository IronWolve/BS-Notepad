// Runs only under EXIT_WHEN_READY, on a disposable virtual-display copy.
(async()=>{
 const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
 const until=async(test,label)=>{for(let n=0;n<120;n++){if(test())return;await sleep(25);}throw Error(label);};
 const check=(condition,label)=>{if(!condition)throw Error(label);};
 try {
  await until(()=>state.tabs?.length===1 && $('text')?.value.length>0,'initial document');
  check(state.name==='BS Notepad','display name');
  check(!!document.querySelector('#pane-files > div'),'file tree');
  const original=state.activeTab,source=$('text').value;
  if(source.includes('rendering-smoke')) {
    const span=[...$('article').querySelectorAll('span')].find(node=>node.textContent==='rendering-smoke');
    check(!!span&&getComputedStyle(span).color==='rgb(34, 197, 94)','embedded HTML color');
    check($('article').querySelector('h2 strong')?.textContent==='Formatted','heading formatting');
  }
  $('tab-new').click();
  await until(()=>state.tabs.length===2&&state.activeTab!==original,'new tab');
  const draftId=state.activeTab;
  const draft=Array.from({length:350},(_,i)=>'Draft line '+i+' retained in this tab.').join('\n');
  $('text').value=draft;$('text').dispatchEvent(new Event('input'));
  send({cmd:'activateTab',id:original});
  await until(()=>state.activeTab===original&&$('text').value===source,'original tab restored');
  send({cmd:'activateTab',id:draftId});
  await until(()=>state.activeTab===draftId&&$('text').value===draft,'draft restored');
  check(state.dirty&&state.editing,'draft editing state');
  app.help(true);
  await until(()=>$('help-logo').complete&&$('help-logo').naturalWidth>0,'embedded logo');
  check($('help-github').href===state.githubUrl&&state.githubUrl.startsWith('https://'),'help link');
  app.help(false);
  send({cmd:'setting',key:'theme',value:'mist'});
  await until(()=>state.settings.theme==='mist','theme applies');
  check($('text').value===draft,'theme retains draft');
  app.chooseTheme();
  check(!$('choice-popup').hidden&&$('choice-list').children.length>=12,'themed dropdown');
  closeChoices();
  send({cmd:'setting',key:'minimap',value:true});
  await until(()=>!$('minimap').hidden&&$('text').scrollHeight>$('text').clientHeight,'map layout');
  await sleep(150);
  $('map-viewport').dispatchEvent(new KeyboardEvent('keydown',{key:'PageDown',bubbles:true}));
  check($('text').scrollTop>0,'map scroll');
  window.ipc.postMessage(JSON.stringify({cmd:'smokeReady',ok:true}));
 } catch(error) {
  window.ipc.postMessage(JSON.stringify({cmd:'smokeReady',ok:false,error:String(error)}));
 }
})();
