// Optional touch regressions. Requires Playwright and a running web build.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
(async () => {
  const browser = await chromium.launch({executablePath: process.env.JARCADE_CHROME || undefined,
    headless: true, args: ['--no-sandbox', '--enable-unsafe-swiftshader',
      '--disable-backgrounding-occluded-windows', '--disable-background-timer-throttling']});
  const ctx = await browser.newContext({viewport: {width: 390, height: 844},
    deviceScaleFactor: 3, hasTouch: true, isMobile: true});
  const p = await ctx.newPage(), errors = [];
  p.on('pageerror', e => errors.push(e.message));
  p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await p.addInitScript(() => {
    // Exercise migration from an existing save, rather than resetting inventory.
    if (!localStorage.getItem('jarcade.fih.v1')) localStorage.setItem('jarcade.fih.v1',
      `1 50 20 20 40 60 500 200 0 0 0 0 0 255 1 1 1 7 9 ${Date.now()/1000} 0`);
    window.renderedFrames = 0;
    const raf = requestAnimationFrame.bind(window);
    window.requestAnimationFrame = fn => raf(t => { renderedFrames++; fn(t); });
  });
  const label = () => p.locator('#glcanvas').getAttribute('aria-label');
  const wait = text => p.waitForFunction(text =>
    document.querySelector('#glcanvas').getAttribute('aria-label').includes(text), text);
  const tap = async (x,y) => { await p.touchscreen.tap(x,y); await p.waitForTimeout(100); };
  const save = () => p.evaluate(() => localStorage.getItem('jarcade.fih.v1').split(' ').map(Number));
  const shot = name => p.screenshot({path: `/tmp/fih-${name}.png`});
  const cd = await ctx.newCDPSession(p);
  const touch = async (type,x,y) => {
    await cd.send('Input.dispatchTouchEvent', {type, touchPoints: type === 'touchEnd' ? [] : [{x,y,id:1}]});
    await p.waitForTimeout(type === 'touchMove' ? 18 : 80);
  };
  const start = (x,y) => touch('touchStart',x,y), move = (x,y) => touch('touchMove',x,y);
  const end = () => touch('touchEnd');
  const feed = async () => {
    await start(195,770);
    for (let i=1;i<=10;i++) await move(195,770-(770-447)*i/10);
    await end();
  };
  await p.goto(process.env.JARCADE_QA_URL || 'http://127.0.0.1:8080/');
  await wait('Select Snake'); await shot('launch'); await tap(100,520); await wait('Fih. Kitchen');
  assert.equal(await p.evaluate(() => document.querySelector('#glcanvas').width), 1170);
  assert.equal((await save())[0],3); assert.equal((await save()).length,42);
  await p.waitForTimeout(1700); let n = await p.evaluate(() => renderedFrames);
  await p.waitForTimeout(200); assert(await p.evaluate(() => renderedFrames) > n);
  await start(80,330); await shot('finger-left'); await move(310,330); await shot('finger-right');
  await end();
  // Former floating wardrobe/settings locations must leave the room unchanged.
  await tap(350,144); await tap(350,34);
  assert(!(await label()).includes('Wardrobe')); assert((await label()).includes('Fih. Kitchen'));
  await tap(74,770); await wait('Pantry'); await shot('pantry');
  await tap(195,794); await wait('Food shop'); await shot('food-shop');
  let f = await save(); await tap(110,236); assert.equal((await save())[6],f[6]);
  await tap(314,236); assert.equal((await save())[6],f[6]-2); assert.equal((await save())[25],f[25]+1);
  await tap(340,748); await wait('Pantry'); await tap(195,530);
  let before = await save(); await tap(195,770); assert.equal((await save())[25],before[25]);
  await start(195,770); await move(265,427); await end(); assert.equal((await save())[25],before[25]);
  await start(195,770); await move(195,447); await shot('food-gaze'); await end();
  f = await save(); assert.equal(f[25],before[25]-1); assert(Math.abs(f[1]-70)<1); await shot('chewing');
  await feed(); await feed(); f = await save(); assert(f[1]>99);
  await feed(); assert.equal((await save())[25],f[25]); await wait('NAH'); await shot('full-refusal');
  // Wardrobe is reached through the bedroom tray.
  await tap(255,28); await wait('Fih. Bedroom'); await tap(286,770); await wait('Wardrobe');
  f = await save(); await tap(195,445); assert.equal((await save())[10],0);
  assert.equal((await save())[6],f[6]); await shot('wardrobe-preview');
  await tap(195,750); assert.equal((await save())[10],1); assert.equal((await save())[6],f[6]-30);
  await tap(195,750); assert.equal((await save())[6],f[6]-30); await tap(344,96);
  await tap(103,770); assert.equal((await save())[8],1); await shot('sleep');
  await tap(103,770); assert.equal((await save())[8],0);
  await tap(135,28); await wait('Fih. Bathroom'); f = await save();
  await tap(286,770); assert.equal((await save())[3],f[3]);
  await start(103,770); await move(195,417); await end(); await tap(286,770);
  assert.equal((await save())[3],f[3]);
  await start(103,770); await move(152,384);
  for(let i=0;i<8;i++) await move(152+i*4,384); await shot('local-soap-trail');
  for(let loop=0;loop<3;loop++) for(let i=0;i<48;i++) {
    const a=i*Math.PI*2/48; await move(195+Math.cos(a)*55,417+Math.sin(a)*55);
  }
  await shot('soap-rubbing'); await end(); await tap(286,770);
  assert(Math.abs((await save())[3]-f[3]-35)<1); await shot('shower');
  await tap(315,28); await wait('Fih. Clinic'); await tap(286,770); await wait('Potion shop');
  f = await save(); await shot('potion-shop'); await tap(314,196);
  assert.equal((await save())[6],f[6]-8); assert.equal((await save())[39],f[39]+1);
  await tap(195,748); await wait('Potion cabinet'); await shot('cabinet'); f = await save();
  await start(81,593); await move(195,447); await end();
  assert.equal((await save())[39],f[39]-1); assert((await save())[5]>f[5]+29);
  await tap(195,28); await wait('Fih. Playroom'); await p.waitForTimeout(1400);
  f = await save(); await shot('playroom-ball'); await start(288,704);
  await move(245,625); await move(180,500); await move(115,370); await end();
  assert((await save())[2]>f[2]+9); await shot('ball-thrown');
  await tap(103,770); await wait('Mini-games'); await shot('games');
  const games = ['Pearl Catch','Bubble Pop','Memory Reef','Reef Hop','Reef Dash','Shell Breaker','Pearl Slalom','Tide Beats'];
  for (let i=0;i<games.length;i++) {
    const kind=games[i]; await tap(i%2 ? 280:100,206+Math.floor(i/2)*160); await wait(`${kind}. Running`);
    await p.waitForTimeout(200); await shot(`game-${kind.replaceAll(' ','-')}`);
    await tap(350,34); await wait(`${kind}. Paused`); await p.waitForTimeout(100);
    n=await p.evaluate(()=>renderedFrames); await p.waitForTimeout(180);
    assert(await p.evaluate(()=>renderedFrames)<=n+1, `paused ${kind} redraws`);
    await tap(34,34); await wait('Mini-games');
  }
  const saved = await save(); await p.reload(); await wait('Select Snake'); await tap(100,520);
  await wait('Fih. Kitchen'); assert.equal((await save())[6],saved[6]);
  assert.deepEqual((await save()).slice(24),saved.slice(24));
  assert.deepEqual(errors,[]);
  console.log('PASS: DPR3, v3 migration/persistence, idle/finger gaze, pantry/shop, mouth feeding/NAH, bedroom wardrobe, soap strokes/rinse, sleep, potion purchase/use, ball enjoyment, eight mini-games and paused scheduling.');
  await browser.close();
})().catch(e=>{ console.error(e); process.exit(1); });
