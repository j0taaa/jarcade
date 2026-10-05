// Optional browser gameplay QA. Requires Playwright, Python/Pillow/numpy and a web build.
const {chromium} = require('playwright');
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const {mkdirSync} = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8091';
const artifacts = '/tmp/jarcade-tennis-qa';
mkdirSync(artifacts, {recursive: true});
function geometry(w, h) {
  const landscape = w > h * 1.3 && h < 620;
  let x, y, width, height;
  if (landscape) {
    width = Math.min(w - 174, (h - 82) / .62, 900); height = width * .62;
    x = 162 + (w - 174 - width) / 2; y = 68 + (h - 82 - height) / 2;
  } else {
    width = Math.min(w - 32, (h - 154) * .62, 520); height = width / .62;
    x = (w - width) / 2; y = 114 + (h - 154 - height) / 2;
  }
  return {x, y, w: width, h: height, landscape,
    point: (a, b) => landscape ? [x + b * width, y + (1 - a) * height] : [x + a * width, y + b * height]};
}
function card(w, h, index) {
  const margin = w < 360 ? 16 : w < 600 ? 20 : 40, content = Math.min(w - margin * 2, 1040);
  const landscape = (w > h * 1.3 && h < 620) || (h < 360 && w >= 360);
  const cols = content >= 960 || (landscape && content >= 480) ? 4 : content >= 600 ? 3 : 2;
  const gap = cols === 2 ? 12 : 20, cw = (content - gap * (cols - 1)) / cols, rows = Math.ceil(4 / cols);
  const top = h < 500 ? 132 : h < 640 ? 208 : 238;
  const ih = Math.min(cw * .88, Math.max(44, (h - top - 24 - gap * (rows - 1)) / rows - 60));
  return [(w - content) / 2 + index % cols * (cw + gap) + cw / 2, top + Math.floor(index / cols) * (ih + 60 + gap) + (ih + 60) / 2];
}
(async () => {
  const browser = await chromium.launch({executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args: ['--no-sandbox', '--enable-unsafe-swiftshader']});
  const errors = [];
  const label = p => p.locator('canvas').getAttribute('aria-label');
  const phase = async (p, name) => {try {await p.waitForFunction(name => document.querySelector('canvas').getAttribute('aria-label').includes(`. ${name}.`), name, {timeout:5000});} catch(e) {throw Error(`${e.message}: ${await label(p)}`);}};
  async function setup(w = 390, h = 844, saver = false, home = false) {
    const context = await browser.newContext({viewport: {width: w, height: h}, deviceScaleFactor: 3, hasTouch: true});
    await context.addInitScript(saver => {
      localStorage.setItem('jarcade.settings.v1', `3 ${saver ? 1 : 0} 1 0 1`);
      window.__frames = 0; window.__sockets = 0;
      const raf = requestAnimationFrame;
      window.requestAnimationFrame = cb => raf.call(window, t => {window.__frames++; cb(t);});
      const Socket = WebSocket;
      window.WebSocket = class extends Socket {constructor(...args) {super(...args); window.__sockets++;}};
    }, saver);
    const page = await context.newPage(); page.on('pageerror', e => errors.push(e.message));
    await page.goto(home ? base : `${base}/?game=table-tennis`);
    await page.waitForFunction(() => !document.getElementById('loading'));
    return {context, page};
  }
  async function idle(p) {
    await p.waitForTimeout(650); const n = await p.evaluate(() => window.__frames);
    await p.waitForTimeout(250); assert.equal(await p.evaluate(() => window.__frames), n, 'Idle screen must stop drawing even with FPS enabled');
  }
  async function pixels(p) {
    const g = geometry(p.viewportSize().width, p.viewportSize().height);
    const result = spawnSync('python3', ['-c', `import sys,io,json,numpy as np
from PIL import Image
g=json.loads(sys.argv[1]);w=int(sys.argv[2]);h=int(sys.argv[3]);im=Image.open(io.BytesIO(sys.stdin.buffer.read())).convert('RGB').resize((w,h),Image.Resampling.NEAREST);a=np.asarray(im);out={}
for name,color in [('red',[235,106,67]),('blue',[43,94,181]),('ball',[255,249,223])]:
 mask=np.all(a==color,axis=2);mask[:int(g['y']),:]=False;mask[int(g['y']+g['h'])+1:,:]=False;mask[:,:int(g['x'])]=False;mask[:,int(g['x']+g['w'])+1:]=False;ys,xs=np.where(mask);out[name]=[float(xs.mean()),float(ys.mean())] if len(xs)>0 else None
out['background']=im.getpixel((2,120));out['court']=im.getpixel((int(g['x']+g['w']*.1),int(g['y']+g['h']*.04)));print(json.dumps(out))`, JSON.stringify(g), String(p.viewportSize().width), String(p.viewportSize().height)], {input: await p.screenshot({scale: "css"})});
    assert.equal(result.status, 0, result.stderr.toString()); return JSON.parse(result.stdout);
  }
  const closeTo = (actual, expected) => {assert(actual, 'Racket must be visible'); assert(Math.hypot(actual[0]-expected[0], actual[1]-expected[1]) < 4, `${actual} should follow ${expected}`);};
  const {context, page} = await setup(); const g = geometry(390, 844);
  await phase(page, 'Ready'); await idle(page);
  // Difficulty selection, then immediate mouse tracking without holding a button.
  await page.mouse.click(g.x + g.w/2 + 86, g.y + g.h/2 + 5); await page.waitForTimeout(40);
  assert((await label(page)).includes('. Hard.'));
  await page.screenshot({path: `${artifacts}/ready.png`});
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  await page.mouse.move(...g.point(.22,.84)); await page.waitForTimeout(40); await page.keyboard.press('Space'); await phase(page, 'Paused'); closeTo((await pixels(page)).red, g.point(.22,.84));
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  await page.mouse.move(...g.point(.77,.84)); await page.waitForTimeout(40); await page.keyboard.press('Space'); await phase(page, 'Paused'); closeTo((await pixels(page)).red, g.point(.77,.84)); await idle(page);
  const paused = await label(page);
  await page.reload(); await phase(page,'Ready');
  // Finger follows native-DPR coordinates and stays clear of the finger itself.
  const cdp = await context.newCDPSession(page);
  const touch = (type, points) => cdp.send('Input.dispatchTouchEvent', {type, touchPoints: points.map(([id,x,y]) => ({id,x,y}))});
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  const a = g.point(.3,.84), b = g.point(.7,.84);
  await touch('touchStart', [[1,a[0],a[1]+26]]); await page.waitForTimeout(35); await page.keyboard.press('Space'); await phase(page,'Paused'); closeTo((await pixels(page)).red, a);
  await touch('touchEnd', []); await page.keyboard.press('Space'); await phase(page, 'Rally');
  await touch('touchStart', [[1,a[0],a[1]+26]]); await page.waitForTimeout(35);
  await touch('touchMove', [[1,b[0],b[1]+26]]); await page.waitForTimeout(35); await page.keyboard.press('Space'); await phase(page,'Paused'); closeTo((await pixels(page)).red, b);
  await touch('touchEnd', []); await page.reload(); await phase(page,'Ready'); await page.keyboard.press('Space'); await phase(page, 'Rally');
  await touch('touchStart', [[1,b[0],b[1]+26]]); await page.waitForTimeout(35);
  await touch('touchMove', [[1,b[0],b[1]+26],[2,a[0],a[1]]]); await page.waitForTimeout(35);
  await page.keyboard.press('Space'); await phase(page,'Paused'); closeTo((await pixels(page)).red, b);
  await touch('touchCancel', []);
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  await page.evaluate(() => window.dispatchEvent(new Event('blur'))); await phase(page, 'Paused'); await idle(page);
  assert((await label(page)).includes('You 0. Computer 0.'), paused);
  // Resize pauses an active rally and reorients the table without a free point.
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  await page.setViewportSize({width:568,height:320}); await phase(page, 'Paused'); await idle(page);
  await page.screenshot({path:`${artifacts}/landscape.png`});
  await page.keyboard.press('Space'); await phase(page, 'Rally');
  const landscape = geometry(568,320), target = landscape.point(.88,.88);
  await page.mouse.move(...target); await page.waitForTimeout(40); await page.keyboard.press('Escape'); await phase(page,'Paused'); closeTo((await pixels(page)).red,target);
  await page.keyboard.press('Space'); await phase(page,'Rally'); await page.keyboard.press('Escape'); await phase(page,'Paused'); await idle(page);
  assert.equal(await page.evaluate(() => window.__sockets),0,'Solo play must never create a room connection');
  await context.close(); console.log('Pointer, touch cancellation, interruption and landscape controls passed.');
  // Complete a match through real input, verify per-point service and retry.
  const match = await setup(); await phase(match.page,'Ready');
  await match.context.setOffline(true);
  await match.page.mouse.click(g.x+g.w/2+86,g.y+g.h/2+5);
  for (let i=0;i<15;i++) {
    await match.page.keyboard.press('Space'); await phase(match.page,'Rally');
    await match.page.mouse.move(...g.point(.045,.94));
    await match.page.waitForFunction(() => /\. (Point|Finished)\./.test(document.querySelector('canvas').getAttribute('aria-label')),null,{timeout:8000});
    const text=await label(match.page); const score=[...text.matchAll(/(?:You|Computer) (\d+)\./g)].map(m=>Number(m[1]));
    assert.equal(score[0]+score[1],i+1,'Each miss awards one point');
    const total=score[0]+score[1], serving=Math.floor(total/2)%2 ? 'Computer' : 'You';
    assert(text.includes(`${serving} serves.`));
    await idle(match.page);
    if(text.includes('. Finished.')) {assert(score[1]>=11);break;}
  }
  await phase(match.page,'Finished'); await match.page.keyboard.press('Space'); await phase(match.page,'Rally');
  assert((await label(match.page)).includes('You 0. Computer 0.'));
  await match.context.close(); console.log('Offline match, scoring, service and retry passed.');
  // Real gameplay cards, both themes, no idle work, native high-DPI buffers.
  for(const saver of [false,true]) for(const [w,h] of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]) {
    const run=await setup(w,h,saver,true); await run.page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Select Snake'));
    await run.page.screenshot({path:`${artifacts}/home-${w}-${h}-${saver}.png`});
    await run.page.touchscreen.tap(...card(w,h,3)); await phase(run.page,'Ready'); await idle(run.page);
    assert.deepEqual(await run.page.evaluate(()=>[document.querySelector('canvas').width,document.querySelector('canvas').height]),[w*3,h*3]);
    const p=await pixels(run.page); assert.deepEqual(p.background,saver?[0,0,0]:[255,255,255]);
    assert.deepEqual(p.court,saver?[0,0,0]:[34,116,131]);
    await run.page.screenshot({path:`${artifacts}/game-${w}-${h}-${saver}.png`});
    await run.page.touchscreen.tap(34,34); await run.page.waitForFunction(()=>document.querySelector('canvas').getAttribute('aria-label').includes('Select Snake')); await idle(run.page);
    assert.equal(await run.page.evaluate(()=>window.__sockets),0);
    await run.context.close();
  }
  assert.deepEqual(errors,[]); await browser.close();
  console.log('Table tennis: mouse/touch, complete match, service/retry, pause/resize, idle FPS, offline, themes and six high-DPI layouts passed.');
})().catch(error=>{console.error(error);process.exit(1);});
