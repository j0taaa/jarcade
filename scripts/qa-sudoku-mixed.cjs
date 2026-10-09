const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {setup,play}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
 const errors=[];
 try {
 const cases=process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false]]:[[390,844,false],[320,568,false],[1440,900,false],[568,320,false],[390,844,true]];
 for(const[w,h,saver]of cases){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const ready=async s=>{try{await p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:90000});}catch(e){console.error(await p.locator('canvas').getAttribute('aria-label'));throw e;}};
  const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
  const game=async()=>JSON.parse(await raw()).game;
  const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(90);};
  const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(100);};
  await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');
  let s=setup(w,h);await tap(s.variant(2));await tap(s.variant(5));await ready('V & X + Diagonal');
  await p.screenshot({path:`/tmp/jarcade-sudoku-mixed-setup-${w}-${saver}.png`});
  await tap(s.marking(0));await ready('V & X (full) + Diagonal');
  await tap([w-30,30]);await ready('Full XV:');await p.screenshot({path:`/tmp/jarcade-sudoku-mixed-help-${w}-${saver}.png`});await key('Escape');
  await tap(s.difficulty(2));await tap(s.new);await ready('Hard. Digit mode');
  let g=await game();assert.deepEqual(g.puzzle.rules,{killer:false,xv:'Full',kropki:'Off',thermo:false,diagonal:true});
  assert(g.puzzle.hard_rating.stalled_cells>=45);assert(g.puzzle.hard_rating.forcing_steps>=3);
  console.log(`${w}×${h}: full XV + Diagonal, ${g.puzzle.givens.filter(Boolean).length} digits, ${g.puzzle.edges.length} symbols`);
  await p.screenshot({path:`/tmp/jarcade-sudoku-mixed-board-${w}-${saver}.png`});
  await key('n');g=await game();assert.equal(g.initial_notes_available,false);assert.equal(g.undo.length,1);
  const masks=g.puzzle.givens.map((v,i)=>v?0:Array.from({length:9},(_,d)=>d+1).filter(d=>!g.puzzle.givens.some((n,j)=>n===d&&(Math.floor(i/9)===Math.floor(j/9)||i%9===j%9||Math.floor(i/27)===Math.floor(j/27)&&Math.floor(i%9/3)===Math.floor(j%9/3)))).reduce((m,d)=>m|1<<(d-1),0));
  assert.deepEqual(g.marks.map(m=>m[1]),masks,'Wand ignores diagonals and XV');await key('Control+z');
  await key('h');await p.waitForFunction(()=>JSON.parse(localStorage.getItem('jarcade.sudoku.v1')).game.hints===1,null,{timeout:30000});
  g=await game();const cell=g.marks.findIndex(m=>m[0]);assert.equal(g.marks[cell][0],g.puzzle.solution[cell]);
  await key('z');await key('Control+Shift+A');await tap(play(w,h).cell(cell));await key(String(g.puzzle.solution[cell]%9+1));await ready('incorrect digit');await key('Control+z');
  const stored=await raw();await p.reload();s=setup(w,h,true);await ready('Hard. Digit mode');assert.equal(await raw(),stored);
  await p.waitForTimeout(350);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(350);assert.equal(await p.evaluate(()=>window.__frames),frames);
  await key('Escape');await ready('Choose a variant');
  // Classic clears every modifier. Two independent controls offer all modes.
  await tap(s.variant(0));await ready('Classic. Difficulty');await tap(s.marking(0));await tap(s.marking(1));await ready('V & X + Kropki');await tap(s.marking(1));await ready('Kropki (full)');
  await tap(s.difficulty(0));await tap(s.new);await ready('Easy. Digit mode');g=await game();assert.equal(g.puzzle.rules.xv,'Partial');assert.equal(g.puzzle.rules.kropki,'Full');assert(g.puzzle.edges.some((e,i,a)=>a.some((x,j)=>i!==j&&e.a===x.a&&e.b===x.b)));
  await p.screenshot({path:`/tmp/jarcade-sudoku-mixed-overlap-${w}-${saver}.png`});
  await key('Escape');await ready('Choose a variant');const before=await raw();await tap(s.difficulty(2));await tap(s.marking(0));await p.touchscreen.tap(...s.new);await ready('Generating');await key('Escape');await ready('Choose a variant');assert.equal(await raw(),before);
  if(w===390&&!saver){
   await tap(s.new);await p.waitForFunction(()=>{const label=document.querySelector('canvas')?.getAttribute('aria-label')||'';return label.includes('No puzzle found')||label.includes('Hard. Digit mode');},null,{timeout:90000});
   const status=await p.locator('canvas').getAttribute('aria-label');
   if(status.includes('No puzzle found')){assert.equal(await raw(),before);await p.screenshot({path:'/tmp/jarcade-sudoku-mixed-budget.png'});await tap([w/2,h/2+72]);await ready('Choose a variant');}
   else assert((await game()).puzzle.hard_rating.forcing_steps>=3);
  }
  await c.close();console.log(`Mixed controls passed: ${w}×${h}, ${saver?'black':'white'}`);
 }
 assert.deepEqual(errors,[]);console.log('Combined setup, independent partial/full modes, Hard floors, classical wand, hints/errors, persistence, overlap rendering, cancellation, responsive high-DPI layouts and idle redraws passed.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
