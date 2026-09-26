window.messages=[];window.fixtureWrites=[];window.nativeTabs=[{id:1,name:'audit.md',path:'/workspace/audit.md',source:FIXTURE_SOURCE,html:FIXTURE_RENDER,dirty:false,editing:false,revision:0}];
window.fixtureSettings=FIXTURE_SETTINGS;window.fixtureThemes=FIXTURE_THEMES;let fixtureActive=1,nextFixtureId=2,fixturePreview=null;
window.emitDoc=()=>{const d=nativeTabs.find(d=>d.id===fixtureActive);app.setTabs({active:d.id,tabs:nativeTabs});app.setDocument({...d,tab:d.id,themeId:fixturePreview||fixtureSettings.theme,outline:[],encoding:'UTF-8',lineEnding:'LF'});app.setEditorText({tab:d.id,text:d.source,revision:d.revision});};
window.ipc={postMessage:raw=>{const m=JSON.parse(raw);messages.push(m);setTimeout(()=>{
 const d=nativeTabs.find(d=>d.id===(m.fromTab||fixtureActive));if(d&&m.view)Object.assign(d,m.view);
 if(m.cmd==='ready'){app.init({name:FIXTURE_NAME,version:FIXTURE_VERSION,logoUrl:'data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2" fill="gray"/></svg>',githubUrl:'https://example.invalid',trayAvailable:true,settings:{...fixtureSettings},defaults:{...fixtureSettings},themes:fixtureThemes,theme:fixtureThemes.find(t=>t.id===fixtureSettings.theme),fonts:[]});app.setTree({dir:'/workspace',parent:true,entries:[]});emitDoc();}
 if(m.cmd==='setting'){fixtureSettings[m.key]=m.value;fixtureWrites.push(m);if(m.key==='theme')fixturePreview=null;app.applySettings({...fixtureSettings});app.applyTheme(fixtureThemes.find(t=>t.id===(fixturePreview||fixtureSettings.theme)));if(['theme','text_contrast'].includes(m.key))emitDoc();}
 if(m.cmd==='previewTheme'){fixturePreview=m.theme;app.themePreview({id:m.theme,token:m.token,theme:fixtureThemes.find(t=>t.id===m.theme)});emitDoc();}
 if(m.cmd==='edit'&&d){d.source=m.text;d.revision=m.revision;d.dirty=true;}
 if(m.cmd==='editPatch'&&d){d.source=d.source.slice(0,m.start)+m.insert+d.source.slice(m.end);d.revision=m.revision;d.dirty=true;}
 if(m.cmd==='save'&&d&&typeof m.text==='string'){d.source=m.text;d.dirty=false;}
 if(m.cmd==='new'){const d={id:nextFixtureId++,name:'Untitled',path:'',source:'',html:'',dirty:false,editing:true,revision:0};nativeTabs.push(d);fixtureActive=d.id;emitDoc();}
 if(m.cmd==='activateTab'){fixtureActive=m.id;emitDoc();}
 if(m.cmd==='preview')emitDoc();
},0);}};
