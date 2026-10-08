const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {play,setup}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
 const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
 try{
  const cases=process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false]]:[[390,844,false],[390,844,true],[1440,900,false],[568,320,false]];
  for(const[w,h,saver]of cases){
   const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
   await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
   const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));const label=()=>p.locator('canvas').getAttribute('aria-label');
   const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:60000});
   const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));const game=async()=>JSON.parse(await raw()).game;
   const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(75);};const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(80);};
   await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');await tap(setup(w,h).new);await ready('Digit mode');
   // A pre-upgrade Hard save still loads unchanged; new rating rules apply to new puzzles.
   const old=JSON.parse(await raw());old.game.puzzle.difficulty='Hard';delete old.game.puzzle.hard_rating;const legacy=JSON.stringify(old);
   await p.evaluate(data=>localStorage.setItem('jarcade.sudoku.v1',data),legacy);await p.reload();await ready('Choose a variant');await tap(setup(w,h,true).resume);await ready('Hard. Digit mode');assert.equal(await raw(),legacy);
   await key('Escape');await ready('Choose a variant');await tap(setup(w,h,true).variant(5));await tap(setup(w,h,true).difficulty(2));
   await p.touchscreen.tap(...setup(w,h,true).new);await ready('Generating Diagonal Hard');const cancelStart=Date.now();await key('Escape');await ready('Choose a variant');assert(Date.now()-cancelStart<1500,'Proof work must let Cancel respond promptly');assert.equal(await raw(),legacy);
   const variants=w===390&&!saver&&process.env.JARCADE_QA_FOCUS!=='smoke'?[0,1,2,3,4,5]:[w===568?4:0];
   for(const variant of variants){
    const s=setup(w,h,true);await tap(s.variant(variant));await tap(s.difficulty(2));const start=Date.now();await tap(s.new);await ready('Hard. Digit mode');const g=await game(),rating=g.puzzle.hard_rating;
    assert(rating,'Every newly generated Hard puzzle is rated');assert(rating.stalled_cells>=45);assert(rating.advanced_steps>=5);assert(rating.forcing_steps>=3);assert(rating.longest_chain>=30);assert.equal(g.puzzle.difficulty,'Hard');assert.equal(g.puzzle.variant,['Classic','Killer','Xv','Kropki','Thermo','Diagonal'][variant]);
    assert(g.puzzle.givens.every((v,i)=>!v||v===g.puzzle.solution[i]));assert.equal(g.hints,0);assert.equal(g.initial_notes_available,true);
    console.log(`${g.puzzle.variant} Hard: ${JSON.stringify(rating)}, generation ${Date.now()-start}ms`);
    await p.screenshot({path:`/tmp/jarcade-sudoku-hard-${w}-${saver}-${variant}.png`});
    // Expert hints, grouped undo and saved progress still work with sparse variants.
    await key('h');await p.waitForFunction(()=>JSON.parse(localStorage.getItem('jarcade.sudoku.v1')).game.hints===1,null,{timeout:30000});
    let hinted=await game();const cell=hinted.marks.findIndex(m=>m[0]);assert(cell>=0);assert.equal(hinted.marks[cell][0],g.puzzle.solution[cell]);assert(!((await label()).includes('incorrect digit')));
    await key('Control+z');assert.deepEqual((await game()).marks,g.marks);await key('Control+y');assert.deepEqual((await game()).marks,hinted.marks);
    const stored=await raw();await p.reload();await ready('Choose a variant');await tap(setup(w,h,true).resume);await ready('Hard. Digit mode');assert.equal(await raw(),stored);
    const l=play(w,h);await tap(l.cell(cell));await key(String(g.puzzle.solution[cell]%9+1));await ready('1 incorrect digit');await key('Control+z');assert.deepEqual((await game()).marks,hinted.marks);
    await p.waitForTimeout(400);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>window.__frames),frames,'Hard mode must remain idle with the FPS counter');
    const dpi=await p.evaluate(()=>({w:document.querySelector('canvas').width,h:document.querySelector('canvas').height}));assert.equal(dpi.w,w*3);assert.equal(dpi.h,h*3);
    await key('Escape');await ready('Choose a variant');
   }
   await c.close();console.log(`Hard generation passed: ${w}×${h}, ${saver?'black':'white'}`);
  }
  assert.deepEqual(errors,[]);console.log('All expert difficulty floors, all variants, cancel responsiveness, old-save compatibility, expert hints, errors, undo/redo, persistence, high DPI and idle rendering passed.');
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
