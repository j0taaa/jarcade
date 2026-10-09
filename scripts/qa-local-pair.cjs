// Four real local two-player games: rules, private screens, persistence and touch.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {mkdirSync}=require('node:fs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const artifacts='/tmp/jarcade-local-pair-qa';mkdirSync(artifacts,{recursive:true});
const games=['tic-tac-toe','connect-four','mastermind','guess-who'];
const names=['Jogo da Velha','Ligue 4','Mastermind','Cara a Cara'];
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
 const label=p=>p.locator('canvas').getAttribute('aria-label');
 const wait=(p,text)=>p.waitForFunction(text=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(text),text);
 const save=p=>p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.local-pair.v1')));
 const click=async(p,x,y)=>{await p.mouse.click(x,y);await p.waitForTimeout(110)};
 const keys=async(p,seq)=>{for(const k of seq){await p.keyboard.press(k);await p.waitForTimeout(65)}};
 const idle=async p=>{await p.waitForTimeout(600);const n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(250);assert.equal(await p.evaluate(()=>window.__frames),n,'Idle games must sleep with FPS enabled')};
 async function setup(w=390,h=844,saver=false,state=null,blocked=false){
  const context=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await context.addInitScript(({saver,state,blocked})=>{
   if(!localStorage.getItem('jarcade.settings.v1'))localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
   if(state&&!localStorage.getItem('jarcade.local-pair.v1'))localStorage.setItem('jarcade.local-pair.v1',JSON.stringify(state));
   window.__frames=0;window.__sockets=0;const raf=requestAnimationFrame;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t)});
   const Socket=WebSocket;window.WebSocket=class extends Socket{constructor(...args){super(...args);window.__sockets++}};
   if(blocked){Storage.prototype.getItem=()=>{throw Error('blocked')};Storage.prototype.setItem=()=>{throw Error('blocked')}}
  },{saver,state,blocked});
  const p=await context.newPage();p.on('pageerror',e=>errors.push(e.message));return{context,p};
 }
 const load=async(p,game)=>{await p.goto(`${base}/games/${game}`);await wait(p,`Jarcade. ${names[games.indexOf(game)]}.`);await p.waitForTimeout(120)};
 const reset=async p=>{await click(p,302,30);await wait(p,'Confirmar reinício');await click(p,285,501);await p.waitForTimeout(100)};
 try{
  const {context,p}=await setup();await load(p,'tic-tac-toe');await idle(p);
  const frames=await p.evaluate(()=>window.__frames);await keys(p,['1']);await p.waitForTimeout(150);assert((await p.evaluate(()=>window.__frames))>=frames+2,'Keyboard moves must present the changed board before idling');await idle(p);
  await keys(p,['4','2','5','3']);await wait(p,'Won(0)');assert.deepEqual((await save(p)).tic.scores,[1,0]);
  await click(p,314,560);assert.equal((await save(p)).tic.moves.length,5);await p.reload();await wait(p,'Won(0)');await click(p,195,805);await wait(p,'Rodada 2.');await keys(p,['1']);assert((await label(p)).includes('Tabuleiro: [2,'));
  await reset(p);await keys(p,['1','2','3','5','4','6','8','7','9']);await wait(p,'Draw.');assert.deepEqual((await save(p)).tic.scores,[0,0]);
  await p.screenshot({path:`${artifacts}/tic-draw.png`,scale:'css'});
  await load(p,'connect-four');await keys(p,['1','2','1','2','1','2','1']);await wait(p,'Won(0)');assert.deepEqual((await save(p)).connect.scores,[1,0]);await click(p,195,805);await keys(p,['1','1','1','1','1','1']);assert.equal((await save(p)).connect.moves.length,6);await keys(p,['1']);assert.equal((await save(p)).connect.moves.length,6);
  await reset(p);const cdp=await context.newCDPSession(p);const touch=(type,pts)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:pts.map(([id,x,y])=>({id,x,y}))});
  await touch('touchStart',[[1,42,440]]);await p.waitForTimeout(50);await touch('touchMove',[[1,100,460]]);await p.waitForTimeout(50);await touch('touchEnd',[]);await p.waitForTimeout(120);assert.equal((await save(p)).connect.moves.length,0,'Board swipe cannot drop a disc');
  await touch('touchStart',[[1,42,440]]);await p.waitForTimeout(50);await touch('touchMove',[[1,42,440],[2,100,460]]);await p.waitForTimeout(50);await touch('touchEnd',[]);await p.waitForTimeout(120);assert.equal((await save(p)).connect.moves.length,0,'Pinch cannot drop a disc');
  await p.touchscreen.tap(16+358/7,440);await p.waitForTimeout(160);assert.equal((await save(p)).connect.moves.length,1,'A shared column edge commits one disc only');
  await reset(p);const draw=[5,3,6,5,5,0,4,6,1,0,4,3,3,2,4,6,3,1,5,0,5,4,1,6,2,5,4,3,4,2,6,2,0,1,2,6,2,3,0,1,0,1];await keys(p,draw.map(i=>String(i+1)));await wait(p,'Draw.');assert.deepEqual((await save(p)).connect.scores,[0,0]);
  await load(p,'mastermind');await wait(p,'CodeCover.');assert(!(await label(p)).includes('Cores selecionadas'));await click(p,195,805);await wait(p,'Code.');await keys(p,['1','1','2','3']);assert.deepEqual((await save(p)).mastermind.code,[0,0,1,2]);assert.equal((await save(p)).mastermind.phase,'CodeCover');
  await p.reload();await wait(p,'CodeCover.');await click(p,195,805);await wait(p,'Code.');assert((await label(p)).includes('[Some(0), Some(0), Some(1), Some(2)]'));
  await p.evaluate(()=>history.back());await wait(p,'Jarcade. Navigation. Mastermind.');assert(!(await label(p)).includes('Some('));await p.evaluate(()=>history.forward());await wait(p,'CodeCover.');await click(p,195,805);await click(p,195,805);await wait(p,'GuessCover.');assert(!(await label(p)).includes('Cores selecionadas'));
  await click(p,195,805);await wait(p,'Guess.');assert((await label(p)).includes('[None, None, None, None]'));await keys(p,['1','2','1','1','Enter']);await wait(p,'1 exatas e 2 em outra posição');assert.equal((await save(p)).mastermind.guesses.length,1);
  await p.screenshot({path:`${artifacts}/mastermind-feedback.png`,scale:'css'});await keys(p,['1']);await p.evaluate(()=>window.dispatchEvent(new Event('blur')));await wait(p,'GuessCover.');await click(p,195,805);await wait(p,'Guess.');assert.equal((await save(p)).mastermind.draft[0],0);await keys(p,['1','1','2','3','Enter']);await wait(p,'Código descoberto.');assert.deepEqual((await save(p)).mastermind.scores,[2,0]);const mmWin=await save(p);await click(p,195,805);await wait(p,'CodeCover.');assert.equal((await save(p)).mastermind.maker,1);
  await load(p,'guess-who');await wait(p,'Cover { player: 0');assert(!(await label(p)).includes('Meu personagem:'));await click(p,195,805);await wait(p,'Choose { player: 0');await click(p,70,215);await wait(p,'Meu personagem: Alex.');await click(p,195,805);await wait(p,'Cover { player: 1');assert(!(await label(p)).includes('Meu personagem: Alex'));
  await click(p,195,805);await wait(p,'Choose { player: 1');await click(p,195,215);await wait(p,'Meu personagem: Bia.');await click(p,195,805);await wait(p,'Cover { player: 0');await click(p,195,805);await wait(p,'Turn { player: 0');assert((await label(p)).includes('Meu personagem: Alex.'));assert(!(await label(p)).includes('Meu personagem: Bia.'));
  await click(p,195,215);assert.equal((await save(p)).faces.eliminated[0][1],true);await click(p,195,215);assert.equal((await save(p)).faces.eliminated[0][1],false);
  const before=(await save(p)).faces.eliminated;await touch('touchStart',[[1,70,220]]);await p.waitForTimeout(60);await touch('touchMove',[[1,70,350]]);await p.waitForTimeout(60);await touch('touchEnd',[]);await p.waitForTimeout(100);assert.deepEqual((await save(p)).faces.eliminated,before,'Portrait swipes cannot eliminate');
  await click(p,285,805);await wait(p,'Cover { player: 1');await click(p,195,805);await wait(p,'Turn { player: 1');assert.equal((await save(p)).faces.eliminated[1][1],false);await p.reload();await wait(p,'Cover { player: 1');assert(!(await label(p)).includes('Meu personagem:'));await click(p,195,805);await wait(p,'Turn { player: 1');await click(p,100,805);await click(p,70,215);await wait(p,'Confirmar palpite: Alex?');await click(p,100,501);await wait(p,'Turn { player: 1');assert.equal((await save(p)).faces.scores[1],0);await click(p,70,215);await click(p,285,501);await wait(p,'End { winner: 1 }');assert.deepEqual((await save(p)).faces.scores,[0,1]);
  const seed=await save(p);seed.mastermind=mmWin.mastermind;seed.mastermind.phase='GuessCover';seed.mastermind.guesses=[[0,1,0,0]];seed.mastermind.draft=[null,null,null,null];seed.faces.phase={Cover:{player:0,choosing:false}};seed.faces.eliminated=[Array(24).fill(false),Array(24).fill(false)];
  assert.equal(await p.evaluate(()=>window.__sockets),0);await context.close();
  for(const [w,h,saver] of [[280,360,false],[320,568,false],[390,844,false],[568,320,false],[820,1180,false],[1440,900,false],[390,844,true]]){
   const q=await setup(w,h,saver,seed);for(const game of games){await load(q.p,game);if(game==='mastermind'||game==='guess-who'){// Covers use the right half in landscape.
     await click(q.p,w>=512&&h<532?w-16-(w-32)*0.23:w/2,h-40);await wait(q.p,game==='mastermind'?'Guess.':'Turn { player: 0');}
    await idle(q.p);assert.deepEqual(await q.p.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);assert.equal(await q.p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');await q.p.screenshot({path:`${artifacts}/${game}-${w}x${h}-${saver}.png`,scale:'css'});assert.equal(await q.p.evaluate(()=>window.__sockets),0);
    if(game==='guess-who'){for(let i=0;i<15;i++){await q.p.keyboard.press('PageDown');await q.p.waitForTimeout(30)}await q.p.screenshot({path:`${artifacts}/faces-bottom-${w}x${h}-${saver}.png`,scale:'css'});}
   }await q.context.close();
  }
  const blocked=await setup(390,844,false,null,true);await load(blocked.p,'tic-tac-toe');await keys(blocked.p,['1','4','2','5','3']);await wait(blocked.p,'Won(0)');await blocked.context.close();
  assert.deepEqual(errors,[]);console.log('Four local games: real wins/draws and score/rematch, duplicate-colour feedback, private reload/Back/blur and separate boards, touch drag/pinch/edge safety, storage failures, zero sockets, DPR3, idle FPS and 28 game/layout combinations passed.');
 }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
