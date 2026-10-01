// Eight-player layout and long-clue checks, using the real room protocol.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8091';
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox']});
  const page=await browser.newPage({viewport:{width:390,height:844},deviceScaleFactor:3});
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.goto(base);await page.waitForFunction(()=>!document.getElementById('loading'));
  const code=await page.evaluate(async()=>{
    const peers=[];let code;
    for(let i=0;i<8;i++){
      const ws=new WebSocket(`${location.protocol==='https:'?'wss:':'ws:'}//${location.host}/ws`);peers.push(ws);
      await new Promise(resolve=>ws.onopen=resolve);
      const welcome=new Promise(resolve=>ws.addEventListener('message',event=>{const message=JSON.parse(event.data);if(message.type==='welcome')resolve(message.session);},{once:true}));
      ws.addEventListener('message',event=>{const message=JSON.parse(event.data);if(message.type==='state')ws.room=message.room;});
      ws.send(JSON.stringify(i?{type:'join',name:`Player ${i}`,room:code}:{type:'create',game:'reverie',name:'Alex'}));
      const session=await welcome;
      if(i===0){code=session.room;localStorage.setItem('jarcade.online.v1',JSON.stringify({name:'Alex',sessions:[session]}));}
      await new Promise(resolve=>setTimeout(resolve,80));
    }
    for(let i=1;i<8;i++){const ws=peers[i];ws.send(JSON.stringify({type:'play',revision:ws.room.epoch,command:{kind:'ready',move:true}}));await new Promise(resolve=>setTimeout(resolve,80));}
    const host=peers[0];host.send(JSON.stringify({type:'play',revision:host.room.epoch,command:{kind:'start'}}));
    await new Promise(resolve=>setTimeout(resolve,150));
    host.send(JSON.stringify({type:'play',revision:host.room.epoch,command:{kind:'reverie',move:{type:'story',card:host.room.reverie.hand[0],clue:'A'.repeat(160)}}}));
    await new Promise(resolve=>setTimeout(resolve,100));return code;
  });
  await page.goto(`${base}/?game=reverie&room=${code}`);await page.waitForFunction(()=>!document.getElementById('loading'));
  await page.mouse.click(190,645);
  await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Round 1'));
  await page.waitForTimeout(1000);
  for(const[width,height]of[[390,844],[568,320],[280,360],[768,1024],[1440,900]]){
    await page.setViewportSize({width,height});await page.waitForTimeout(200);
    await page.screenshot({path:`/tmp/jarcade-eight-${width}x${height}.png`});
  }
  await page.setViewportSize({width:568,height:320});await page.waitForTimeout(150);
  await page.mouse.click(110,248);await page.waitForTimeout(100);
  await page.screenshot({path:'/tmp/jarcade-long-clue.png'});
  await page.keyboard.press('Escape');await page.waitForTimeout(100);
  assert((await page.locator('canvas').getAttribute('aria-label')).includes('Round 1'));
  assert.deepEqual(errors,[]);
  await browser.close();console.log('Eight-player table at five sizes; 160-character clue expansion and safe Escape passed.');
})().catch(error=>{console.error(error);process.exit(1);});
