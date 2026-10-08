const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {play,setup}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const values=g=>g.puzzle.givens.map((v,i)=>v||g.marks[i][0]);
function expectedNotes(g){
  const visible=values(g);
  return visible.map((value,i)=>value?0:Array.from({length:9},(_,n)=>n+1).filter(n=>!visible.some((v,j)=>v===n&&(Math.floor(i/9)===Math.floor(j/9)||i%9===j%9||(Math.floor(i/27)===Math.floor(j/27)&&Math.floor(i%9/3)===Math.floor(j%9/3))))).reduce((mask,n)=>mask|(1<<(n-1)),0));
}
function verify(before,after){
  const expected=expectedNotes(before),visible=values(before);
  assert.deepEqual(values(after),visible,'Mechanical notes must never enter a large digit');
  assert.equal(after.hints,before.hints,'This button is not a hint');
  for(let i=0;i<81;i++){
    assert.equal(after.marks[i][3],before.marks[i][3],'Preserve colours');
    if(visible[i])assert.deepEqual(after.marks[i],before.marks[i],'Preserve filled cells');
    else{assert.equal(after.marks[i][1],expected[i],`${before.puzzle.variant}, cell ${i}: only classic visible peers`);assert.equal(after.marks[i][2],0,'Replace stale centre notes');}
  }
  return expected;
}
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
  try{
    for(const[w,h,saver]of[[390,844,false],[390,844,true],[320,568,false],[1440,900,false],[568,320,false]]){
      const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
      await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
      const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:30000});
      const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
      const game=async()=>JSON.parse(await raw()).game;
      const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(75);};
      const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(90);};
      const l=play(w,h);let saved=false;
      await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');
      for(const variant of w===390&&!saver?[0,1,2,3,4,5]:[0]){
        const s=setup(w,h,saved);await tap(s.variant(variant));await tap(s.difficulty(1));await tap(s.new);await ready('Digit mode');saved=true;
        let g=await game();const blank=g.puzzle.givens.map((v,i)=>!v?i:-1).filter(i=>i>=0),i=blank[0],j=blank[1],wrong=g.puzzle.solution[i]%9+1;
        await tap(l.cell(i));await key(String(wrong));await ready('1 incorrect digit');
        await tap(l.cell(j));await key('x');await key('2');await key('c');await key('3');await key('v');await key('4');
        const before=await game();await tap(l.candidates);await ready('Corner notes mode');await ready('Notes filled.');let after=await game();const masks=verify(before,after);assert.equal(after.undo.length,before.undo.length+1,'All cells are one undoable action');
        if(variant===0)await p.screenshot({path:`/tmp/jarcade-sudoku-fill-notes-${w}-${saver}.png`});
        await key('Control+z');assert.deepEqual((await game()).marks,before.marks);await key('Control+y');assert.deepEqual((await game()).marks,after.marks);
        const idempotent=await raw();await key('n');assert.equal(await raw(),idempotent,'Repeat fill must not add history');
        // Generated notes participate in digit lookup without changing the save.
        await key('Control+Shift+a');const digit=Array.from({length:9},(_,n)=>n+1).find(n=>masks.some(mask=>mask&(1<<(n-1))));assert(digit);await key(String(digit));const matches=masks.map((mask,i)=>mask&(1<<(digit-1))?i:-1).filter(i=>i>=0).map(i=>`r${Math.floor(i/9)+1}c${i%9+1}`).join(', ');await ready(`Candidate digit ${digit}: ${matches}.`);assert.equal(await raw(),idempotent);
        // The smaller eraser must still clear only the selected cell's notes.
        const eraseCell=masks.findIndex(mask=>mask);await tap(l.cell(eraseCell));await tap(l.erase);const erased=await game();const expected=after.marks.map(m=>m.slice());expected[eraseCell][1]=0;assert.deepEqual(erased.marks,expected);
        await key('n');verify(erased,await game());const stored=await raw();await p.reload();await ready('Choose a variant');await tap(setup(w,h,true).resume);await ready('Digit mode');assert.equal(await raw(),stored);
        if(w===1440){await key('Tab');for(let f=0;f<16;f++)await key('Tab');await ready('Focused control: Fill notes:');const beforeFocus=await raw();await key('Enter');await ready('Corner notes mode');assert.equal(await raw(),beforeFocus);await key('Space');}
        await tap([w-30,30]);await ready('Rules.');if(variant===0)await p.screenshot({path:`/tmp/jarcade-sudoku-fill-notes-help-${w}-${saver}.png`});await key('Escape');await ready('Digit mode');
        await p.waitForTimeout(350);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>window.__frames),frames,'Fill notes must not force idle redraws');
        await key('Escape');await ready('Choose a variant');
      }
      await c.close();console.log(`Fill notes passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);console.log('Classic-only mechanical candidates in all six variants, visible wrong digits, untouched values/colours, one-step undo/redo, idempotence, lookup, erase, touch/keyboard accessibility, saved notes, responsive layouts and idle rendering passed.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
