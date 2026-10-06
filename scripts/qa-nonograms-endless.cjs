// Endless generation, size-specific offline saves, migration and idle rendering.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const artifacts = '/tmp/jarcade-endless-qa';
const { setup } = require('./nonograms-layout.cjs');
mkdirSync(artifacts, { recursive: true });
const layouts = process.env.JARCADE_QA_FOCUS === 'compact' ? [[280,360],[568,320]] : [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]];
(async () => {
  const browser = await chromium.launch({executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args:['--no-sandbox','--enable-unsafe-swiftshader']});
  const errors=[];
  const saved=p=>p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
  const label=p=>p.locator('canvas').getAttribute('aria-label');
  const idle=async p=>{
    let stable=false;
    for(let i=0;i<16;i++) { const n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(200);if(await p.evaluate(()=>window.__frames)===n){stable=true;break;} }
    assert(stable,'Idle rendering must settle');
    const n=await p.evaluate(()=>window.__frames),data=await saved(p);
    await p.waitForTimeout(300);
    assert.equal(await p.evaluate(()=>window.__frames),n,'FPS counter must not redraw idle screens');
    assert.deepEqual(await saved(p),data,'No puzzles should be generated or saved in the background');
  };
  try {
    for(const saver of [false,true]) for(const [w,h] of layouts) {
      const context=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
      await context.addInitScript(saver=>{
        // Settings are the only storage reset on reload; game progress must survive.
        localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);
        const raf=requestAnimationFrame;window.__frames=0;window.__sockets=0;
        window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});
        const Socket=WebSocket;window.WebSocket=class extends Socket{constructor(...a){super(...a);window.__sockets++;}};
      },saver);
      const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
      await page.goto(`${base}/?game=nonograms`);
      await page.waitForFunction(()=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes('Endless puzzles'));
      assert.equal(await saved(page),null,'Opening setup must not generate anything');
      assert.deepEqual(await page.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);
      assert.equal(await page.evaluate(()=>getComputedStyle(document.body).backgroundColor),saver?'rgb(0, 0, 0)':'rgb(255, 255, 255)');
      await idle(page);await page.screenshot({path:`${artifacts}/setup-${w}x${h}-${saver}.png`,scale:'css'});
      const l=setup(w,h);
      for(const [i,side] of [5,10,15].entries()) {
        await page.touchscreen.tap(...l.size(i));
        await page.touchscreen.tap(...l.play);
        await page.waitForFunction(side=>document.querySelector('canvas').getAttribute('aria-label').includes(`${side} × ${side}. Fill mode`),side);
        const initial=await saved(page),record=initial.endless.find(p=>p.side===side);
        assert.equal(record.number,1);assert.equal(record.solution.length,side*side);
        assert.equal(record.cells.filter(Boolean).length,0);
        await page.keyboard.press('h');await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('1 hints used'));
        await idle(page);await page.screenshot({path:`${artifacts}/board-${side}-${w}x${h}-${saver}.png`,scale:'css'});
        await page.keyboard.press('Escape');
        await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Endless puzzles'));
      }
      const progress=await saved(page);
      assert.equal(progress.endless.length,3);
      await page.reload();await page.waitForFunction(()=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes('Endless puzzles'));
      assert.deepEqual(await saved(page),progress);
      for(const [i,side] of [5,10,15].entries()) {
        await page.touchscreen.tap(...l.size(i));await page.touchscreen.tap(...l.play);
        await page.waitForFunction(side=>document.querySelector('canvas').getAttribute('aria-label').includes(`${side} × ${side}. Fill mode`),side);
        assert.equal((await saved(page)).endless.find(p=>p.side===side).hints,1);
        await page.keyboard.press('Escape');
      }
      // Complete a small puzzle through the actual UI, then request its successor.
      await page.touchscreen.tap(...l.size(0));await page.touchscreen.tap(...l.play);
      for(let i=0;i<25 && !(await label(page)).includes('Puzzle complete');i++){await page.keyboard.press('h');await page.waitForTimeout(40);}
      assert((await label(page)).includes('Puzzle complete: Puzzle 1'));
      const done=await saved(page),old=done.endless.find(p=>p.side===5);
      await page.touchscreen.tap(w>=540&&h<500?88:w/2,h-38);
      await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('5 × 5. Fill mode')&&!document.querySelector('canvas').getAttribute('aria-label').includes('Puzzle complete'));
      const next=(await saved(page)).endless.find(p=>p.side===5);
      assert.equal(next.number,2);assert.notDeepEqual(next.solution,old.solution);
      assert.equal(next.cells.filter(Boolean).length,0);assert.equal(next.hints,0);
      for(const side of [10,15]) assert.deepEqual((await saved(page)).endless.find(p=>p.side===side),done.endless.find(p=>p.side===side));
      await page.keyboard.press('Escape');await page.touchscreen.tap(...l.mode(1));
      await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Choose a picture'));
      await page.touchscreen.tap(...l.play);
      await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('5 × 5. Fill mode'));
      await page.keyboard.press('h');await page.waitForTimeout(100);
      assert.equal((await saved(page)).boards[0].hints,1);
      await page.keyboard.press('Escape');await page.touchscreen.tap(...l.mode(0));await page.touchscreen.tap(...l.play);
      await page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('5 × 5. Fill mode'));
      assert.equal((await saved(page)).selected,'endless-5');
      assert.deepEqual((await saved(page)).endless.find(p=>p.side===5),next);
      await idle(page);assert.equal(await page.evaluate(()=>window.__sockets),0);
      await context.close();console.log(`Endless passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);
    console.log('Endless: all sizes, generation, next, separate saves, reload, original pack, DPR3 and event-driven idle passed.');
  } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exitCode=1;});
