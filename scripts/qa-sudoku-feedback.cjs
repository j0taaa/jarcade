// Automatic mistake feedback and digit lookup must never expose hidden answers
// or turn a visual search into an entry/undo action.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const {play,setup}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const coordinates=cells=>cells.map(i=>`r${Math.floor(i/9)+1}c${i%9+1}`).join(', ');
function pixels(path,points){
  const result=spawnSync('python3',['-c','import sys,json; from PIL import Image; im=Image.open(sys.argv[1]).convert("RGB"); print(json.dumps([im.getpixel((round(x*3),round(y*3))) for x,y in json.loads(sys.argv[2])]))',path,JSON.stringify(points)],{encoding:'utf8'});
  assert.equal(result.status,0,result.stderr);return JSON.parse(result.stdout);
}
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
  try{
    for(const[w,h,saver]of[[390,844,false],[390,844,true],[1440,900,false],[1440,900,true],[568,320,false]]){
      const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
      await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
      const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:30000});
      const label=()=>p.locator('canvas').getAttribute('aria-label');
      const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
      const game=async()=>JSON.parse(await raw()).game;
      const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(75);};
      const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(90);};
      const l=play(w,h),outside=[w/2,60];
      const sample=i=>{const a=l.cell(i),u=l.cell(1)[0]-l.cell(0)[0];return[a[0]+u*.3,a[1]+u*.3];};
      const candidates=(g,n)=>g.marks.map((m,i)=>!g.puzzle.givens[i]&&!m[0]&&((m[1]|m[2])&(1<<(n-1)))?i:-1).filter(i=>i>=0);
      const highlighted=async n=>{const g=await game(),matches=g.puzzle.givens.map((v,i)=>[v||g.marks[i][0],i]).filter(([v])=>v===n).map(([,i])=>i);
        await ready(`Highlighted digit ${n}: ${coordinates(matches)||'none'}.`);await ready(`Candidate digit ${n}: ${coordinates(candidates(g,n))||'none'}.`);await ready('Selected cells: none.');return matches;};
      await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');await tap(setup(w,h).new);await ready('Digit mode');await ready('Selected cells: none.');
      const g=await game();const initial=await raw();
      const hidden=g.puzzle.givens.findIndex((v,i)=>!v&&g.puzzle.givens.includes(g.puzzle.solution[i]));assert(hidden>=0);const query=g.puzzle.solution[hidden];
      await key(String(query));const matches=await highlighted(query);assert.equal(await raw(),initial,'Search must not change progress or undo history');assert(!matches.includes(hidden));
      const shot=`/tmp/jarcade-sudoku-feedback-${w}-${saver}.png`;await p.screenshot({path:shot});
      const sampled=pixels(shot,[...matches.map(sample),sample(hidden)]),expected=saver?[30,22,56]:[234,226,254];
      for(const colour of sampled.slice(0,-1))assert(colour.every((v,i)=>Math.abs(v-expected[i])<=3),`Every matching revealed digit must visibly highlight: ${colour}`);
      assert.deepEqual(sampled.at(-1),saver?[0,0,0]:[255,255,255],'Hidden answer cells must stay unhighlighted');
      await key(String(query));await tap(l.number(2));await highlighted(2);assert.equal(await raw(),initial);await tap(l.number(2));assert(!(await label()).includes('Highlighted digit'));
      // Every tool uses number lookup while selection is empty.
      for(const[shortcut,mode]of[['x','Corner notes'],['c','Centre notes'],['v','Colour'],['z','Digit']]){await key(shortcut);await ready(`${mode} mode`);await key('3');await highlighted(3);assert.equal(await raw(),initial);await key('3');}
      const i=g.puzzle.givens.findIndex(v=>v===0),correct=g.puzzle.solution[i],wrong=correct%9+1;
      await tap(l.cell(i));await ready(`Selected cells: ${coordinates([i])}.`);await key(String(wrong));
      await ready('1 incorrect digit');assert.equal((await game()).marks[i][0],wrong);
      const errorShot=`/tmp/jarcade-sudoku-error-${w}-${saver}.png`;await p.screenshot({path:errorShot});
      const red=pixels(errorShot,[sample(i)])[0];assert(red[0]>red[2],'Wrong digit must be red immediately without Check');
      const wrongSave=await raw();await tap(outside);await ready('Selected cells: none.');assert.equal(await raw(),wrongSave);
      await key(String(wrong));assert((await highlighted(wrong)).includes(i));assert.equal(await raw(),wrongSave);
      // Selecting a cell clears lookup; a correction clears the warning.
      await tap(l.cell(i));assert(!(await label()).includes('Highlighted digit'));await key(String(correct));assert(!(await label()).includes('incorrect digit'));
      await key('Control+z');await ready('1 incorrect digit');await key('Control+y');assert(!(await label()).includes('incorrect digit'));
      await key('Control+z');await ready('1 incorrect digit');
      const beforeReload=await raw();await p.reload();await ready('Digit mode');await ready('1 incorrect digit');await ready('Selected cells: none.');assert.equal(await raw(),beforeReload);
      await tap(l.cell(i));await tap(l.erase);assert(!(await label()).includes('incorrect digit'));await key('Control+z');await ready('1 incorrect digit');await key('Control+y');assert(!(await label()).includes('incorrect digit'));
      // Both note styles use the lighter shade; duplicates count once.
      const blanks=g.puzzle.givens.map((v,i)=>!v?i:-1).filter(i=>i>=0),j=blanks[1],k=blanks[2];
      await key('x');await key('1');await tap(l.cell(j));await key('c');await key('1');await tap(l.cell(k));await key('1');await key('x');await key('1');
      const notes=await raw();await key('Control+Shift+a');await ready('Selected cells: none.');await key('1');const found=await highlighted(1);assert(!found.includes(i));assert.equal(await raw(),notes);assert.deepEqual(candidates(await game(),1),[i,j,k]);
      const noteShot=`/tmp/jarcade-sudoku-candidates-${w}-${saver}.png`;await p.screenshot({path:noteShot});
      const pale=saver?[14,11,27]:[246,242,255];for(const colour of pixels(noteShot,[i,j,k].map(sample)))assert(colour.every((v,n)=>Math.abs(v-pale[n])<=3),`Both note styles must visibly use the lighter shade: ${colour}`);
      await key('1');assert(!(await label()).includes('Candidate digit'));assert.equal(await raw(),notes);
      const clearShot=`/tmp/jarcade-sudoku-candidates-cleared-${w}-${saver}.png`;await p.screenshot({path:clearShot});assert.deepEqual(pixels(clearShot,[sample(i)])[0],saver?[0,0,0]:[255,255,255]);
      await key('1');await highlighted(1);
      await p.mouse.click(...l.cell(i));await p.waitForTimeout(90);assert(!(await label()).includes('Highlighted digit'));await p.mouse.click(...outside);await p.waitForTimeout(90);await ready('Selected cells: none.');
      // A digit lookup followed by an arrow resumes selection at r1c1.
      await key('1');await highlighted(1);await key('ArrowRight');await ready('Selected cells: r1c1.');assert(!(await label()).includes('Highlighted digit'));await key('Control+Shift+a');await ready('Selected cells: none.');
      const beforeIdle=await raw();await tap(l.number(4));await highlighted(4);assert.equal(await raw(),beforeIdle);
      await p.waitForTimeout(350);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>window.__frames),frames,'Search and warnings must not force idle redraws');
      await c.close();console.log(`Feedback/search passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);console.log('Immediate errors, correction/erase/undo/redo/reload, deselection, filled-digit highlighting, lighter corner/centre candidates, hidden-answer exclusion, unchanged history and idle rendering passed.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
