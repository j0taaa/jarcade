// Count visible large digits only, with immediate visual updates and usable lookup.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const {play,setup}=require('./sudoku-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
function greyPixels(path,point){
 const result=spawnSync('python3',['-c',`from PIL import Image
import sys
im=Image.open(sys.argv[1]).convert('RGB'); x,y=map(float,sys.argv[2:]); crop=im.crop((int((x-12)*3),int((y-15)*3),int((x+12)*3),int((y+15)*3)))
print(sum(all(abs(v-140)<=3 for v in rgb) for rgb in crop.getdata()))`,path,...point.map(String)],{encoding:'utf8'});
 assert.equal(result.status,0,result.stderr);return Number(result.stdout.trim());
}
(async()=>{
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});const errors=[];
 try{
 const cases=process.env.JARCADE_QA_FOCUS==='smoke'?[[390,844,false]]:[[390,844,false],[390,844,true],[320,568,false],[568,320,false],[820,1180,false],[1440,900,false]];
 for(const[w,h,saver]of cases){
  const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
  await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
  const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
  const ready=s=>p.waitForFunction(s=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes(s),s);
  const raw=()=>p.evaluate(()=>localStorage.getItem('jarcade.sudoku.v1'));
  const key=async k=>{await p.keyboard.press(k);await p.waitForTimeout(90);};
  const tap=async xy=>{await p.touchscreen.tap(...xy);await p.waitForTimeout(90);};
  const shot=`/tmp/jarcade-sudoku-numberpad-${w}-${h}-${saver}.png`;
  const grey=async()=>{await p.screenshot({path:shot});return greyPixels(shot,l.number(n));};
  await p.goto(base+'/games/sudoku');await ready('Choose a variant');await tap(setup(w,h).new);await ready('Digit mode');
  const g=JSON.parse(await raw()).game,l=play(w,h);
  const n=[1,2,3,4,5,6,7,8,9].find(n=>g.puzzle.givens.filter(v=>v===n).length<8);
  const cells=g.puzzle.solution.map((d,i)=>d===n&&!g.puzzle.givens[i]?i:-1).filter(i=>i>=0),last=cells.at(-1);
  await tap(l.cell(last));await key('x');await key(String(n));await key('c');await key(String(n));await key('z');
  assert.equal(await grey(),0,'Corner/centre notes and hidden solutions never dim keys');
  for(const i of cells.slice(0,-1)){await tap(l.cell(i));await key(String(n));}
  assert.equal(await grey(),0,'Eight visible instances keep the normal key');
  await tap(l.cell(last));await key(String(n));assert((await grey())>40,'The ninth entry immediately turns the number grey');
  const filled=await raw();await p.keyboard.press('Control+Shift+a');await tap(l.number(n));await ready(`Highlighted digit ${n}:`);assert.equal(await raw(),filled);assert((await grey())>40,'Highlighted completed digits remain grey');
  await tap(l.cell(last));await key('Delete');assert.equal(await grey(),0,'Erase restores the normal key');
  await key('Control+z');assert((await grey())>40,'Undo restores the completed key');await key('Control+y');assert.equal(await grey(),0,'Redo restores the normal key');await key('Control+z');
  await key('v');assert.equal(await grey(),0,'Colour swatches remain usable');await key('z');assert((await grey())>40);
  const saved=await raw();await p.reload();await ready('Digit mode');assert.equal(await raw(),saved);assert((await grey())>40,'Saved digits still dim after reload');
  // Keyboard focus also announces completion, without adding visible UI text.
  for(let i=0;i<=n+1;i++)await key('Tab');await ready(`${n}: all nine instances filled`);
  await p.waitForTimeout(300);const frames=await p.evaluate(()=>window.__frames);await p.waitForTimeout(300);assert.equal(await p.evaluate(()=>window.__frames),frames,'Completed pad must stay idle with FPS enabled');
  await c.close();console.log(`Number pad passed: ${w}×${h}, ${saver?'black':'white'}`);
 }
 assert.deepEqual(errors,[]);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
