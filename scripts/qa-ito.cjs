// Real shared-device play, privacy, touch cancellation and responsive/idle checks.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {mkdirSync}=require('node:fs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const artifacts='/tmp/jarcade-ito-qa';mkdirSync(artifacts,{recursive:true});
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
 const label=p=>p.locator('canvas').getAttribute('aria-label');
 const phase=(p,value)=>p.waitForFunction(value=>document.querySelector('canvas').getAttribute('aria-label')?.includes(value),value);
 const save=p=>p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.ito.v1')));
 const click=async(p,x,y)=>{await p.mouse.click(x,y);await p.waitForTimeout(120)};
 const idle=async p=>{await p.waitForTimeout(700);const n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(250);assert.equal(await p.evaluate(()=>window.__frames),n,'Static Ito screen must sleep with FPS enabled')};
 async function open(w=390,h=844,saver=false,state=null,blocked=false){
  const context=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await context.addInitScript(({saver,state,blocked})=>{
   if(!localStorage.getItem('jarcade.settings.v1'))localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
   if(state&&!localStorage.getItem('jarcade.ito.v1'))localStorage.setItem('jarcade.ito.v1',JSON.stringify(state));
   window.__frames=0;window.__sockets=0;const raf=requestAnimationFrame;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t)});
   const Socket=WebSocket;window.WebSocket=class extends Socket{constructor(...args){super(...args);window.__sockets++}};
   if(blocked){Storage.prototype.getItem=()=>{throw Error('blocked')};Storage.prototype.setItem=()=>{throw Error('blocked')}}
  },{saver,state,blocked});
  const p=await context.newPage();p.on('pageerror',e=>errors.push(e.message));await p.goto(`${base}/games/ito`);await p.waitForFunction(()=>!document.querySelector('#loading'));return{context,p};
 }
 async function clue(p,index,text,w=390,h=844){const cw=(w-32-10)/2;await click(p,16+index*(cw+10)+cw/2,240);await click(p,w/2,h-120);const input=p.getByRole('textbox',{name:'Ito clue'});await input.fill(text);await input.press('Enter');await p.waitForFunction(text=>JSON.parse(localStorage.getItem('jarcade.ito.v1')).cards.some(c=>c.clue===text),text)}
 try{
  const {context,p}=await open();await phase(p,'. Setup.');await idle(p);
  await click(p,352,227);await click(p,280,482);assert.equal((await save(p)).config.language,'Portuguese');
  await click(p,100,482);await click(p,195,805);await phase(p,'Handoff { player: 0');await idle(p);assert(!/Card [ABC]: \d/.test(await label(p)));
  await click(p,195,805);await phase(p,'Hand { player: 0');assert.equal((await save(p)).phase.kind,'handoff');
  assert.equal((await label(p)).match(/Card [ABC]: \d+/g).length,2);
  await p.reload();await phase(p,'Handoff { player: 0');await click(p,195,805);await phase(p,'Hand { player: 0');
  await p.evaluate(()=>window.dispatchEvent(new Event('blur')));await phase(p,'Handoff { player: 0');await click(p,195,805);await phase(p,'Hand { player: 0');
  await p.evaluate(()=>history.back());await phase(p,'Jarcade. Navigation. Ito.');assert(!/Card [ABC]: \d/.test(await label(p)));
  await p.evaluate(()=>history.forward());await phase(p,'Handoff { player: 0');await click(p,195,805);await phase(p,'Hand { player: 0');
  await clue(p,0,'Feather');await clue(p,1,'Cat');await click(p,195,805);await phase(p,'Handoff { player: 1');assert(!/Card [ABC]: \d/.test(await label(p)));
  await click(p,195,805);await phase(p,'Hand { player: 1');await clue(p,0,'Dog');await clue(p,1,'Elephant');await click(p,195,805);await phase(p,'. Arrange.');await idle(p);assert(!/Number \d/.test(await label(p)));
  let state=await save(p);const start=[...state.order];
  const cdp=await context.newCDPSession(p);const touch=(type,pts)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:pts.map(([id,x,y])=>({id,x,y}))});
  // Scroll rows start immediately below the category, at y=224.
  await touch('touchStart',[[1,352,261]]);await p.waitForTimeout(60);await touch('touchMove',[[1,352,495]]);await p.waitForTimeout(60);await touch('touchEnd',[]);await p.waitForTimeout(160);
  state=await save(p);assert.notDeepEqual(state.order,start,'Actual high-DPI grip drag must reorder');
  let before=[...state.order];await touch('touchStart',[[1,352,261]]);await p.waitForTimeout(60);await touch('touchMove',[[1,352,339]]);await p.waitForTimeout(60);await touch('touchCancel',[]);await p.waitForTimeout(120);assert.deepEqual((await save(p)).order,before);
  await touch('touchStart',[[1,352,261]]);await p.waitForTimeout(60);await touch('touchMove',[[1,352,339],[2,100,400]]);await p.waitForTimeout(60);await touch('touchEnd',[]);await p.waitForTimeout(120);assert.deepEqual((await save(p)).order,before);
  await click(p,142,805);await click(p,95,461);await phase(p,'Handoff { player: 0, review: true');await click(p,195,805);await clue(p,0,'Tiny feather');await click(p,195,805);await phase(p,'. Arrange.');assert((await label(p)).includes('Tiny feather'));
  // Sort through actual card selection + arrow controls; the saved numbers are a test oracle only.
  state=await save(p);const sorted=[...state.order].sort((a,b)=>state.cards[a].number-state.cards[b].number);
  for(let target=0;target<sorted.length;target++){
   let current=(await save(p)).order.indexOf(sorted[target]);await click(p,150,224+current*78+37);
   while(current-->target)await click(p,40,805);
  }
  assert.deepEqual((await save(p)).order,sorted);
  await click(p,150,261);await p.keyboard.press('Control+ArrowDown');await p.waitForTimeout(120);
  assert.equal((await save(p)).order[1],sorted[0]);
  await p.keyboard.press('Control+ArrowUp');await p.waitForTimeout(120);assert.deepEqual((await save(p)).order,sorted);
  const arranged=await save(p);
  await p.screenshot({path:`${artifacts}/arrange.png`,scale:'css'});
  await click(p,270,805);await click(p,285,509);await phase(p,'. Reveal.');assert(!/Number \d/.test(await label(p)));
  for(let i=0;i<4;i++){await click(p,195,805);assert.equal((await save(p)).revealed,i+1);if(i<3)assert.equal((await label(p)).match(/Number \d+/g).length,i+1)}
  await phase(p,'Result { won: true }');await p.screenshot({path:`${artifacts}/win.png`,scale:'css'});await idle(p);
  await click(p,195,805);await phase(p,'. Setup.');assert.equal((await save(p)).cards.length,0);await click(p,195,805);assert.equal((await save(p)).round,2);assert.equal(await p.evaluate(()=>window.__sockets),0);await context.close();
  const loss=structuredClone(arranged);loss.order.reverse();const losing=await open(390,844,false,loss);await phase(losing.p,'. Arrange.');await click(losing.p,270,805);await click(losing.p,285,509);await click(losing.p,195,805);await click(losing.p,195,805);await phase(losing.p,'Result { won: false }');await losing.context.close();
  for(const [w,h,saver] of [[280,360,false],[320,568,false],[390,844,false],[568,320,false],[820,1180,false],[1440,900,false],[390,844,true]]){
   const q=await open(w,h,saver,arranged);await phase(q.p,'. Arrange.');await idle(q.p);assert.equal(await q.p.evaluate(()=>window.__sockets),0);
   assert.deepEqual(await q.p.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);
   assert.equal(await q.p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');
   await q.p.screenshot({path:`${artifacts}/board-${w}x${h}-${saver}.png`,scale:'css'});
   await click(q.p,(w-Math.min(w-32,900))/2+Math.min(w-32,900)-72,30);await q.p.screenshot({path:`${artifacts}/reset-${w}x${h}.png`,scale:'css'});await q.context.close();
  }
  const large=structuredClone(arranged);large.config={players:10,cards_each:3,language:'English'};large.cards=Array.from({length:30},(_,i)=>({number:i+1,clue:`Example ${String.fromCharCode(65+i)}`}));large.order=Array.from({length:30},(_,i)=>i);large.phase={kind:'arrange'};large.revealed=0;
  const q=await open(320,568,false,large);await phase(q.p,'Player 10 card C');const original=[...(await save(q.p)).order];for(let i=0;i<15;i++){await q.p.keyboard.press('PageDown');await q.p.waitForTimeout(30)}await q.p.screenshot({path:`${artifacts}/thirty-cards.png`,scale:'css'});assert.deepEqual((await save(q.p)).order,original);await idle(q.p);await q.context.close();
  const tinyHand=structuredClone(arranged);tinyHand.phase={kind:'handoff',player:0,review:true};
  const tiny=await open(280,360,false,tinyHand);await phase(tiny.p,'Handoff');await click(tiny.p,140,321);await phase(tiny.p,'Hand {');
  for(let i=0;i<3;i++){await tiny.p.keyboard.press('PageDown');await tiny.p.waitForTimeout(40)}
  await tiny.p.screenshot({path:`${artifacts}/tiny-hand.png`,scale:'css'});
  await click(tiny.p,140,200);const tinyInput=tiny.p.getByRole('textbox',{name:'Ito clue'});await tinyInput.fill('Small screen');await tinyInput.press('Enter');
  await tiny.p.waitForFunction(()=>JSON.parse(localStorage.getItem('jarcade.ito.v1')).cards[0].clue==='Small screen');
  await tiny.context.close();
  const blocked=await open(390,844,false,null,true);await phase(blocked.p,'. Setup.');await click(blocked.p,195,805);await phase(blocked.p,'Handoff { player: 0');await click(blocked.p,195,805);await phase(blocked.p,'Hand { player: 0');await blocked.context.close();
  assert.deepEqual(errors,[]);console.log('Ito: shared-device round, hidden handoff/reload/blur, private review, real grip drag and cancelled/multitouch rollback, ordered win/loss, offline storage failures, 30-card scroll, seven DPR3 layouts and idle FPS passed.');
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
