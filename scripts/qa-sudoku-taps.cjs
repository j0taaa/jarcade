// Tap toggles and double-tap entry must stay distinct from painting and panning.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {play,setup}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
  try{
    const cases=process.env.JARCADE_QA_FOCUS==='desktop'?[[1440,900,false]]:[[390,844,false],[390,844,true],[1440,900,false],[568,320,false]];
    for(const[w,h,saver]of cases){
      const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:Number(process.env.JARCADE_QA_DPI||3),hasTouch:true});
      await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});window.__tapTimes=[];document.addEventListener('touchend',()=>window.__tapTimes.push(performance.now()),{passive:true});},saver);
      const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      const ready=async s=>{try{await p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:30000});}catch(e){console.error(await p.evaluate(()=>({label:document.querySelector('canvas')?.getAttribute('aria-label'),tapTimes:window.__tapTimes.slice(-4),marks:JSON.parse(localStorage.getItem('jarcade.sudoku.v1'))?.game.marks.slice(0,9)})));await p.screenshot({path:'/tmp/jarcade-sudoku-taps-failure.png'});throw e;}};
      const label=()=>p.locator('canvas').getAttribute('aria-label');
      const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
      const game=async()=>JSON.parse(await raw()).game;
      const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(70);};
      const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(80);};
      const doubleTap=async xy=>{await tap(xy);await tap(xy);};
      const l=play(w,h),selected=i=>`Selected cells: r${Math.floor(i/9)+1}c${i%9+1}.`;
      await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');await tap(setup(w,h).new);await ready('Digit mode');
      const g=await game(),i=g.puzzle.givens.findIndex(v=>!v),correct=g.puzzle.solution[i],wrong=correct%9+1,initial=await raw();
      await tap(l.cell(i));await ready(selected(i));await tap(l.cell(i));await ready('Selected cells: none.');assert.equal(await raw(),initial,'No-note double tap only toggles selection');
      await p.mouse.click(...l.cell(i));await p.waitForTimeout(80);await ready(selected(i));await p.mouse.click(...l.cell(i));await p.waitForTimeout(80);await ready('Selected cells: none.');assert.equal(await raw(),initial);
      // Space uses the first two tools; all four remain directly accessible.
      for(const[shortcut,mode]of[['z','Digit'],['x','Corner notes'],['c','Centre notes'],['v','Colour']]){
        await key(shortcut);await ready(`${mode} mode`);await key('Space');await ready(`${shortcut==='z'?'Corner notes':'Digit'} mode`);await key('Space');await ready(`${shortcut==='z'?'Digit':'Corner notes'} mode`);
      }
      await tap(l.cell(i));await ready(selected(i));await key('x');await key(String(wrong));const notes=await game(),beforeFill=await raw();
      await doubleTap(l.cell(i));await ready('1 incorrect digit');await ready('Corner notes mode');await ready(selected(i));
      let filled=await game();assert.equal(filled.marks[i][0],wrong,'Fill the user candidate, not the hidden answer');assert.equal(filled.marks[i][1]|filled.marks[i][2],0);assert.equal(filled.hints,0);assert.equal(filled.undo.length,notes.undo.length+1);
      await key('Control+z');assert.deepEqual((await game()).marks,notes.marks);await key('Control+y');assert.equal((await game()).marks[i][0],wrong);assert.notEqual(await raw(),beforeFill);
      await key('z');await key('Delete');await key('c');await key(String(correct));await key('Control+Shift+a');
      await p.mouse.dblclick(...l.cell(i),{delay:80});await p.waitForTimeout(100);await ready(selected(i));assert.equal((await game()).marks[i][0],correct);assert(!(await label()).includes('incorrect digit'));await ready('Centre notes mode');
      // Identical corner/centre notes are one distinct candidate.
      await key('z');await key('Delete');await key('x');await key(String(correct));await key('c');await key(String(correct));await doubleTap(l.cell(i));assert.equal((await game()).marks[i][0],correct);
      // Two distinct user notes, even if one is the solution, never auto-fill.
      await key('z');await key('Delete');await key('x');await key(String(correct));await key('c');await key(String(wrong));await key('Control+Shift+a');const twoNotes=await raw();await doubleTap(l.cell(i));await ready('Selected cells: none.');assert.equal(await raw(),twoNotes);
      await tap(l.cell(i));await key('c');await key(String(wrong));await key('Control+Shift+a');const singleNote=await raw();
      // Slow taps deselect rather than fill a sole candidate.
      await tap(l.cell(i));await p.waitForTimeout(420);await tap(l.cell(i));await ready('Selected cells: none.');assert.equal(await raw(),singleNote);
      // A drag returning to its starting cell must not count as the second tap.
      await p.waitForTimeout(420);await tap(l.cell(i));await p.mouse.move(...l.cell(i));await p.mouse.down();await p.mouse.move(...l.cell((i+1)%81),{steps:6});await p.mouse.move(...l.cell(i),{steps:6});await p.mouse.up();await p.waitForTimeout(90);assert.equal(await raw(),singleNote);
      await key('Control+Shift+a');await tap(l.cell(i));const beforePinch=await raw();
      const client=await c.newCDPSession(p),a=l.cell(20),b=l.cell(60);
      await client.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:a[0],y:a[1],id:1},{x:b[0],y:b[1],id:2}]});await p.waitForTimeout(60);
      await client.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:a[0]-15,y:a[1]-15,id:1},{x:b[0]+15,y:b[1]+15,id:2}]});await p.waitForTimeout(60);
      await client.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await p.waitForTimeout(90);assert.equal(await raw(),beforePinch);await p.mouse.move(...l.cell(40));await p.mouse.wheel(0,1000);await p.waitForTimeout(100);await tap(l.cell(i));assert.equal(await raw(),beforePinch,'Pinch and wheel zoom must break a double tap');
      // At a zoomed board, panning never enters a value; a real double tap does.
      await key('Control+Shift+a');await p.mouse.move(...l.cell(i));await p.mouse.wheel(0,-100);await p.waitForTimeout(100);const beforePan=await raw();
      await p.mouse.down();await p.mouse.move(l.cell(i)[0]+25,l.cell(i)[1]+25,{steps:6});await p.mouse.move(...l.cell(i),{steps:6});await p.mouse.up();await p.waitForTimeout(90);assert.equal(await raw(),beforePan);
      await doubleTap(l.cell(i));assert.equal((await game()).marks[i][0],correct);await p.mouse.move(...l.cell(40));await p.mouse.wheel(0,1000);await p.waitForTimeout(100);
      const final=await raw();await p.reload();await ready('Digit mode');assert.equal(await raw(),final);
      await tap([w-30,30]);await ready('Rules.');await p.screenshot({path:`/tmp/jarcade-sudoku-tap-help-${w}-${saver}.png`});await key('Escape');await ready('Digit mode');
      await p.waitForTimeout(350);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>window.__frames),frames,'Tap handling must not force idle redraws');
      await c.close();console.log(`Tap controls passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);console.log('Touch/mouse deselection, user-note double-tap fill, duplicates/multiple notes, warnings, undo/redo, two-tool Space, drag/pinch/pan safety, zoomed taps, persistence and idle rendering passed.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
