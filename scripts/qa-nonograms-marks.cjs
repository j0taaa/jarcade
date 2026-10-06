// Space switches tools; every route must clear before applying an opposite mark.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {setup}=require('./nonograms-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
  const errors=[];
  try{
    for(const saver of [false,true])for(const[w,h]of [[390,844],[1440,900]]){
      const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
      await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
      const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.nonograms.v1'));
      const cells=async()=>JSON.parse(await raw()).boards.find(b=>b.id==='heart')?.cells||Array(25).fill(0);
      const mode=m=>p.waitForFunction(m=>document.querySelector('canvas').getAttribute('aria-label').includes(`${m} mode`),m);
      const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(65);};
      const mark=async n=>assert.equal((await cells())[0],n);
      await p.goto(`${base}/?game=nonograms`);await p.waitForFunction(()=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes('Endless puzzles'));
      await p.touchscreen.tap(...setup(w,h).mode(1));await p.waitForTimeout(80);await p.touchscreen.tap(...setup(w,h).play);await mode('Fill');
      let before=await raw();await key('Space');await mode('Cross');assert.equal(await raw(),before);await key('Space');await mode('Fill');assert.equal(await raw(),before);
      await key('Enter');await mark(1);before=await raw();await key('Space');await mode('Cross');assert.equal(await raw(),before);
      await key('x');await mark(0);await key('x');await mark(2);
      await key('Space');await mode('Fill');await key('Enter');await mark(0);await key('Enter');await mark(1);
      const width=Math.min(w-24,780),zoom=Math.max(.69,Math.min(2.5,width/(6.64*32),(h-192)/(6.06*32))),u=32*zoom;
      const x=(w-width)/2+Math.max(0,(width-6.64*u)/2)+1.64*u;
      const y=112+Math.max(0,(h-192-6.06*u)/2)+1.06*u;
      const cell=i=>[x+(i%5+.5)*u,y+(Math.floor(i/5)+.5)*u];
      const tap=async i=>{await p.touchscreen.tap(...cell(i));await p.waitForTimeout(80);};
      await key('Space');await mode('Cross');await tap(0);await mark(0);await tap(0);await mark(2);
      await key('Space');await mode('Fill');await p.mouse.click(...cell(0),{button:'right'});await p.waitForTimeout(80);await mark(0);
      await p.mouse.click(...cell(0),{button:'right'});await p.waitForTimeout(80);await mark(2);
      await p.mouse.click(...cell(0));await p.waitForTimeout(80);await mark(0);await p.mouse.click(...cell(0));await p.waitForTimeout(80);await mark(1);
      // Reset, then create Cross, Cross, Fill, Blank across the first row.
      await key('r');await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Clear this picture?'));
      const modalWidth=Math.min(w-24,460),modalX=(w-modalWidth)/2;
      await p.touchscreen.tap(modalX+26+(modalWidth-42)*.75,(h-198)/2+159);await p.waitForTimeout(80);
      await key('c');await tap(0);await tap(1);await key('f');await tap(2);
      assert.deepEqual((await cells()).slice(0,4),[2,2,1,0]);
      const drag=async revisit=>{await p.mouse.move(...cell(3));await p.mouse.down();await p.mouse.move(...cell(0),{steps:4});if(revisit)await p.mouse.move(...cell(3),{steps:4});await p.mouse.up();await p.waitForTimeout(100);};
      await drag(true);assert.deepEqual((await cells()).slice(0,4),[0,0,1,1],'Fill drag must only clear crosses, even when revisited');
      await key('u');assert.deepEqual((await cells()).slice(0,4),[2,2,1,0]);
      await key('c');await drag(true);assert.deepEqual((await cells()).slice(0,4),[2,2,0,2],'Cross drag must only clear blocks');
      await key('u');assert.deepEqual((await cells()).slice(0,4),[2,2,1,0]);
      before=await raw();await key('f');await p.mouse.move(...cell(3));await p.mouse.down();await p.mouse.move(...cell(0));await p.waitForTimeout(65);
      await key('Space');await mode('Cross');await p.mouse.up();await p.waitForTimeout(100);assert.equal(await raw(),before,'Switching tools must cancel the unfinished stroke');
      await key('v');await mode('Move');await key('Enter');assert.equal(await raw(),before);await key('Space');await mode('Fill');assert.equal(await raw(),before);
      await key('Tab');await key('Space');await mode('Cross');assert.equal(await raw(),before,'Space must switch tools while a toolbar button has focus');
      // Hints also clear contradictions before replacing their marks.
      await key('h');assert.equal((await cells())[1],0);await key('h');assert.equal((await cells())[2],0);await key('h');assert.equal((await cells())[1],1);
      const stored=await raw();await p.reload();await p.waitForFunction(()=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes('Choose a picture'));
      await p.touchscreen.tap(...setup(w,h).play);await mode('Fill');assert.equal(await raw(),stored);
      await p.waitForTimeout(350);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(350);assert.equal(await p.evaluate(()=>window.__frames),frames,'Idle FPS must not redraw');
      await c.close();console.log(`Mark transitions passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);console.log('Space switching, clear-first keyboard/touch/mouse/right-click/drags/hints, stroke cancellation, undo and saved progress passed.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
