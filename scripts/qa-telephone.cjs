// Real touch sketchbook + independent private seats on a disposable room server.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {mkdirSync}=require('node:fs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const artifacts='/tmp/jarcade-telephone-qa';mkdirSync(artifacts,{recursive:true});
const pause=ms=>new Promise(r=>setTimeout(r,ms));
const clients=[];let browser,page;
async function until(fn,label){const start=Date.now();while(!fn()){assert(Date.now()-start<15000,`Timeout ${label}`);await pause(20);}}
async function seat(message){const socket=new WebSocket(base.replace(/^http/,'ws')+'/ws');const c={socket,room:null,session:null,errors:[]};clients.push(c);socket.addEventListener('message',e=>{const d=JSON.parse(e.data);if(d.type==='state')c.room=d.room;if(d.type==='welcome')c.session=d.session;if(d.type==='error')c.errors.push(d.message);});await until(()=>socket.readyState===WebSocket.OPEN,'socket');socket.send(JSON.stringify(message));await until(()=>c.room,'seat');return c;}
async function command(c,move,kind='telephone'){const rev=c.room.revision;c.socket.send(JSON.stringify({type:'play',revision:c.room.epoch,command:{kind,move}}));await until(()=>c.room.revision>rev||c.errors.length,'command');assert.deepEqual(c.errors,[]);await pause(80);}
const drawing={strokes:[{color:4,width:12,points:[[100,600],[500,100],[900,600],[100,600]]}]};
const saved=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.online.v1')).telephone_draft);
async function click(x,y){await page.mouse.click(x,y);await page.waitForTimeout(160);}
async function seal(height=844){await click(195,height-40);await page.waitForTimeout(150);await click(280,(height+66-16)/2+46);}
(async()=>{
  browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
  const errors=[];
  async function context(w=390,h=844,saver=false,session=null){
    const ctx=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
    await ctx.addInitScript(({saver,session})=>{
      localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
      if(session)localStorage.setItem('jarcade.online.v1',JSON.stringify({name:'Guest',sessions:[session]}));
      const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});
      const Socket=WebSocket;window.WebSocket=class extends Socket{constructor(...a){super(...a);window.__socket=this;this.addEventListener('message',e=>{const d=JSON.parse(e.data);if(d.type==='state')window.__room=d.room;if(d.type==='welcome')window.__session=d.session;if(d.type==='error')(window.__errors||=[]).push(d.message);});}};
    },{saver,session});
    const p=await ctx.newPage();p.on('pageerror',e=>errors.push(e.message));await p.goto(base+'/games/drawing-telephone');await p.waitForFunction(()=>!document.getElementById('loading'));return {ctx,p};
  }
  async function idle(p){await p.waitForTimeout(700);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(250);assert.equal(await p.evaluate(()=>window.__frames),frames,'Idle drawing/room screens must sleep with FPS enabled');}
  const host=await context();page=host.p;
  await click(195,379);await page.locator('#jarcade-text-editor').fill('Mila');await page.locator('#jarcade-text-editor').press('Enter');await click(195,444);await page.waitForFunction(()=>!!window.__room);
  const code=await page.evaluate(()=>window.__room.code);
  const bots=await Promise.all(['Rio','Nora'].map(name=>seat({type:'join',room:code,name})));
  await page.waitForFunction(()=>window.__room.members.length===3);
  await page.screenshot({path:artifacts+'/lobby-phone.png',scale:'css'});
  for(const bot of bots)await command(bot,true,'ready');
  await click(195,492);await page.waitForFunction(()=>window.__room?.telephone?.stage===0);
  await idle(page);await page.screenshot({path:artifacts+'/prompt-phone.png',scale:'css'});
  await click(190,740);await page.locator('#jarcade-text-editor').fill('A cat on the moon');await page.locator('#jarcade-text-editor').press('Enter');
  await page.waitForFunction(()=>JSON.parse(localStorage.getItem('jarcade.online.v1')).telephone_draft?.text==='A cat on the moon');
  assert.equal((await saved()).text,'A cat on the moon');
  await seal();await page.waitForFunction(()=>window.__room.telephone.submitted[0]);
  assert.equal(await page.evaluate(()=>window.__room.telephone.task),null);
  for(let i=0;i<bots.length;i++){
    assert.equal(bots[i].room.telephone.task,null);
    await command(bots[i],{action:'text',text:['A flying strawberry','A penguin wearing sunglasses'][i]});
  }
  await page.waitForFunction(()=>window.__room.telephone.stage===1);
  assert.equal(await page.evaluate(()=>window.__room.telephone.task.type),'text');
  const task=await page.evaluate(()=>window.__room.telephone.task.value);assert(['A flying strawberry','A penguin wearing sunglasses'].includes(task));
  await idle(page);
  // Draw with physical high-DPI touch coordinates, then undo, redo and erase.
  await click(84,690);const cdp=await host.ctx.newCDPSession(page);
  const touch=(type,pts)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:pts});
  await touch('touchStart',[{id:1,x:130,y:375}]);await page.waitForTimeout(60);
  for(const [x,y]of [[150,350],[180,350],[200,375],[180,400],[150,400],[130,375]]){await touch('touchMove',[{id:1,x,y}]);await page.waitForTimeout(40);}
  await touch('touchEnd',[]);await page.waitForTimeout(200);
  let draft=await saved();assert.equal(draft.drawing.strokes.length,1);assert(draft.drawing.strokes[0].points.length>=6);
  assert(draft.drawing.strokes[0].points.every(([x,y])=>x>=0&&x<=1000&&y>=0&&y<=750));
  const before=draft.drawing;
  await page.keyboard.press('Control+z');await page.waitForTimeout(180);assert.equal((await saved()).drawing.strokes.length,0);
  await page.keyboard.press('Control+Shift+z');await page.waitForTimeout(180);assert.deepEqual((await saved()).drawing,before);
  await click(351,690);await touch('touchStart',[{id:1,x:170,y:350}]);await page.waitForTimeout(60);await touch('touchMove',[{id:1,x:170,y:370}]);await touch('touchEnd',[]);await page.waitForTimeout(180);
  assert.equal((await saved()).drawing.strokes.at(-1).color,7);
  await page.keyboard.press('Control+z');await page.waitForTimeout(180);assert.deepEqual((await saved()).drawing,before);
  await idle(page);await page.screenshot({path:artifacts+'/sketch-phone.png',scale:'css'});
  // The live sketchbook keeps normalized work across rotation and narrow layouts.
  for(const [w,h] of [[320,568],[568,320],[820,1180],[1440,900]]) {
    await page.setViewportSize({width:w,height:h});await idle(page);
    assert.deepEqual((await saved()).drawing,before);
    await page.screenshot({path:`${artifacts}/sketch-${w}x${h}.png`,scale:'css'});
  }
  await page.setViewportSize({width:390,height:844});await idle(page);
  // Reload and reconnect the same task; unfinished work survives.
  const session=await page.evaluate(()=>window.__session);await page.reload();await page.waitForFunction(()=>!document.getElementById('loading'));
  await click(195,645);await page.waitForFunction(()=>window.__room?.telephone?.stage===1);
  assert.equal(await page.evaluate(()=>window.__session.token),session.token);assert.deepEqual((await saved()).drawing,before);
  await page.screenshot({path:artifacts+'/reconnect-sketch.png',scale:'css'});
  await seal();await page.waitForFunction(()=>window.__room.telephone.submitted[0]);
  for(const bot of bots)await command(bot,{action:'draw',drawing});
  await page.waitForFunction(()=>window.__room.telephone.stage===2);
  assert.equal(await page.evaluate(()=>window.__room.telephone.task.type),'drawing');
  await page.screenshot({path:artifacts+'/describe-phone.png',scale:'css'});
  await click(195,740);await page.locator('#jarcade-text-editor').fill('A rocket made of fruit');await page.locator('#jarcade-text-editor').press('Enter');await seal();
  for(let i=0;i<bots.length;i++)await command(bots[i],{action:'text',text:`The final surprise ${i}`});
  await page.waitForFunction(()=>window.__room.telephone.phase==='reveal');
  await idle(page);await page.screenshot({path:artifacts+'/album-text-phone.png',scale:'css'});
  const revision=bots[0].room.revision;
  bots[0].socket.send(JSON.stringify({type:'play',revision:bots[0].room.epoch,command:{kind:'telephone',move:{action:'next'}}}));
  await until(()=>bots[0].errors.length,'nonhost album rejection');assert.match(bots[0].errors.pop(),/host/);assert.equal(bots[0].room.revision,revision);
  await click(270,804);await page.waitForFunction(()=>window.__room.telephone.step===1);
  await page.screenshot({path:artifacts+'/album-drawing-phone.png',scale:'css'});
  // Same shared album in different viewport sizes; all are crisp and event driven.
  for(const [w,h,saver]of [[320,568,false],[568,320,false],[820,1180,false],[1440,900,false],[390,844,true]]){
    const {ctx,p}=await context(w,h,saver,bots[0].session);
    const width=Math.min(w-32,1100),wide=width>=800,compact=h<650;
    const formY=66+(wide?64:compact?146:248);const target=formY+331;
    const requested=Math.ceil(Math.max(0,target-h+70)/80)*80;
    const shift=Math.min(requested,Math.max(0,(wide?510:(compact?146:248)+390)-(h-82)));
    for(let i=0;i<Math.ceil(shift/80);i++){await p.keyboard.press('ArrowDown');await p.waitForTimeout(90);}
    const x=wide?(w-width)/2+width*.54+200:w/2;
    await p.mouse.click(x,target-shift);await p.waitForFunction(()=>!!window.__room?.telephone);
    await idle(p);
    assert.deepEqual(await p.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);
    assert.equal(await p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');
    await p.screenshot({path:`${artifacts}/album-${w}x${h}-${saver}.png`,scale:'css'});
    await ctx.close();
  }
  // Reveal all nine pages and use the real host rematch action.
  for(let i=1;i<9;i++){await click(270,804);await page.waitForTimeout(100);}
  await page.waitForFunction(()=>window.__room.telephone.phase==='finished');
  await click(270,804);await page.waitForFunction(()=>!window.__room.telephone);
  await page.keyboard.press('Escape');await page.waitForFunction(()=>location.pathname==='/multiplayer');
  await page.keyboard.press('PageDown');await page.waitForTimeout(100);
  await page.screenshot({path:artifacts+'/gallery-phone.png',scale:'css'});
  await page.touchscreen.tap(280,685);await page.waitForFunction(()=>location.pathname==='/games/drawing-telephone');
  assert.deepEqual(await page.evaluate(()=>window.__errors||[]),[]);assert.deepEqual(errors,[]);
  const response=await fetch(base+'/rooms.json');assert.equal(response.status,404);
  console.log('PASS: private chains, touch vectors, eraser, undo/redo, draft reload/reconnect, shared album, host authority, rematch, high-DPI responsive layouts and sleeping FPS-on screens.');
})().catch(async e=>{if(page)await page.screenshot({path:artifacts+'/failure.png',scale:'css'}).catch(()=>{});console.error(e);process.exitCode=1;}).finally(async()=>{for(const c of clients)c.socket.close();if(browser)await browser.close();});
