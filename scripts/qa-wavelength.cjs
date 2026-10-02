// Same-device play, privacy, touch safety, offline behavior and display sizing.
const {chromium} = require('playwright');
const assert = require('node:assert/strict');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8091';
(async () => {
  const browser = await chromium.launch({executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args: ['--no-sandbox']});
  const errors = [];
  async function setup(width = 390, height = 844, saver = false, blocked = false) {
    const context = await browser.newContext({viewport: {width, height}, deviceScaleFactor: 3, hasTouch: true});
    await context.addInitScript(({saver, blocked}) => {
      localStorage.setItem('jarcade.settings.v1', `3 ${saver ? 1 : 0} 1 0 1`);
      window.__frames = 0; window.__sockets = 0;
      const raf = requestAnimationFrame;
      window.requestAnimationFrame = callback => raf.call(window, time => { window.__frames++; callback(time); });
      const Socket = WebSocket;
      window.WebSocket = class extends Socket { constructor(...args) { super(...args); window.__sockets++; } };
      if (blocked) { Storage.prototype.getItem = () => { throw Error('blocked'); }; Storage.prototype.setItem = () => { throw Error('blocked'); }; }
    }, {saver, blocked});
    const page = await context.newPage(); page.on('pageerror', error => errors.push(error.message));
    await page.goto(`${base}/?game=wavelength`);
    await page.waitForFunction(() => !document.getElementById('loading'));
    return {context, page};
  }
  const label = page => page.locator('canvas').getAttribute('aria-label');
  const phase = (page, value) => page.waitForFunction(value => document.querySelector('canvas').getAttribute('aria-label').includes(`. ${value}.`), value);
  const save = page => page.evaluate(() => JSON.parse(localStorage.getItem('jarcade.wavelength.v1')));
  async function click(page, x, y) { await page.mouse.click(x, y); await page.waitForTimeout(80); }
  async function next(page, value) { await page.keyboard.press('Space'); await phase(page, value); await page.waitForTimeout(50); }
  async function hidden(page) { assert(!/target \d+ percent/i.test(await label(page)), 'Concealed phase must not announce target'); }
  async function idle(page) { await page.waitForTimeout(180); const n = await page.evaluate(() => window.__frames); await page.waitForTimeout(250); assert.equal(await page.evaluate(() => window.__frames), n, 'Still screen must stop drawing even with FPS badge'); }
  const {context, page} = await setup();
  await phase(page, 'Ready'); await hidden(page); await idle(page);
  // Tab/Enter can activate a control; held Space cannot advance through phases.
  for (let i = 0; i < 5; i++) { await page.keyboard.press('Tab'); await page.waitForTimeout(35); }
  await page.keyboard.press('Enter'); await phase(page, 'Peek');
  const keyCdp = await context.newCDPSession(page);
  await keyCdp.send('Input.dispatchKeyEvent', {type: 'keyDown', key: ' ', code: 'Space', windowsVirtualKeyCode: 32, autoRepeat: true});
  await keyCdp.send('Input.dispatchKeyEvent', {type: 'keyUp', key: ' ', code: 'Space', windowsVirtualKeyCode: 32});
  await page.waitForTimeout(80); await phase(page, 'Peek');
  await page.reload(); await phase(page, 'Ready');
  // Reloading or backgrounding a private peek covers it again.
  await next(page, 'Peek'); assert(/Private target \d+ percent/.test(await label(page)));
  const target = (await save(page)).target;
  await page.reload(); await phase(page, 'Ready'); await hidden(page);
  await next(page, 'Peek'); assert.equal((await save(page)).target, target);
  await page.evaluate(() => window.dispatchEvent(new Event('blur'))); await phase(page, 'Ready'); await hidden(page);
  await next(page, 'Peek'); await next(page, 'Handoff'); await hidden(page); await idle(page);
  await next(page, 'Guess'); await hidden(page);
  // Arrow keys are processed immediately, Shift makes a larger step.
  await page.keyboard.press('ArrowRight'); await page.waitForTimeout(50);
  assert(Math.abs((await save(page)).guess - .515) < .00001);
  await page.keyboard.press('Shift+ArrowRight'); await page.waitForTimeout(50);
  assert(Math.abs((await save(page)).guess - .565) < .00001);
  await idle(page);
  // Actual high-DPI finger drag, then drag across the confirmation button.
  const cdp = await context.newCDPSession(page);
  const touch = (type, points) => cdp.send('Input.dispatchTouchEvent', {type, touchPoints: points.map(([id, x, y]) => ({id, x, y}))});
  await touch('touchStart', [[1, 120, 430]]); await page.waitForTimeout(50);
  await touch('touchMove', [[1, 280, 420]]); await page.waitForTimeout(80);
  assert(/Guess (7\d|8\d) percent/.test(await label(page)), await label(page));
  await touch('touchMove', [[1, 195, 800]]); await touch('touchEnd', []); await phase(page, 'Guess'); await hidden(page);
  // Multiple fingers and cancellation cannot confirm or continue a dial gesture.
  await touch('touchStart', [[1, 120, 420]]); await page.waitForTimeout(50);
  await touch('touchMove', [[1, 120, 420], [2, 195, 800]]); await page.waitForTimeout(50);
  await touch('touchMove', [[2, 195, 800]]); await touch('touchEnd', []); await page.waitForTimeout(60); await phase(page, 'Guess');
  await touch('touchStart', [[1, 150, 420]]); await touch('touchCancel', []); await page.waitForTimeout(80); await phase(page, 'Guess'); await idle(page);
  // Reorientation cancels a held pointer without turning it into a button tap.
  await touch('touchStart', [[1, 150, 420]]); await page.waitForTimeout(50);
  await page.setViewportSize({width: 568, height: 320}); await page.waitForTimeout(50);
  await touch('touchEnd', []); await page.setViewportSize({width: 390, height: 844}); await phase(page, 'Guess'); await idle(page);
  // A restored guess requires the handoff, but keeps the chosen needle position.
  const guess = (await save(page)).guess;
  await page.reload(); await phase(page, 'Handoff'); await hidden(page);
  await next(page, 'Guess'); assert.equal((await save(page)).guess, guess);
  // No sockets or server requests are needed to finish and play another round.
  await context.setOffline(true);
  await next(page, 'Result'); assert(/Target \d+ percent; guess \d+ percent/.test(await label(page)));
  await next(page, 'Ready'); assert.equal((await save(page)).round, 2);
  await next(page, 'Peek'); await next(page, 'Handoff'); await next(page, 'Guess'); await next(page, 'Result');
  assert.equal(await page.evaluate(() => window.__sockets), 0);
  await context.setOffline(false);
  // Preset packs and custom text use the same phone keyboard adapter.
  await click(page, 290, 30); assert((await label(page)).includes('Prompt decks'));
  await click(page, 195, 296); await phase(page, 'Ready'); assert.equal((await save(page)).deck, 'Portuguese');
  await click(page, 290, 30); await click(page, 195, 372);
  await click(page, 195, 185); await page.getByRole('textbox', {name: 'First extreme'}).fill('Frio de inverno'); await page.getByRole('textbox', {name: 'First extreme'}).press('Enter');
  await click(page, 195, 265); await page.getByRole('textbox', {name: 'Second extreme'}).fill('Calor de verão'); await page.getByRole('textbox', {name: 'Second extreme'}).press('Enter');
  await click(page, 195, 800); await phase(page, 'Ready'); assert((await label(page)).includes('Frio de inverno to Calor de verão'));
  await page.reload(); await phase(page, 'Ready'); assert.equal((await save(page)).deck, 'Custom'); await idle(page);
  // Home preview is a working entry point rather than an online room launcher.
  await page.keyboard.press('Escape'); await page.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').includes('Select Coupe, Dicksit, or Wavelength'));
  await page.screenshot({path: '/tmp/jarcade-wave-home.png'});
  await click(page, 100, 565); await phase(page, 'Ready');
  await context.close();
  // All phases and menus at tiny/phone/tablet/landscape/desktop sizes, both themes.
  for (const saver of [false, true]) for (const [width, height] of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]) {
    const {context, page} = await setup(width, height, saver);
    for (const state of ['Ready','Peek','Handoff','Guess','Result']) {
      await phase(page, state); await page.waitForTimeout(60);
      await page.screenshot({path: `/tmp/jarcade-wave-${width}x${height}-${saver ? 'black' : 'white'}-${state}.png`});
      if (!['Peek','Result'].includes(state)) await hidden(page);
      if (state !== 'Result') await page.keyboard.press('Space');
    }
    await idle(page);
    assert.deepEqual(await page.evaluate(() => [document.querySelector('canvas').width, document.querySelector('canvas').height]), [width * 3, height * 3]);
    assert.equal(await page.evaluate(() => getComputedStyle(document.body).backgroundColor), saver ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)');
    const hw = Math.min(width - 32, 1060), hx = (width - hw) / 2;
    await click(page, hx + hw - 72, 30); assert((await label(page)).includes('Prompt decks')); await page.screenshot({path: `/tmp/jarcade-wave-decks-${width}x${height}.png`}); await page.keyboard.press('Escape');
    await click(page, hx + hw - 22, 30); assert((await label(page)).includes('How to play')); await page.screenshot({path: `/tmp/jarcade-wave-help-${width}x${height}.png`}); await page.keyboard.press('Escape');
    assert.equal(await page.evaluate(() => window.__sockets), 0); await context.close();
  }
  const blocked = await setup(390, 844, false, true);
  for (const state of ['Peek','Handoff','Guess','Result']) await next(blocked.page, state);
  await idle(blocked.page); await blocked.context.close();
  assert.deepEqual(errors, []); await browser.close();
  console.log('Wavelength: private handoff/reload/focus loss, retina touch + keyboard, safe drags/cancellation, offline play, no sockets, decks/custom saves, blocked storage, idle FPS, both themes and six layouts passed.');
})().catch(error => {console.error(error); process.exit(1);});
