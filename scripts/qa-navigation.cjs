const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {setup,play}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
 const errors=[];
 try {
 const cases=process.env.JARCADE_QA_FOCUS==='desktop'?[[1440,900,false,false]]:process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false,true]]:[[390,844,false,true],[320,568,false,true],[568,320,false,true],[820,1180,false,true],[390,844,true,true],[1440,900,false,false]];
 for(const[w,h,saver,mobile]of cases){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:mobile,isMobile:mobile});
  await c.addInitScript(saver=>{
   localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
   const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});
  },saver);
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s);
  const tap=async xy=>{await p.mouse.click(...xy);await p.waitForTimeout(100);};
  const idle=async()=>{await p.waitForTimeout(400);const f=await p.evaluate(()=>window.__frames);await p.waitForTimeout(350);assert.equal(await p.evaluate(()=>window.__frames),f,'Navigation must sleep with FPS enabled');};
  const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
  await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');assert.equal(new URL(p.url()).pathname,'/games/sudoku');
  await tap(setup(w,h).new);await ready('Digit mode');assert.equal(new URL(p.url()).pathname,'/games/sudoku/play');
  const saved=JSON.parse(await raw()),i=saved.game.puzzle.givens.findIndex(v=>!v);
  await tap(play(w,h).cell(i));await p.keyboard.press('x');await p.keyboard.press('4');await p.waitForTimeout(100);
  const before=await raw(), beforeLabel=await p.locator('canvas').getAttribute('aria-label');
  if(mobile){
   await p.evaluate(()=>history.back());await ready('Jarcade. Navigation.');
   assert.equal(new URL(p.url()).pathname,'/games/sudoku/play');await idle();
   await p.screenshot({path:`/tmp/jarcade-navigation-${w}-${h}-${saver}.png`,scale:'css'});
   await p.keyboard.press('8');assert.equal(await raw(),before,'Sidebar consumes gameplay input');
   assert(!(await p.locator('canvas').getAttribute('aria-label')).includes('Settings'),'App settings stay outside games');
   await p.keyboard.press('Escape');await ready('Corner notes mode');assert.equal(await p.locator('canvas').getAttribute('aria-label'),beforeLabel);assert.equal(await raw(),before);
   const n=await p.evaluate(()=>history.length);await p.reload();await ready('Digit mode');assert.equal(await raw(),before);assert.equal(await p.evaluate(()=>history.length),n,'Reload must not grow guard history');
   await p.evaluate(()=>history.back());await ready('Navigation.');await p.evaluate(()=>history.back());await ready('Choose a variant');assert.equal(new URL(p.url()).pathname,'/games/sudoku');
   await p.evaluate(()=>history.forward());await ready('Navigation.');await p.keyboard.press('Escape');await ready('Digit mode');assert.equal(await raw(),before);
   await p.evaluate(()=>history.back());await ready('Navigation.');await tap([150,192]);await ready('Select Snake');assert.equal(new URL(p.url()).pathname,'/');
  } else {
   await p.evaluate(()=>history.back());await ready('Choose a variant');assert.equal(new URL(p.url()).pathname,'/games/sudoku');
   await p.evaluate(()=>history.forward());await ready('Corner notes mode');assert.equal(await raw(),before);
   await p.goto(`${base}/`);await ready('Select Snake');
  }
  const width=Math.min(w-48,1040),x=(w-width)/2;
  // Main menu control, background theme and all sidebar rows remain reachable.
  await tap([x+22,42]);await ready('Navigation.');await idle();
  await tap([150,212]);await ready('Jarcade. Settings.');assert.equal(new URL(p.url()).pathname,'/settings');
  await p.reload();await ready('Settings.');assert.equal(await p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');
  // Clean URLs survive refresh; old invitation URLs retain public room codes.
  for(const[path,label]of [['/multiplayer','Select Coupe'],['/games/fih/bedroom','Bedroom'],['/games/nonograms','Nonograms.'],['/games/coupe?room=ABC123','Coupe.'],['/games/dicksit','Dicksit.'],['/games/wolvesville','Wolvesville.'],['/games/codenames','Codenames.'],['/games/wavelength','Wavelength.'],['/games/table-tennis','Table tennis.'],['/games/snake','Snake.'],['/games/minesweeper','Minesweeper.']]){
   await p.goto(base+path);await ready(label);assert.equal(new URL(p.url()).pathname,path.split('?')[0]);
  }
  await p.goto(`${base}/?game=reverie&room=ABC123`);await ready('Dicksit.');assert.equal(new URL(p.url()).pathname,'/games/dicksit');assert.equal(new URL(p.url()).search,'?room=ABC123');
  assert.equal((await p.request.get(base+'/rooms.json')).status(),404);assert.equal((await p.request.get(base+'/games/unknown')).status(),404);
  if(mobile && w===390 && !saver){
   await p.goto(`${base}/games/wavelength`);await ready('. Ready.');await p.keyboard.press('Space');await ready('. Peek.');
   await p.evaluate(()=>history.back());await ready('Navigation.');
   assert.equal(await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.wavelength.v1')).phase),'Ready');
   assert(!/target \d+ percent/i.test(await p.locator('canvas').getAttribute('aria-label')));
   await p.keyboard.press('Escape');await ready('. Ready.');await idle();
  }
  await c.close();console.log(`Navigation passed: ${w}×${h}, ${saver?'black':'white'}, ${mobile?'touch':'desktop'}`);
 }
 assert.deepEqual(errors,[]);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
