// Exercise the icon controls, visual help and responsive gallery in a real browser.
const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const {mkdirSync}=require('node:fs');
const {setup}=require('./nonograms-layout.cjs');
const base=process.env.JARCADE_QA_URL||'http://127.0.0.1:8092';
const artifacts='/tmp/jarcade-nonograms-ui-qa';mkdirSync(artifacts,{recursive:true});
const layouts=process.env.JARCADE_QA_FOCUS==='compact'?[[280,360],[390,844],[568,320]]:[[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]];
const utility=(w,h,i)=>w>=540&&h<500?[38+(i%3)*48,i<3?86:142]:[(w-Math.min(w-16,600))/2+(i<3?i*44:Math.min(w-16,600)-132+(i-3)*44)+22,80];
const tool=(w,h,i)=>{const wide=w>=540&&h<500,width=wide?144:Math.min(w-24,280),x=wide?16:(w-width)/2;return[x+4+(i+.5)*(width-8)/3,h-38];};
(async()=>{
  const browser=await chromium.launch({executablePath:process.env.JARCADE_CHROME||'/usr/bin/google-chrome',args:['--no-sandbox','--enable-unsafe-swiftshader']});
  const errors=[];
  const data=p=>p.evaluate(()=>JSON.parse(localStorage.getItem('jarcade.nonograms.v1')));
  const wait=p=>p.waitForTimeout(100);
  const idle=async p=>{let stable=false;for(let i=0;i<12;i++){let n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(200);if(await p.evaluate(()=>window.__frames)===n){stable=true;break;}}assert(stable);let n=await p.evaluate(()=>window.__frames);await p.waitForTimeout(300);assert.equal(await p.evaluate(()=>window.__frames),n);};
  try{
    for(const saver of [false,true])for(const[w,h]of layouts){
      const c=await browser.newContext({viewport:{width:w,height:h},deviceScaleFactor:3,hasTouch:true});
      await c.addInitScript(saver=>{localStorage.setItem('jarcade.settings.v1',`3 ${saver?1:0} 1 0 1`);const raf=requestAnimationFrame;window.__frames=0;window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t);});},saver);
      const p=await c.newPage();p.on('pageerror',e=>errors.push(e.message));
      await p.goto(`${base}/?game=nonograms`);await p.waitForFunction(()=>document.querySelector('canvas')?.getAttribute('aria-label')?.includes('Endless puzzles'));
      await idle(p);await p.screenshot({path:`${artifacts}/endless-${w}x${h}-${saver}.png`,scale:'css'});
      await p.touchscreen.tap(...setup(w,h).mode(1));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Choose a picture'));
      await p.screenshot({path:`${artifacts}/gallery-${w}x${h}-${saver}.png`,scale:'css'});
      await p.touchscreen.tap(w-30,28);await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Close help'));
      await p.screenshot({path:`${artifacts}/help-${w}x${h}-${saver}.png`,scale:'css'});
      const height=Math.min(h-24,338);await p.touchscreen.tap(w/2,(h-height)/2+height-39);await wait(p);
      await p.touchscreen.tap(...setup(w,h).play);await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Fill mode'));
      await p.touchscreen.tap(...utility(w,h,1));await wait(p);
      assert.equal((await data(p)).boards[0].hints,1,'The bulb must provide a hint');
      await p.touchscreen.tap(...utility(w,h,0));await wait(p);
      assert.equal((await data(p)).boards[0].cells.filter(Boolean).length,0,'The curved arrow must undo');
      await p.touchscreen.tap(...tool(w,h,1));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Cross mode'));
      await p.keyboard.press('Space');await wait(p);
      assert((await data(p)).boards[0].cells.includes(2));
      await p.touchscreen.tap(...tool(w,h,2));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Move mode'));
      const before=await data(p);await p.keyboard.press('Space');await wait(p);assert.deepEqual(await data(p),before,'Move must not paint');
      await p.touchscreen.tap(...tool(w,h,0));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Fill mode'));
      for(const i of [5,3,4]){await p.touchscreen.tap(...utility(w,h,i));await wait(p);assert.deepEqual(await data(p),before,'Zoom controls must not paint');}
      await p.touchscreen.tap(...utility(w,h,2));await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Clear this picture?'));
      const width=Math.min(w-24,460),x=(w-width)/2;await p.touchscreen.tap(x+16+(width-42)/4,(h-198)/2+159);await wait(p);
      assert.deepEqual(await data(p),before,'Cancel reset must preserve progress');
      await p.mouse.move(...utility(w,h,1));await wait(p);
      await p.screenshot({path:`${artifacts}/board-${w}x${h}-${saver}.png`,scale:'css'});
      await p.mouse.move(0,0);await idle(p);
      // The toolbar remains usable with keyboard-only focus.
      for(let i=0;i<4;i++){await p.keyboard.press('Tab');await p.waitForTimeout(70);}
      assert((await p.locator('canvas').getAttribute('aria-label')).includes('Focused control: Hint'),'Focused icons must have spoken labels');
      await p.screenshot({path:`${artifacts}/focus-${w}x${h}-${saver}.png`,scale:'css'});
      await p.keyboard.press('Enter');await wait(p);
      assert.equal((await data(p)).boards[0].hints,2,'Keyboard focus must activate the bulb');
      await p.keyboard.press('Escape');await p.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Choose a picture'));
      await idle(p);await p.screenshot({path:`${artifacts}/progress-${w}x${h}-${saver}.png`,scale:'css'});
      await c.close();console.log(`Icon UI passed: ${w}×${h}, ${saver?'black':'white'}`);
    }
    assert.deepEqual(errors,[]);console.log('Nonograms UI: tool icons, undo/hint/reset, zoom, visual help, keyboard navigation, thumbnails and idle rendering passed.');
  }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
