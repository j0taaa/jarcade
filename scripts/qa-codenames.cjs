// Isolated real browser UI + independent private WebSocket seats.
// Run against a disposable server with JARCADE_QA_URL; artifacts stay in /tmp.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const artifacts = '/tmp/jarcade-codenames-screens';
mkdirSync(artifacts, { recursive: true });
let browser, activePage;
const clients = [];
const pause = ms => new Promise(r => setTimeout(r, ms));
async function until(fn, label, timeout = 10000) {
  const start = Date.now(); while (!fn()) { assert(Date.now() - start < timeout, `Timed out: ${label}`); await pause(20); }
}
async function seat(message) {
  const socket = new WebSocket(base.replace(/^http/, 'ws') + '/ws');
  const client = { socket, room: null, session: null, errors: [] }; clients.push(client);
  socket.addEventListener('message', e => { const d = JSON.parse(e.data); if (d.type === 'state') client.room = d.room; if (d.type === 'welcome') client.session = d.session; if (d.type === 'error') client.errors.push(d.message); });
  await until(() => socket.readyState === WebSocket.OPEN, 'socket open'); socket.send(JSON.stringify(message));
  await until(() => client.room || client.errors.length, 'room handshake'); assert.deepEqual(client.errors, []); return client;
}
async function command(client, move, kind = 'codenames') {
  const revision = client.room.revision;
  client.socket.send(JSON.stringify({ type: 'play', revision: client.room.epoch, command: { kind, move } }));
  await until(() => client.room.revision > revision || client.errors.length, `${kind} ${JSON.stringify(move)}`); assert.deepEqual(client.errors, []); await pause(80);
}
function geometry(width, height, canClue = false) {
  const w = Math.min(width - 32, 1100), x = (width - w) / 2;
  const boundsH = height - 82, compact = boundsH < 430, sidecar = compact && w >= 480;
  const bw = Math.min(sidecar ? w - 206 : w, 840), bx = sidecar ? x : (width - bw) / 2;
  const infoX = sidecar ? bx + bw + 16 : bx, infoW = sidecar ? w - bw - 16 : bw;
  const topH = compact ? 108 : 124, footerH = canClue && (!compact || sidecar) ? 112 : 60;
  const viewportY = sidecar ? 66 : 66 + topH, viewportH = sidecar ? boundsH - 8 : Math.max(36, boundsH - topH - footerH - 14);
  const gap = bw < 400 ? 5 : 9, cellW = (bw - gap * 4) / 5;
  const cellH = sidecar ? Math.max(44, Math.min(103, (viewportH - gap * 4) / 5)) : Math.max(52, Math.min(103, cellW * .9 + 10));
  const boardH = cellH * 5 + gap * 4, boardY = boardH < viewportH ? viewportY + (viewportH - boardH) * .35 : viewportY;
  return { bx, bw, infoX, infoW, gap, cellW, cellH, boardY, footerY: height - 16 - footerH, center(i) { return [bx + i % 5 * (cellW + gap) + cellW / 2, boardY + Math.floor(i / 5) * (cellH + gap) + cellH / 2]; } };
}
(async () => {
  browser = await chromium.launch({ executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args: ['--no-sandbox', '--enable-unsafe-swiftshader'] });
  const errors = [];
  async function context(width = 390, height = 844, saver = false, resume = null) {
    const c = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 3, hasTouch: true });
    await c.addInitScript(({ saver, resume }) => {
      localStorage.setItem('jarcade.settings.v1', `3 ${saver ? 1 : 0} 1 0 1`);
      if (resume) localStorage.setItem('jarcade.online.v1', JSON.stringify({ name: 'Guest', sessions: [resume] }));
      const original = requestAnimationFrame; window.__frames = 0;
      window.requestAnimationFrame = cb => original.call(window, t => { window.__frames++; cb(t); });
      const Native = WebSocket; window.WebSocket = class extends Native {
        constructor(...a) { super(...a); window.__socket = this; this.addEventListener('message', e => { const d = JSON.parse(e.data); if (d.type === 'state') window.__room = d.room; if (d.type === 'welcome') window.__session = d.session; if (d.type === 'error') (window.__serverErrors ||= []).push(d.message); }); }
        send(text) { const d = JSON.parse(text); if (resume && d.type === 'create') text = JSON.stringify({ type: 'resume', room: resume.room, token: resume.token }); super.send(text); }
      };
    }, { saver, resume });
    const p = await c.newPage(); activePage = p; p.on('pageerror', e => errors.push(e.message));
    await p.goto(`${base}/?game=codenames`); await p.waitForFunction(() => !document.getElementById('loading')); return { c, p };
  }
  const click = async (p, x, y) => { await p.mouse.click(x, y); await p.waitForTimeout(120); };
  async function connectResume(p, width, height) {
    const w = Math.min(width - 32, 1100), wide = w >= 800, compact = height < 650;
    const targetY = 66 + (wide ? 64 : compact ? 146 : 248) + 130;
    const offset = Math.ceil(Math.max(0, targetY - height + 100) / 80) * 80;
    for (let n = 0; n < offset / 80; n++) { await p.keyboard.press('ArrowDown'); await p.waitForTimeout(70); }
    const formWidth = wide ? 400 : Math.min(w, 440), formX = wide ? (width - w) / 2 + w * .54 : (width - formWidth) / 2;
    await click(p, formX + formWidth / 2, targetY - offset); await p.waitForFunction(() => !!window.__room?.codenames);
  }
  const host = await context(); const p = host.p;
  await click(p, 180, 379); await p.locator('#jarcade-text-editor').fill('Mila'); await p.locator('#jarcade-text-editor').press('Enter'); await click(p, 180, 444); await p.waitForFunction(() => !!window.__room);
  const code = await p.evaluate(() => window.__room.code), bots = [];
  for (let i = 1; i < 4; i++) bots.push(await seat({ type: 'join', room: code, name: ['Rio', 'Nora', 'Ari'][i - 1] }));
  await p.waitForFunction(() => window.__room.members.length === 4);
  await p.screenshot({ path: `${artifacts}/lobby-phone.png`, scale: 'css' });
  // Host word-deck toggle uses actual UI and invalidates previous readiness.
  await click(p, 285, 339); await p.waitForFunction(() => window.__room.codenames_setup.language === 'portuguese');
  await click(p, 100, 339); await p.waitForFunction(() => window.__room.codenames_setup.language === 'english');
  for (const bot of bots) await command(bot, true, 'ready');
  await click(p, 180, 650); await p.waitForFunction(() => !!window.__room.codenames);
  // If blue starts, make one legal neutral guess to hand the clue to the browser's red spymaster.
  if ((await p.evaluate(() => window.__room.codenames.team)) === 'blue') {
    await command(bots[0], { type: 'clue', word: 'quintessential', number: 1 });
    const neutral = bots[0].room.codenames.cards.findIndex(c => c.identity === 'neutral');
    await command(bots[2], { type: 'guess', card: neutral });
  }
  await p.waitForFunction(() => window.__room.codenames.can_clue);
  const spySession = await p.evaluate(() => window.__session);
  assert((await p.evaluate(() => window.__room.codenames.cards)).every(c => c.identity));
  for (const bot of [bots[1], bots[2]]) assert(bot.room.codenames.cards.every(c => c.revealed || c.identity === null));
  let frames = await p.evaluate(() => window.__frames); await p.waitForTimeout(400); assert.equal(await p.evaluate(() => window.__frames), frames, 'Waiting for a clue must be event-driven, even with FPS');
  const spyGeo = geometry(390, 844, true);
  await click(p, 130, spyGeo.footerY + 22); await p.locator('#jarcade-text-editor').fill('quintessential');
  assert.equal(await p.locator('#jarcade-text-editor').getAttribute('aria-label'), 'One-word clue');
  await p.setViewportSize({ width: 568, height: 320 }); await p.waitForTimeout(180);
  const rotatedEditor = await p.locator('#jarcade-text-editor').boundingBox();
  assert(rotatedEditor.x >= 0 && rotatedEditor.y >= 0 && rotatedEditor.x + rotatedEditor.width <= 568 && rotatedEditor.y + rotatedEditor.height <= 320, 'Open clue editor must follow phone rotation');
  await p.setViewportSize({ width: 390, height: 844 }); await p.waitForTimeout(180);
  await p.locator('#jarcade-text-editor').press('Enter'); await click(p, 180, spyGeo.footerY + 76);
  await p.waitForFunction(() => window.__room.codenames.phase === 'guess');
  assert.equal((await p.evaluate(() => window.__room.codenames.clue)).word, 'quintessential');
  await p.screenshot({ path: `${artifacts}/spymaster-phone.png`, scale: 'css' });
  // Resume the red operative in another isolated browser and use the real confirmation flow.
  const operativeSession = bots[1].session; bots[1].socket.close(); await pause(120);
  const operative = await context(390, 844, false, operativeSession); const op = operative.p; await connectResume(op, 390, 844);
  assert((await op.evaluate(() => window.__room.codenames.cards)).every(c => c.revealed || c.identity === null));
  const own = await p.evaluate(() => window.__room.codenames.cards.findIndex(c => !c.revealed && c.identity === 'red'));
  const geo = geometry(390, 844), center = geo.center(own);
  await op.mouse.move(...center); await op.mouse.down(); await op.mouse.move(center[0], center[1] - 35, { steps: 8 }); await op.mouse.up();
  assert(!(await op.locator('canvas').getAttribute('aria-label')).includes('selected. Confirm'), 'Dragging must not select a card');
  await click(op, ...center); assert((await op.locator('canvas').getAttribute('aria-label')).includes(`Card ${own + 1} selected`));
  await click(op, 285, geo.footerY + 22); await op.waitForFunction(i => window.__room.codenames.cards[i].revealed, own);
  assert.equal((await op.evaluate(() => window.__room.codenames.cards.filter(c => c.identity !== null))).length, 1 + (bots[0].room.codenames.cards.filter(c => c.revealed && c.identity === 'neutral').length));
  await op.screenshot({ path: `${artifacts}/operative-phone.png`, scale: 'css' });
  // On a short landscape phone the board stays visible next to a compact confirmation rail.
  await op.setViewportSize({ width: 568, height: 320 }); await op.waitForTimeout(150);
  const secondOwn = await p.evaluate(() => window.__room.codenames.cards.findIndex(c => !c.revealed && c.identity === 'red'));
  const shortGeo = geometry(568, 320);
  await click(op, ...shortGeo.center(secondOwn)); assert((await op.locator('canvas').getAttribute('aria-label')).includes(`Card ${secondOwn + 1} selected`));
  await click(op, shortGeo.infoX + shortGeo.infoW * .75, shortGeo.footerY + 22);
  await op.waitForFunction(i => window.__room.codenames.cards[i].revealed, secondOwn);
  await op.screenshot({ path: `${artifacts}/operative-landscape.png`, scale: 'css' });
  await op.setViewportSize({ width: 390, height: 844 }); await op.waitForTimeout(150);

  // Refresh + reconnect preserves role, revealed cards, and the private projection.
  await op.reload(); await op.waitForFunction(() => !document.getElementById('loading')); await connectResume(op, 390, 844);
  assert.equal((await op.evaluate(() => window.__room.codenames.seats[window.__room.you].role)), 'operative');
  assert((await op.evaluate(() => window.__room.codenames.cards)).every(c => c.revealed || c.identity === null));
  // Pass via actual UI; then finish by deliberately finding the assassin on blue's turn.
  await click(op, 180, geo.footerY + 22); await op.waitForFunction(() => window.__room.codenames.phase === 'clue' && window.__room.codenames.team === 'blue');
  await command(bots[0], { type: 'clue', word: 'quintessential', number: null });
  const assassin = bots[0].room.codenames.cards.findIndex(c => c.identity === 'assassin');
  await command(bots[2], { type: 'guess', card: assassin });
  await op.waitForFunction(() => window.__room.codenames.phase === 'finished');
  assert.equal(await op.evaluate(() => window.__room.codenames.winner), 'red');
  assert((await op.evaluate(() => window.__room.codenames.cards)).every(c => c.identity));
  frames = await op.evaluate(() => window.__frames); await op.waitForTimeout(400); assert.equal(await op.evaluate(() => window.__frames), frames);
  await op.screenshot({ path: `${artifacts}/finished-phone.png`, scale: 'css' });
  await operative.c.close();
  // Rematch preserves teams, then every layout resumes the private spymaster key.
  await p.waitForFunction(() => window.__room.codenames.phase === 'finished');
  await click(p, 180, geo.footerY + 22); await p.waitForFunction(() => !window.__room.codenames);
  const replacement = await seat({ type: 'resume', room: operativeSession.room, token: operativeSession.token }); bots[1] = replacement;
  for (const bot of bots) await command(bot, true, 'ready');
  await click(p, 180, 596); await p.waitForFunction(() => window.__room.members[0].ready);
  await click(p, 180, 650); await p.waitForFunction(() => !!window.__room.codenames);
  if ((await p.evaluate(() => window.__room.codenames.team)) === 'blue') {
    await command(bots[0], { type: 'clue', word: 'quintessential', number: 1 });
    await command(bots[2], { type: 'guess', card: bots[0].room.codenames.cards.findIndex(c => c.identity === 'neutral') });
  }
  await p.waitForFunction(() => window.__room.codenames.can_clue);
  await host.c.close();
  for (const saver of [false, true]) for (const [width, height] of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]) {
    const { c, p } = await context(width, height, saver, spySession); await connectResume(p, width, height);
    const data = await p.evaluate(() => ({ buffer: [document.querySelector('canvas').width, document.querySelector('canvas').height], background: getComputedStyle(document.body).backgroundColor, cards: window.__room.codenames.cards }));
    assert.deepEqual(data.buffer, [width * 3, height * 3]); assert.equal(data.background, saver ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)'); assert.equal(data.cards.length, 25); assert(data.cards.every(c => c.identity));
    assert((await p.locator('canvas').getAttribute('aria-label')).includes('Spymaster'));
    const layoutGeo = geometry(width, height, true);
    await click(p, layoutGeo.infoX + 25, layoutGeo.footerY + 22);
    const editor = p.locator('#jarcade-text-editor'); await editor.fill('quintessential');
    const rect = await editor.boundingBox(); assert(rect.x >= 0 && rect.y >= 0 && rect.x + rect.width <= width && rect.y + rect.height <= height, 'Clue editor must fit the visible viewport');
    await editor.press('Enter');
    await p.screenshot({ path: `${artifacts}/board-${width}x${height}-${saver ? 'black' : 'white'}.png`, scale: 'css' });
    await p.keyboard.press('Tab'); await p.keyboard.press('Tab'); await c.close();
  }
  assert.deepEqual(errors, []);
  for (const client of clients) if (client.socket.readyState === WebSocket.OPEN) { client.socket.send(JSON.stringify({type:'leave'})); client.socket.close(); }
  await browser.close();
  console.log('Codenames: team/deck lobby, private keys, real clue/guess/pass/rematch UI, drag safety, assassin win, reconnect, event-driven FPS, DPR 3 and twelve layouts passed.');
})().catch(async e => { console.error(e); if (activePage && !activePage.isClosed()) { await activePage.screenshot({path: `${artifacts}/failure.png`, scale: 'css'}).catch(() => {}); console.error(await activePage.locator('canvas').getAttribute('aria-label').catch(() => '')); } for (const client of clients) client.socket.close(); await browser?.close().catch(() => {}); process.exit(1); });
