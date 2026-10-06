// Real-browser checks for original puzzles, painting gestures and offline saves.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const artifacts = '/tmp/jarcade-nonograms-qa';
mkdirSync(artifacts, { recursive: true });
const sizes = [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]];
const clueGeometry = (w, h, side, left, top) => {
  const landscape = w >= 540 && h < 500;
  const boardWidth=Math.min(w-20,720);
  const view = landscape ? [210,58,w-220,h-70] : [(w-boardWidth)/2,106,boardWidth,Math.max(62,h-228)];
  const zoom = Math.max(.69,Math.min(w>=600?2.5:1.6,view[2]/((side+left)*32),view[3]/((side+top)*32)));
  const unit = 32*zoom;
  const x = view[0] + Math.max(0,(view[2]-(side+left)*unit)/2)+left*unit;
  const y = view[1] + Math.max(0,(view[3]-(side+top)*unit)/2)+top*unit;
  return { view, unit, cell: (xCell,yCell) => [x+(xCell+.5)*unit,y+(yCell+.5)*unit] };
};
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args:['--no-sandbox','--enable-unsafe-swiftshader'] });
  const errors = [];
  try {
    const contextFor = async (w,h,saver=false) => {
      const c = await browser.newContext({ viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true });
      await c.addInitScript(saver => {
        localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
        const raf = requestAnimationFrame; window.__frames=0; window.__sockets=0;
        window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});
        const Socket=WebSocket;window.WebSocket=class extends Socket{constructor(...a){super(...a);window.__sockets++;}};
      },saver);
      const p = await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      await p.goto(`${base}/?game=nonograms`);await p.waitForFunction(()=>!document.getElementById('loading'));
      await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label')?.includes('Endless puzzles'));
      const width=Math.min(w-24,720),x=(w-width)/2;
      await p.touchscreen.tap(x+width*.75,152);
      await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label')?.includes('Choose a picture'));
      return [c,p];
    };
    const label=p=>p.locator('canvas').getAttribute('aria-label');
    const idle=async p=>{let stable=false;for(let i=0;i<12;i++){const n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(250);if((await p.evaluate(()=>window.__frames))===n){stable=true;break;}}assert(stable,'Rendering must settle after the input');const n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(400);assert.equal(await p.evaluate(()=>window.__frames),n,'Nonograms should sleep when idle, including with FPS enabled');};
    if(process.env.JARCADE_QA_FOCUS !== 'gameplay') for (const saver of [false,true]) for (const [w,h] of sizes) {
      const [c,p]=await contextFor(w,h,saver);
      assert.deepEqual(await p.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);
      assert.equal(await p.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');
      await idle(p);await p.screenshot({path:`${artifacts}/setup-${w}x${h}-${saver}.png`,scale:'css'});
      for (const [number,side] of [5,10,15].entries()) {
        const width=Math.min(w-24,720),x=(w-width)/2;
        await p.touchscreen.tap(x+4+(number+.5)*(width-8)/3,92);
        await p.touchscreen.tap(w/2,h-36);
        await p.waitForFunction(side=>document.querySelector('canvas').getAttribute('aria-label').includes(`${side} × ${side}. Fill mode`),side);
        await idle(p);await p.screenshot({path:`${artifacts}/board-${side}-${w}x${h}-${saver}.png`,scale:'css'});
        await p.keyboard.press('ArrowRight');await p.keyboard.press('Space');await p.waitForTimeout(100);
        const saved=await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
        assert.equal(saved.boards.find(b=>b.id===saved.selected).cells[1],1);
        await p.keyboard.press('u');await p.waitForTimeout(100);
        const undone=await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
        assert.equal(undone.boards.find(b=>b.id===undone.selected)?.cells[1]||0,0);
        await p.keyboard.press('v');await p.keyboard.press('Space');await p.waitForTimeout(70);
        assert.equal(await p.evaluate(()=>localStorage.getItem('jarcade.nonograms.v1')),JSON.stringify(undone),'Move must never paint');
        await p.keyboard.press('Escape');await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Choose a picture'));
      }
      assert.equal(await p.evaluate(()=>window.__sockets),0,'Nonograms is fully offline');
      await c.close();
      console.log(`Nonogram layouts checked: ${w}×${h}, ${saver ? 'black' : 'white'}`);
    }
    const [c,p]=await contextFor(390,844);
    await p.touchscreen.tap(195,808);await p.waitForTimeout(100);
    const geometry=clueGeometry(390,844,5,1.64,1.06);
    await p.mouse.move(...geometry.cell(0,1));await p.mouse.down();await p.mouse.move(...geometry.cell(4,1));await p.mouse.up();await p.waitForTimeout(100);
    let progress=await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
    assert.deepEqual(progress.boards[0].cells.slice(5,10),[1,1,1,1,1],'Fast pointer strokes must paint crossed cells');
    await p.keyboard.press('u');await p.waitForTimeout(70);
    assert.equal((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards.length,0,'One Undo clears one complete stroke');
    // Explicitly deliver a complete mouse stroke within one JS task/frame.
    await p.evaluate(({from,to})=>{const canvas=document.querySelector('canvas');for(const [type,point]of [['mousemove',from],['mousedown',from],['mousemove',to],['mouseup',to]])canvas.dispatchEvent(new MouseEvent(type,{clientX:point[0],clientY:point[1],button:0,bubbles:true}));},{from:geometry.cell(0,1),to:geometry.cell(4,1)});
    await p.waitForTimeout(120);
    assert.deepEqual((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards[0].cells.slice(5,10),[1,1,1,1,1],'A complete stroke delivered between display frames must keep its down-point');
    await p.keyboard.press('u');await p.waitForTimeout(70);
    // A complete two-finger gesture arriving between frames must not paint.
    await p.evaluate(points=>{const canvas=document.querySelector('canvas');const changedTouches=points.map((point,index)=>new Touch({identifier:100+index,target:canvas,clientX:point[0],clientY:point[1]}));for(const type of ['touchstart','touchmove','touchend'])canvas.dispatchEvent(new TouchEvent(type,{changedTouches,bubbles:true,cancelable:true}));},[geometry.cell(0,1),geometry.cell(3,1)]);
    await p.waitForTimeout(120);
    assert.equal((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards.length,0,'A complete collapsed two-finger gesture must never paint');
    // A second finger changes the gesture to zoom and rolls back its first paint.
    const cdp=await c.newCDPSession(p),a=geometry.cell(0,1),b=geometry.cell(3,1);
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{id:1,x:a[0],y:a[1]}]});await p.waitForTimeout(40);
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{id:1,x:a[0],y:a[1]},{id:2,x:b[0],y:b[1]}]});await p.waitForTimeout(40);
    await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{id:1,x:a[0]-10,y:a[1]},{id:2,x:b[0]+10,y:b[1]}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await p.waitForTimeout(70);
    assert.equal((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards.length,0,'Pinch must roll back the whole interrupted paint stroke');
    // Keyboard hints correct a wrong fill without penalties.
    for(let i=0;i<5;i++)await p.keyboard.press('ArrowLeft');for(let i=0;i<5;i++)await p.keyboard.press('ArrowUp');await p.keyboard.press('f');await p.keyboard.press('Space');await p.keyboard.press('h');await p.waitForTimeout(100);
    progress=await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
    assert.equal(progress.boards[0].cells[0],2);assert.equal(progress.boards[0].hints,1);
    await p.keyboard.press('r');await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Clear this picture?'));
    await p.touchscreen.tap(282,461);await p.waitForTimeout(100);
    assert.equal((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards[0].cells.filter(Boolean).length,0);
    await p.keyboard.press('u');await p.waitForTimeout(70);
    assert.equal((await p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')))).boards[0].cells[0],2);
    // Solve a full picture in the actual app and verify persistence on reload.
    for(let i=0;i<25 && !(await label(p)).includes('Picture complete');i++){await p.keyboard.press('h');await p.waitForTimeout(35);}
    assert((await label(p)).includes('Picture complete: Heart'));
    await idle(p);await p.screenshot({path:`${artifacts}/completed.png`,scale:'css'});
    await p.reload();await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Completed'));
    await p.touchscreen.tap(195,808);await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Picture complete: Heart'));
    await idle(p);await c.close();assert.deepEqual(errors,[]);
    console.log('Nonograms: fast drag painting, whole-stroke undo, pinch rollback, hints/reset, full win, offline restore and idle frames passed.');
  } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
