// Launch gallery regression checks with real mouse, touch, and keyboard input.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const artifacts = '/tmp/jarcade-home-qa';
mkdirSync(artifacts, { recursive: true });
function gallery(w, h) {
  const margin = w < 360 ? 16 : w < 600 ? 20 : 40;
  const width = Math.min(w - margin * 2, 1040), x = (w - width) / 2;
  const landscape = (w > h * 1.3 && h < 620) || (h < 360 && w >= 360);
  const columns = width >= 960 ? 4 : width >= 600 || (landscape && width >= 480) ? 3 : 2;
  const gap = columns === 2 ? 12 : 20, cardWidth = (width - gap * (columns - 1)) / columns;
  const top = h < 500 ? 132 : h < 640 ? 208 : 238, rows = Math.ceil(5 / columns);
  const imageHeight = Math.min(cardWidth * .88, Math.max(88, (h - top - 24 - gap * (rows - 1)) / rows - 60));
  const cardHeight = imageHeight + 60, contentHeight = rows * cardHeight + (rows - 1) * gap;
  const maximum = Math.max(0, contentHeight - (h - top - 16));
  return { x, width, top, maximum,
    card: (index, offset = 0) => [x + index % columns * (cardWidth + gap) + cardWidth / 2,
      top + Math.floor(index / columns) * (cardHeight + gap) + cardHeight / 2 - offset] };
}
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome',
    args: ['--no-sandbox', '--enable-unsafe-swiftshader'] });
  try {
    const errors = [];
    for (const saver of [false, true]) for (const [w, h] of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]) {
      console.log(`Gallery ${w}x${h}, ${saver ? 'black' : 'white'}`);
      const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 3, hasTouch: true });
      await context.addInitScript(saver => {
        localStorage.setItem('jarcade.settings.v1', `3 ${saver ? 1 : 0} 1 0 1`);
        const raf = requestAnimationFrame; window.__frames = 0; window.__sockets = 0;
        window.requestAnimationFrame = cb => raf.call(window, t => { window.__frames++; cb(t); });
        const Socket = WebSocket; window.WebSocket = class extends Socket { constructor(...args) { super(...args); window.__sockets++; } };
      }, saver);
      const p = await context.newPage(); p.on('pageerror', e => errors.push(e.message));
      const waitHome = () => p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label')?.includes('Select Snake'));
      const idle = async () => { await p.waitForTimeout(750); const n = await p.evaluate(() => window.__frames);
        await p.waitForTimeout(250); assert.equal(await p.evaluate(() => window.__frames), n, 'Static gallery/game must sleep with FPS enabled'); };
      await p.goto(base); await p.waitForFunction(() => !document.getElementById('loading')); await waitHome(); await idle();
      assert.deepEqual(await p.evaluate(() => [document.querySelector('canvas').width, document.querySelector('canvas').height]), [w*3,h*3]);
      assert.equal(await p.evaluate(() => getComputedStyle(document.body).backgroundColor), saver ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)');
      const g = gallery(w, h);
      // A swipe starts on a game card, scrolls, and never enters that game.
      const cdp = await context.newCDPSession(p);
      const touch = (type, points) => cdp.send('Input.dispatchTouchEvent', { type, touchPoints: points });
      await touch('touchStart', [{id:1,x:g.x+45,y:g.top+70}]); await p.waitForTimeout(35);
      await touch('touchMove', [{id:1,x:g.x+45,y:g.top+12}]); await p.waitForTimeout(35);
      await touch('touchEnd', []); await p.waitForTimeout(60); await waitHome(); await idle();
      // Scroll to the bottom and launch the fifth solo game by touch.
      for (let i=0;i<5;i++) { await p.keyboard.press('PageDown'); await p.waitForTimeout(40); }
      await p.screenshot({ path: `${artifacts}/solo-${w}x${h}-${saver}.png`, scale:'css' });
      await p.touchscreen.tap(...g.card(4,g.maximum));
      await p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').startsWith('Jarcade. Nonograms.'));
      await idle(); assert.equal(await p.evaluate(() => window.__sockets), 0);
      await p.keyboard.press('Escape'); await waitHome();
      // Switching category restores the top; the new multiplayer card remains reachable.
      await p.touchscreen.tap(g.x+g.width*.75,g.top-34);
      await p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').includes('Select Coupe'));
      for (let i=0;i<5;i++) { await p.keyboard.press('PageDown'); await p.waitForTimeout(40); }
      await p.screenshot({ path: `${artifacts}/multiplayer-${w}x${h}-${saver}.png`, scale:'css' });
      await p.touchscreen.tap(...g.card(4,g.maximum));
      await p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').startsWith('Jarcade. Codenames.'));
      await idle(); assert.equal(await p.evaluate(() => window.__sockets), 0, 'Opening a room menu alone should not connect');
      if (w === 320 && !saver) {
        await p.goto(base); await p.waitForFunction(() => !document.getElementById('loading')); await waitHome();
        for(let i=0;i<9;i++) { await p.keyboard.press('Tab'); await p.waitForTimeout(50); }
        await p.screenshot({path:`${artifacts}/keyboard-last-card.png`,scale:'css'});
        await p.keyboard.press('Enter');
        await p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').startsWith('Jarcade. Nonograms.'));
      }
      await context.close();
    }
    assert.deepEqual(errors, []);
    console.log('Home: fifth solo/online cards, safe touch scrolling, keyboard visibility, category switching, idle FPS, native DPR 3 and twelve layouts passed.');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
