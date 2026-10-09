const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {setup,play}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const names=['Arrow','Renban','Whispers','Region sum','Palindrome','Between','Entropic','Sandwich','Anti-knight','Anti-king','Non-consecutive','Miracle'];
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
 const errors=[];
 try{
 const cases=process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false]]:[[390,844,false],[320,568,false],[1440,900,false],[568,320,false],[390,844,true]];
 for(const[w,h,saver]of cases){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const ready=async s=>{try{await p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:120000});}catch(e){console.error(await p.locator('canvas').getAttribute('aria-label'));throw e;}};
  const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
  const game=async()=>JSON.parse(await raw()).game;
  const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(90);};
  const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(90);};
  await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');let saved=false;
  const choose=async(n,hard=false)=>{
   const s=setup(w,h,saved);await tap(s.page(0));await tap(s.variant(0));await tap(s.page(n<6?1:2));await tap(s.variant(n%6));await ready(names[n]);await tap(s.difficulty(hard?2:0));
   await p.screenshot({path:`/tmp/jarcade-sudoku-variants-setup-${w}-${saver}-${n}.png`});await tap(s.new);await ready(`${hard?'Hard':'Easy'}. Digit mode`);saved=true;return game();
  };
  for(const n of w===390&&!saver?Array.from({length:12},(_,i)=>i):[0,7,11]){
   const g=await choose(n);assert.equal(g.puzzle.difficulty,'Easy');const field=['arrow','renban','whispers','region_sum','palindrome','between','entropic','sandwich','anti_knight','anti_king','non_consecutive'][n];
   if(n===11)assert(g.puzzle.rules.anti_knight&&g.puzzle.rules.anti_king&&g.puzzle.rules.non_consecutive);else assert(g.puzzle.rules[field]);
   if(n<7)assert(g.puzzle.lines.some(l=>l.kind===['Arrow','Renban','Whispers','RegionSum','Palindrome','Between','Entropic'][n]));
   if(n===7)assert.equal(g.puzzle.sandwiches.length,18);
   await p.screenshot({path:`/tmp/jarcade-sudoku-variants-board-${w}-${saver}-${n}.png`});
   // The board inset for Sandwich also governs touch hit testing.
   const i=g.puzzle.givens.findIndex(d=>d===0),l=play(w,h,n===7);await tap(l.cell(i));await key(String(g.puzzle.solution[i]%9+1));await ready('incorrect digit');assert.equal((await game()).marks[i][0],g.puzzle.solution[i]%9+1);await key('Control+z');
   await key('h');await p.waitForFunction(()=>JSON.parse(localStorage.getItem('jarcade.sudoku.v1')).game.hints===1,null,{timeout:30000});
   const state=await raw();await p.reload();await ready('Digit mode');assert.equal(await raw(),state);
   await p.waitForTimeout(300);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(300);assert.equal(await p.evaluate(()=>window.__frames),frames);
   await key('Escape');await ready('Choose a variant');console.log(`${w}×${h} ${names[n]}: generated, touch/errors/hints, restored, idle`);
  }
  if(w===390&&!saver){for(const n of [0,7,11]){const g=await choose(n,true);const r=g.puzzle.hard_rating;assert(r.stalled_cells>=45&&r.advanced_steps>=5&&r.forcing_steps>=3&&r.longest_chain>=30);console.log(`Hard ${names[n]}: ${g.puzzle.givens.filter(Boolean).length} digits`,r);await key('Escape');await ready('Choose a variant');}}
  // Rules persist across tabs; the plain popup lists active line constraints.
  const s=setup(w,h,true);await tap(s.page(0));await tap(s.variant(0));await tap(s.page(1));for(let i=0;i<6;i++)await tap(s.variant(i));await tap(s.page(2));await tap(s.variant(0));await tap(s.variant(1));await tap([w-30,30]);await ready('Rules.');
  await p.screenshot({path:`/tmp/jarcade-sudoku-variants-help-${w}-${saver}.png`});await key('End');await key('Escape');

  const before=await raw();await tap(s.page(0));await tap(s.variant(0));await tap(s.variant(5));await tap(s.page(2));await tap(s.variant(2));await p.touchscreen.tap(...s.new);await ready('Generating');const start=Date.now();await key('Escape');await ready('Choose a variant');assert(Date.now()-start<1500);assert.equal(await raw(),before);
  await c.close();console.log(`New variants passed ${w}×${h}, ${saver?'black':'white'}`);
 }
 assert.deepEqual(errors,[]);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
