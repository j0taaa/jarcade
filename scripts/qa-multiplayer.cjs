// Real UI and authoritative WebSocket QA. Requires Playwright and a room server.
const {chromium}=require('playwright');const assert=require('node:assert/strict');
const debugPages=[];
const url=process.env.JARCADE_QA_URL||'http://127.0.0.1:8091';
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});const errors=[];
 const pages=[];
 async function page(game,name,code){const context=await browser.newContext({viewport:{width:390,height:844},deviceScaleFactor:3,hasTouch:true});const p=await context.newPage();pages.push(p);debugPages.push(p);p.on('pageerror',e=>errors.push(e.message));
  await p.addInitScript(()=>{const Native=WebSocket;window.__frames=0;const original=requestAnimationFrame;window.requestAnimationFrame=callback=>original.call(window,t=>{window.__frames++;callback(t);});window.WebSocket=class extends Native{constructor(...args){super(...args);window.__socket=this;this.addEventListener('message',event=>{const data=JSON.parse(event.data);window.__events=(window.__events||[]).concat(data.type);if(data.type==='state')window.__room=data.room;});}};});
  await p.goto(`${url}/?game=${game}${code?`&room=${code}`:''}`);await p.waitForFunction(()=>!document.getElementById('loading'));
  await p.mouse.click(180,314);await p.locator('#jarcade-text-editor').fill(name);await p.locator('#jarcade-text-editor').press('Enter');
  await p.mouse.click(code?320:180,code?451:378);await p.waitForFunction(()=>!!window.__room);return p;
 }
 const room=p=>p.evaluate(()=>window.__room);
 const click=async(p,x,y)=>{await p.mouse.click(x,y);await p.waitForTimeout(90);};
 const wait=async(p,phase)=>p.waitForFunction(phase=>window.__room?.court?.phase===phase||window.__room?.reverie?.phase===phase,phase);
 async function lobby(ps){for(let i=1;i<ps.length;i++){const r=await room(ps[i]);await click(ps[i],180,66+80+ps.length*58+12+24);await ps[i].waitForFunction(()=>window.__room.members[window.__room.you].ready);}
  await ps[0].waitForFunction(()=>window.__room.members.every(m=>m.ready));await click(ps[0],180,66+80+ps.length*58+12+60+24);await ps[0].waitForFunction(()=>window.__room.court||window.__room.reverie);
 }
 // Coupe: original vectors, a challenge, then a complete two-player match.
 const alice=await page('court','Alice');const code=(await room(alice)).code;const bob=await page('court','Bob',code);await lobby([alice,bob]);await wait(alice,'turn');
 let a=await room(alice);assert(a.court.players[0].cards.every(c=>c.role));assert(a.court.players[1].cards.every(c=>c.role===null));
 await click(alice,100,472);await wait(bob,'response');await click(bob,288,414);await wait(alice,'loss');a=await room(alice);const loser=a.court.choices.length?alice:bob;await click(loser,100,414);await wait(alice,'turn');
 await alice.screenshot({path:'/tmp/jarcade-court-game.png'});
 let steps=0;while((await room(alice)).court.phase!=='finished'){
  assert(++steps<100);const state=(await room(alice)).court;const active=state.turn===0?alice:bob;
  if(state.phase==='turn'){const r=await room(active);const v=r.court;const action=v.actions.includes('coup')?'coup':'income';const index=v.actions.indexOf(action);await click(active,index%2?288:100,414+Math.floor(index/2)*58);if(action==='coup')await click(active,100,414);}
  else if(state.phase==='loss'){const p=(await room(alice)).court.choices.length?alice:bob;await click(p,100,414);}
  else {throw new Error(`Unexpected phase ${state.phase}`);}
 }
 // Dicksit: 3-player variant, real previews/selection, scoring and full game.
 const mia=await page('reverie','Mia');const rc=(await room(mia)).code;const noah=await page('reverie','Noah',rc);const ava=await page('reverie','Ava',rc);const ps=[mia,noah,ava];await lobby(ps);await wait(mia,'story');assert.equal((await room(mia)).reverie.hand.length,7);
 async function choose(p,index,phase){const r=await room(p);const v=r.reverie;await p.mouse.move(180,500);await p.mouse.wheel(0,-10000);await p.waitForTimeout(50);
  // Derive actual gallery start: 64 + 2*34 + 8 + 98 = 238.
  const start=238;const ch=177*1.5;const y=start+Math.floor(index/2)*(ch+32+12)+60;const maxVisible=844-82;
  let offset=0;if(y>maxVisible){const count=phase==='vote'?v.table.length:v.hand.length;const maxOffset=Math.ceil(count/2)*(ch+32+12)-(844-start-82);offset=Math.min(y-450,maxOffset);await p.mouse.wheel(0,offset);await p.waitForTimeout(80);}
  await click(p,index%2?288:100,y-offset);
  await click(p,288,800);
 }
 let rounds=0;
 while((await room(mia)).reverie.phase!=='finished'){
  assert(++rounds<35);console.log(`Dicksit round ${rounds}`);let r=await room(mia);let v=r.reverie;const teller=ps[v.storyteller];
  await teller.waitForFunction(()=>document.querySelector("canvas").getAttribute("aria-label").includes("story"));await teller.waitForTimeout(150);
  await teller.mouse.click(180,200);await teller.locator('#jarcade-text-editor').fill(`A light in the dark ${rounds}`);await teller.locator('#jarcade-text-editor').press('Enter');
  const secret=(await room(teller)).reverie.hand[0];await choose(teller,0,'story');await click(teller,150,800);await wait(mia,'submit');
  for(let i=0;i<3;i++){if(i===v.storyteller)continue;await choose(ps[i],0,'submit');await choose(ps[i],1,'submit');await click(ps[i],150,800);}
  await wait(mia,'vote');r=await room(mia);assert.equal(r.reverie.table.length,5);assert(r.reverie.table.every(c=>c.owner===null&&!c.story&&!c.votes.length));
  if(rounds===1){await mia.waitForTimeout(1000);await mia.screenshot({path:'/tmp/jarcade-reverie-vote.png'});const before=await mia.evaluate(()=>window.__frames);await mia.waitForTimeout(600);assert((await mia.evaluate(()=>window.__frames))-before<4,'Idle multiplayer must stop rendering');
   // Dragging a card must not open preview, and cannot submit a vote.
   await noah.mouse.move(90,300);await noah.mouse.down();await noah.mouse.move(90,240,{steps:8});await noah.mouse.up();await noah.waitForTimeout(100);assert.equal((await room(noah)).reverie.vote,null);
   await noah.mouse.wheel(0,-2000);await noah.waitForTimeout(80);
  }
  for(let i=0;i<3;i++){if(i===v.storyteller)continue;const own=await room(ps[i]);const index=own.reverie.table.findIndex(c=>c.card===secret);await choose(ps[i],index,'vote');await click(ps[i],150,800);}
  await mia.waitForFunction(()=>['results','finished'].includes(window.__room?.reverie?.phase));
  r=await room(mia);assert(r.reverie.players[v.storyteller].gained===0);assert(r.reverie.players.filter((_,i)=>i!==v.storyteller).every(p=>p.gained===2));
  if(rounds===1){await mia.screenshot({path:'/tmp/jarcade-reverie-results.png'});const hand=(await room(ava)).reverie.hand;await ava.reload();await ava.waitForFunction(()=>!document.getElementById('loading'));await click(ava,180,518);await ava.waitForFunction(()=>window.__room?.reverie?.phase==='results');assert.deepEqual((await room(ava)).reverie.hand,hand);}
  if(r.reverie.phase!=='finished'){for(const p of ps)await click(p,150,800);await wait(mia,'story');}
 }
 assert.equal(errors.length,0,errors.join('\n'));
 console.log(`Coupe complete; Dicksit complete in ${rounds} rounds; separate sessions, hidden cards/votes, previews, reload/resume, idle rendering verified.`);
 await browser.close();
})().catch(async e=>{console.error(e);for(let i=0;i<debugPages.length;i++){const p=debugPages[i];try{await p.screenshot({path:`/tmp/jarcade-online-failure-${i}.png`});console.log(i,await p.locator("canvas").getAttribute("aria-label"));}catch{}}process.exit(1);});
