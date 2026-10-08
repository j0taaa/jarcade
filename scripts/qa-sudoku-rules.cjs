const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {setup,play}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
 const errors=[];
 try{
 const cases=process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false]]:[[390,844,false],[320,568,false],[1440,900,false],[568,320,false],[390,844,true]];
 for(const[w,h,saver]of cases){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s,{timeout:30000});
  const label=()=>p.locator('canvas').getAttribute('aria-label');
  const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
  const game=async()=>JSON.parse(await raw()).game;
  const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(100);};
  const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(100);};
  await p.goto(`${base}/?game=sudoku`);await ready('Choose a variant');const s=setup(w,h);
  for(const i of [2,3,4,5])await tap(s.variant(i));await tap(s.marking(0));await tap(s.marking(1));await tap(s.page(1));for(let i=0;i<6;i++)await tap(s.variant(i));await tap(s.page(2));await tap(s.variant(0));await tap(s.variant(1));await tap(s.new);await ready('Easy. Digit mode');
  const g=await game(),cell=g.puzzle.givens.findIndex(d=>!d);await tap(play(w,h,true).cell(cell));await key('Shift+4');
  const before=await raw(),beforeLabel=await label();
  await p.screenshot({path:`/tmp/jarcade-sudoku-rules-button-${w}-${saver}.png`});await tap([w-46,30]);await ready('Rules.');
  const text=await label();for(const rule of ['Grey arrow','Purple Renban','green Whispers','blue region-sum','Grey palindrome','orange between','gold entropic','Outside sandwich','Full XV:','Full dots:','Lavender thermometers','purple long diagonals'])assert(text.includes(rule),rule);
  assert(!text.includes('chess knight'));assert(!text.includes('Space switches'));assert.equal(await raw(),before);
  const image=()=>p.screenshot({path:`/tmp/jarcade-sudoku-rules-popup-${w}-${saver}.png`});await image();
  const initial=await p.screenshot();await key('End');const bottom=await p.screenshot();assert(!initial.equals(bottom),'Keyboard reaches bottom of long list');await p.screenshot({path:`/tmp/jarcade-sudoku-rules-bottom-${w}-${saver}.png`});await key('Home');assert(initial.equals(await p.screenshot()));
  const mh=Math.min(h-24,540),top=(h-mh)/2+62,bottomY=(h+mh)/2-76;
  await p.mouse.move(w/2,top+25);await p.mouse.wheel(0,500);await p.waitForTimeout(180);assert(!initial.equals(await p.screenshot()),'Mouse wheel scrolls list');await key('Home');
  const session=await c.newCDPSession(p);
  await session.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:w/2,y:bottomY-20}]});
  for(let i=1;i<=5;i++){await session.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:w/2,y:bottomY-20-(bottomY-top-45)*i/5}]});await p.waitForTimeout(35);}
  await session.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await p.waitForTimeout(120);assert(!initial.equals(await p.screenshot()),'Finger scrolls list');await session.detach();
  await key('8');await key('Space');assert.equal(await raw(),before,'Popup never edits digits or notes');
  await p.waitForTimeout(300);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(300);assert.equal(await p.evaluate(()=>window.__frames),frames,'Popup stays idle');
  await tap([w/2,(h+mh)/2-38]);await ready('Digit mode');assert.equal(await label(),beforeLabel);assert.equal(await raw(),before);
  await tap([w-46,30]);await ready('Rules.');await key('Escape');assert.equal(await raw(),before);assert.equal(await label(),beforeLabel);
  // A simple puzzle has just the base rule and a smaller popup.
  await key('Escape');await ready('Choose a variant');const saved=setup(w,h,true);await tap(saved.page(0));await tap(saved.variant(0));await tap(saved.new);await ready('Digit mode');await tap([w-46,30]);await ready('Rules.');assert(!(await label()).includes('Renban'));await p.screenshot({path:`/tmp/jarcade-sudoku-rules-classic-${w}-${saver}.png`});await key('Escape');
  await c.close();console.log(`Rules popup passed ${w}×${h}, ${saver?'black':'white'}: active rules, touch/wheel/keyboard scroll, Close/Escape, unchanged selection/progress, idle.`);
 }
 assert.deepEqual(errors,[]);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
