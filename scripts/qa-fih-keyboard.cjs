// Optional regression for keyboard care and cancellation.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
(async () => {
  const browser = await chromium.launch({executablePath: process.env.JARCADE_CHROME || undefined,
    headless: true, args: ['--no-sandbox', '--enable-unsafe-swiftshader']});
  const page = await browser.newPage({viewport: {width: 390, height: 844}});
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.addInitScript(() => localStorage.setItem('jarcade.fih.v1',
    `1 50 20 20 40 60 500 200 0 0 0 0 0 255 1 1 1 7 9 ${Date.now()/1000} 0`));
  await page.goto(process.env.JARCADE_QA_URL || 'http://127.0.0.1:8080/');
  await page.waitForFunction(() => document.querySelector('#glcanvas').getAttribute('aria-label').includes('Select Snake'));
  await page.mouse.click(100,520); await page.waitForTimeout(100);
  // Focus order: back, five stats, three navigation controls, pantry, food.
  for (let i=0;i<11;i++) { await page.keyboard.press('Tab'); await page.waitForTimeout(20); }
  await page.keyboard.press('Enter'); await page.waitForTimeout(100);
  for (let i=0;i<23;i++) { await page.keyboard.press('ArrowUp'); await page.waitForTimeout(20); }
  await page.screenshot({path:'/tmp/fih-keyboard-mouth.png'});
  await page.keyboard.press('Enter'); await page.waitForTimeout(150);
  const saved = await page.evaluate(() => localStorage.getItem('jarcade.fih.v1').split(' ').map(Number));
  assert(saved[1]>67 && saved[1]<69, 'Enter did not drop food into the mouth');
  await page.keyboard.press('Enter'); await page.waitForTimeout(70);
  await page.keyboard.press('Escape'); await page.waitForTimeout(80);
  assert((await page.locator('#glcanvas').getAttribute('aria-label')).includes('Fih. Kitchen'),
    'Escape left Fih instead of canceling the carried item');
  assert.deepEqual(errors,[]);
  console.log('PASS: keyboard pickup, arrow movement, mouth drop, and Escape cancellation.');
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });
