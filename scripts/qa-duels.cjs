const {chromium}=require('playwright');const assert=require('node:assert/strict');const fs=require('node:fs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';const dir='/tmp/jarcade-duel-qa';fs.mkdirSync(dir,{recursive:true});
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
 const wait=(p,text)=>p.waitForFunction(text=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(text),text);
 const label=p=>p.locator('canvas').getAttribute('aria-label');
 const save=p=>p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.local-duel.v1')));
 const click=async(p,x,y)=>{await p.mouse.click(x,y);await p.waitForTimeout(100)};
 const keys=async(p,seq)=>{for(const key of seq){await p.keyboard.press(key);await p.waitForTimeout(90)}};
 async function setup(w=390,h=844,saver=false,seed=null){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await c.addInitScript(({saver,seed})=>{if(!localStorage.getItem('jarcade.settings.v1'))localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);if(seed&&!localStorage.getItem('jarcade.local-duel.v1'))localStorage.setItem('jarcade.local-duel.v1',JSON.stringify(seed));
   window.__frames=0;const raf=requestAnimationFrame;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t)});window.__sockets=0;const S=WebSocket;window.WebSocket=class extends S{constructor(...args){super(...args);window.__sockets++}};
  },{saver,seed});const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));return{c,p};
 }
 async function idle(p){await p.waitForTimeout(500);const f=await p.evaluate(()=>window.__frames);await p.waitForTimeout(250);assert.equal(await p.evaluate(()=>window.__frames),f)}
 async function open(p,game){await p.goto(`${base}/games/${game}`);await wait(p,game==='air-hockey'?'Jarcade. Air Hockey.':'Jarcade. Batalha Naval.');await p.waitForTimeout(100)}
 try{
  const {c,p}=await setup();await open(p,'air-hockey');await idle(p);await keys(p,['Space']);await wait(p,'Playing.');
  const cdp=await c.newCDPSession(p);const touch=(type,points)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:points.map(([id,x,y])=>({id,x,y}))});
  await touch('touchStart',[[1,195,628],[2,195,212]]);await p.waitForTimeout(50);await touch('touchMove',[[1,70,560],[2,320,280]]);await p.waitForTimeout(180);
  await p.screenshot({path:`${dir}/two-fingers.png`,scale:'css'});await touch('touchEnd',[]);await keys(p,['Space']);await wait(p,'Paused.');let state=await save(p);
  assert(state.hockey.paddles[0][0]<0.3&&state.hockey.paddles[1][0]>0.7,'Both independent fingers must move their own paddles');
  await idle(p);await p.reload();await wait(p,'Paused.');assert.deepEqual((await save(p)).hockey.scores,state.hockey.scores);
  await keys(p,['Space']);await p.keyboard.down('d');await p.keyboard.down('ArrowLeft');await p.waitForTimeout(150);await p.keyboard.up('d');await p.keyboard.up('ArrowLeft');await keys(p,['Space']);await wait(p,'Paused.');const keyboard=await save(p);assert(keyboard.hockey.paddles[0][0]>state.hockey.paddles[0][0]);assert(keyboard.hockey.paddles[1][0]<state.hockey.paddles[1][0]);
  const beforeTap=await save(p);await keys(p,['Space','a','ArrowRight']);await p.waitForTimeout(120);await keys(p,['Space']);await wait(p,'Paused.');const afterTap=await save(p);assert(afterTap.hockey.paddles[0][0]<beforeTap.hockey.paddles[0][0]);assert(afterTap.hockey.paddles[1][0]>beforeTap.hockey.paddles[1][0]);
  await keys(p,['Space']);await p.evaluate(()=>window.dispatchEvent(new Event('blur')));await wait(p,'Paused.');await idle(p);
  await open(p,'battleship');await wait(p,'Cover { player: 0');assert(!(await label(p)).includes('Frota:'));await click(p,195,804);await wait(p,'Setup(0).');
  await click(p,345.3,451.3);await keys(p,['Enter']);assert((await save(p)).naval.fleets[0].every(v=>v===null),'Out-of-bounds placement cannot be confirmed');await keys(p,['R','R']);
  // Five manually confirmed placements; player 2 uses random setup.
  for(let i=0;i<5;i++){await click(p,44.7,150.7+i*33.4);await keys(p,['Enter']);}
  assert((await save(p)).naval.fleets[0].every(Boolean));assert.equal((await save(p)).naval.phase.Cover.resume,'Setup');
  await p.reload();await wait(p,'Cover { player: 0');assert(!(await label(p)).includes('Frota:'));await click(p,195,804);await wait(p,'Setup(0).');
  await click(p,195,804);await wait(p,'Cover { player: 1');await click(p,195,804);await wait(p,'Setup(1).');await keys(p,['A']);assert((await save(p)).naval.fleets[1].every(Boolean));
  const prepared=await save(p);await click(p,195,804);await wait(p,'Cover { player: 0');await click(p,195,804);await wait(p,'Turn(0).');assert(!(await label(p)).includes('Frota:'));assert(!(await label(p)).includes('Ship'));
  await p.evaluate(()=>history.back());await wait(p,'Jarcade. Navigation. Batalha Naval.');assert(!(await label(p)).includes('Alvos:'));await p.evaluate(()=>history.forward());await wait(p,'Cover { player: 0');await click(p,195,804);await wait(p,'Turn(0).');
  // A swipe or pinch that begins on a cell never chooses or fires at it.
  await touch('touchStart',[[1,44,202]]);await p.waitForTimeout(40);await touch('touchMove',[[1,145,255]]);await p.waitForTimeout(40);await touch('touchEnd',[]);await p.waitForTimeout(100);assert(!(await label(p)).includes('Selecionado'));
  await touch('touchStart',[[1,44,202],[2,90,202]]);await p.waitForTimeout(50);await touch('touchEnd',[]);await p.waitForTimeout(100);assert.equal((await save(p)).naval.shots.length,0);
  const cells=fleet=>fleet.flatMap((s,i)=>Array.from({length:[5,4,3,3,2][i]},(_,n)=>(s.y+(s.vertical?n:0))*10+s.x+(s.vertical?0:n)));
  const targets=cells(prepared.naval.fleets[1]);const own=new Set(cells(prepared.naval.fleets[0]));const misses=Array.from({length:100},(_,i)=>i).filter(i=>!own.has(i));
  const choose=async cell=>{await click(p,28+(cell%10+0.5)*33.4,186+(Math.floor(cell/10)+0.5)*33.4);await keys(p,['Enter']);};
  for(let i=0;i<targets.length;i++){
   if(i===1){const count=(await save(p)).naval.shots.length;await choose(targets[0]);await wait(p,'Turn(0).');assert.equal((await save(p)).naval.shots.length,count,'Repeated shots cannot be confirmed');}
   await choose(targets[i]);if(i===targets.length-1)break;await wait(p,'Report(0).');
   if(i===0){await p.reload();await wait(p,'resume: Report');assert(!(await label(p)).includes('Alvos:'));await click(p,195,804);await wait(p,'Report(0).');}
   await click(p,195,804);await wait(p,'Cover { player: 1');await click(p,195,804);await wait(p,'Turn(1).');await choose(misses[i]);await wait(p,'Report(1).');await click(p,195,804);await click(p,195,804);await wait(p,'Turn(0).');
  }
  await wait(p,'End(0).');assert.deepEqual((await save(p)).naval.scores,[1,0]);const win=await save(p);await idle(p);await p.screenshot({path:`${dir}/naval-win.png`,scale:'css'});await click(p,195,804);await wait(p,'Cover { player: 0');assert.equal((await save(p)).naval.round,2);
  assert.equal(await p.evaluate(()=>window.__sockets),0);await c.close();
  for(const player of [0,1]){const fixture=structuredClone(win);fixture.hockey={...fixture.hockey,phase:'Paused',scores:player===0?[6,0]:[0,6],puck:[0.5,player===0?0.04:1.56],velocity:[0,player===0?-3:3],paddles:[[0.12,1.4],[0.12,0.2]],targets:[[0.12,1.4],[0.12,0.2]]};
   const q=await setup(390,844,false,fixture);await open(q.p,'air-hockey');await keys(q.p,['Space']);await wait(q.p,`Finished(${player}).`);assert.equal((await save(q.p)).hockey.matches[player],1);await q.c.close();}
  const seed=structuredClone(prepared);seed.hockey.phase='Ready';seed.hockey.scores=[0,0];seed.naval.phase={Cover:{player:0,resume:'Turn'}};
  for(const [w,h,saver]of [[280,360,false],[320,480,false],[390,844,false],[568,320,false],[820,1180,false],[1440,900,false],[390,844,true]]){
   const q=await setup(w,h,saver,seed);for(const game of ['air-hockey','battleship']){await open(q.p,game);if(game==='battleship'){await click(q.p,w>=512&&h<532?w*.75:w/2,h-40);await wait(q.p,'Turn(0).');}
    await idle(q.p);assert.deepEqual(await q.p.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);assert.equal(await q.p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');await q.p.screenshot({path:`${dir}/${game}-${w}x${h}-${saver}.png`,scale:'css'});assert.equal(await q.p.evaluate(()=>window.__sockets),0);
    if(game==='battleship'){await q.p.keyboard.press('PageDown');await q.p.waitForTimeout(100);await q.p.screenshot({path:`${dir}/naval-bottom-${w}x${h}.png`,scale:'css'});}
   }await q.c.close();
  }
  assert.deepEqual(errors,[]);console.log('Simultaneous fingers, independent keyboard players, pause/blur/reload, both goals and first-to-seven, private manual/random fleets, real naval win and rematch, hidden report reload, drag/pinch safety, zero sockets, DPR3 and fourteen layouts passed.');
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
