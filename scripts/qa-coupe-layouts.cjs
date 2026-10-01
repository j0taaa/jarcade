// Crowded table, real actions, scroll reachability and saver/retina checks.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8091';
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const context=await browser.newContext({viewport:{width:390,height:844},deviceScaleFactor:3,hasTouch:true});
 const controller=await context.newPage();await controller.goto(base);
 await controller.waitForFunction(()=>!document.getElementById('loading'));
 await controller.evaluate(async()=>{
  const peers=[];window.peers=peers;let code;
  for(let i=0;i<6;i++){
   const ws=new WebSocket(`${location.protocol==='https:'?'wss:':'ws:'}//${location.host}/ws`);peers.push(ws);
   await new Promise(resolve=>ws.onopen=resolve);
   const welcome=new Promise(resolve=>ws.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.type==='welcome')resolve(m.session);},{once:true}));
   ws.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.type==='state')ws.room=m.room;});
   ws.send(JSON.stringify(i?{type:'join',room:code,name:`Very long player name ${i}`}:{type:'create',game:'court',name:'Ariadne'}));
   const session=await welcome;
   if(!i){code=session.room;localStorage.setItem('jarcade.online.v1',JSON.stringify({name:'Ariadne',sessions:[session]}));}
   await new Promise(resolve=>setTimeout(resolve,70));
  }
  for(let i=1;i<6;i++){const ws=peers[i];ws.send(JSON.stringify({type:'play',revision:ws.room.epoch,command:{kind:'ready',move:true}}));await new Promise(resolve=>setTimeout(resolve,70));}
  peers[0].send(JSON.stringify({type:'play',revision:peers[0].room.epoch,command:{kind:'start'}}));
 });
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{const original=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>original.call(window,t=>{window.__frames++;cb(t);});});
 await page.goto(`${base}/?game=court`);await page.waitForFunction(()=>!document.getElementById('loading'));
 await page.mouse.click(180,645);await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Actions: Income'));
 for(const saver of [false,true]) {
  await page.evaluate(saver=>localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`),saver);
  await page.reload();await page.waitForFunction(()=>!document.getElementById('loading'));await page.mouse.click(180,645);
  await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Actions: Income'));
  for(const [width,height]of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]){
   await page.setViewportSize({width,height});await page.waitForTimeout(120);
   await page.screenshot({path:`/tmp/jarcade-coupe-table-${width}x${height}-${saver?'black':'white'}.png`});
   assert.deepEqual(await page.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[width*3,height*3]);
  }
  await page.setViewportSize({width:390,height:844});await page.waitForTimeout(100);
 }
 // A drag over the action grid must never submit an action.
 await page.mouse.move(100,650);await page.mouse.down();await page.mouse.move(100,570,{steps:8});await page.mouse.up();await page.waitForTimeout(100);
 assert((await page.locator('canvas').getAttribute('aria-label')).includes('Actions: Income'));
 await page.mouse.move(180,500);await page.mouse.wheel(0,-10000);await page.waitForTimeout(100);
 await page.mouse.click(100,652);
 await page.waitForFunction(()=>!document.querySelector('canvas').getAttribute('aria-label').includes('Actions: Income'));
 assert.equal(await controller.evaluate(()=>peers[1].room.court.phase),'response');
 // On a tiny display the last control remains reachable, and Stay preserves the table.
 await page.setViewportSize({width:280,height:360});await page.waitForTimeout(100);
 await page.mouse.move(130,200);await page.mouse.wheel(0,10000);await page.waitForTimeout(150);
 await page.screenshot({path:'/tmp/jarcade-coupe-scrolled.png'});
 await page.mouse.click(140,312);await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Leave this table?'));
 await page.screenshot({path:'/tmp/jarcade-coupe-leave.png'});
 await page.mouse.click(76,262);await page.waitForFunction(()=>!document.querySelector('canvas').getAttribute('aria-label').includes('Leave this table?'));
 const frames=await page.evaluate(()=>window.__frames);await page.waitForTimeout(350);
 assert((await page.evaluate(()=>window.__frames))-frames<3);
 assert.deepEqual(errors,[]);
 await browser.close();console.log('Six-player Coupe: responsive/saver/retina, drag protection, real Tax action and idle rendering passed.');
})().catch(e=>{console.error(e);process.exit(1)});
