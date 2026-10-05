// Real browser controls plus independent private WebSocket seats.
// Run against a disposable room service; screenshots go outside the repository.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const artifacts = '/tmp/jarcade-wolves-screens';
mkdirSync(artifacts, { recursive: true });
let activePage, qaBrowser;
const pause = ms => new Promise(r => setTimeout(r, ms));
async function until(fn, label, timeout = 8000) {
  const start = Date.now(); while (!fn()) { assert(Date.now() - start < timeout, `Timed out: ${label}`); await pause(20); }
}
const wolves = role => ['werewolf', 'wolf_seer', 'alpha_wolf', 'junior_wolf'].includes(role);
async function seat(message) {
  const socket = new WebSocket(base.replace(/^http/, 'ws') + '/ws');
  const client = { socket, room: null, session: null, errors: [] };
  socket.addEventListener('message', e => { const d = JSON.parse(e.data); if (d.type === 'state') client.room = d.room; if (d.type === 'welcome') client.session = d.session; if (d.type === 'error') client.errors.push(d.message); });
  await until(() => socket.readyState === WebSocket.OPEN, 'socket open'); socket.send(JSON.stringify(message));
  await until(() => client.room || client.errors.length, 'seat handshake'); assert.deepEqual(client.errors, []); return client;
}
async function command(client, move, kind = 'wolves') {
  const revision = client.room.revision;
  client.socket.send(JSON.stringify({ type: 'play', revision: client.room.epoch, command: { kind, move } }));
  await until(() => client.room.revision > revision || client.errors.length, `${kind} ${JSON.stringify(move)}`); assert.deepEqual(client.errors, []);
  await pause(75);
}
(async () => {
  const browser = await chromium.launch({ executablePath: process.env.JARCADE_CHROME || '/usr/bin/google-chrome', args: ['--no-sandbox', '--enable-unsafe-swiftshader'] });
  qaBrowser = browser;
  const errors = [];
  async function context(width = 390, height = 844, saver = false, resume = null) {
    const c = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 3, hasTouch: true });
    await c.addInitScript(({ saver, resume }) => {
      localStorage.setItem('jarcade.settings.v1', `3 ${saver ? 1 : 0} 1 0 1`);
      if (resume) localStorage.setItem('jarcade.online.v1', JSON.stringify({ name: 'Guest', sessions: [resume] }));
      const original = requestAnimationFrame; window.__frames = 0;
      window.requestAnimationFrame = cb => original.call(window, t => { window.__frames++; cb(t); });
      const Native = WebSocket; window.WebSocket = class extends Native {
        constructor(...a) { super(...a); window.__socket = this; this.addEventListener('message', e => { const d = JSON.parse(e.data); if (d.type === 'state') window.__room = d.room; if (d.type === 'welcome') window.__session = d.session; }); }
        send(text) { const d = JSON.parse(text); if (resume && d.type === 'create') text = JSON.stringify({ type: 'resume', room: resume.room, token: resume.token }); super.send(text); }
      };
    }, { saver, resume });
    const p = await c.newPage(); activePage = p; p.on('pageerror', e => errors.push(e.message));
    await p.goto(`${base}/?game=wolves`); await p.waitForFunction(() => !document.getElementById('loading')); return { c, p };
  }
  const { c, p } = await context();
  const click = async (x, y) => { await p.mouse.click(x, y); await p.waitForTimeout(110); };
  const room = () => p.evaluate(() => window.__room);
  const phase = value => p.waitForFunction(value => window.__room?.wolves?.phase === value, value);
  await click(180, 379); await p.locator('#jarcade-text-editor').fill('Mila'); await p.locator('#jarcade-text-editor').press('Enter'); await click(180, 444); await p.waitForFunction(() => !!window.__room);
  const code = (await room()).code, bots = [];
  for (let i = 1; i < 6; i++) bots.push(await seat({ type: 'join', room: code, name: ['Rio', 'Nora', 'Ari', 'Sol', 'Eli'][i - 1] }));
  await p.waitForFunction(() => window.__room.members.length === 6);
  // Real host setup editor: inspect advanced roles, then replace a villager with a Doctor.
  await click(180, 222); await p.waitForFunction(() => document.querySelector('canvas').getAttribute('aria-label').includes('Role setup'));
  await p.screenshot({ path: `${artifacts}/advanced-setup.png`, scale: 'css' });
  await click(312, 128); await click(246, 221); await click(246, 221); await click(340, 453); await click(340, 569);
  await p.screenshot({ path: `${artifacts}/custom-setup.png`, scale: 'css' });
  await click(245, 800); await p.waitForFunction(() => window.__room.wolves_setup.preset === 'custom');
  assert.equal((await room()).wolves_setup.roles.length, 6); assert((await room()).wolves_setup.roles.includes('doctor'));
  assert((await room()).members.slice(1).every(m => !m.ready));
  for (const bot of bots) await command(bot, true, 'ready');
  await click(180, 644); await phase('night');
  await p.screenshot({ path: `${artifacts}/night-phone.png`, scale: 'css' });
  let r = await room(); const hostRole = r.wolves.role;
  assert.equal(r.wolves.players.filter(v => v.role !== null).length, wolves(hostRole) ? 2 : 1);
  await p.waitForTimeout(350); let frames = await p.evaluate(() => window.__frames);
  await p.waitForTimeout(2200); assert((await p.evaluate(() => window.__frames)) - frames <= 4, 'Countdown should render at 1 Hz, not continuously');
  // Drags and multi-touch never select a portrait.
  await p.mouse.move(60, 310); await p.mouse.down(); await p.mouse.move(65, 260, { steps: 8 }); await p.mouse.up();
  assert(!(await p.locator('canvas').getAttribute('aria-label')).includes('Player selected'));
  await p.mouse.move(180, 450); await p.mouse.wheel(0, -2000); await p.waitForTimeout(100);
  // Select a legal target in the visible grid using actual UI input.
  r = await room(); const target = (wolves(hostRole) ? r.wolves.victims : r.wolves.targets)[0];
  if (target !== undefined) {
    if (wolves(hostRole)) { const modes = 2 + (r.wolves.can_mark ? 1 : 0); await click(16 + ((358 - 8 * (modes - 1)) / modes + 8) * 1.5 - 4, 251); }
    await click(16 + (target % 4) * 91.5 + 40, 315 + Math.floor(target / 4) * 104);
    assert((await p.locator('canvas').getAttribute('aria-label')).includes('Player selected'));
  }
  await click(180, 800); await p.waitForFunction(() => window.__room.wolves.locked);
  // Private pack messages cannot appear in village projections.
  const wolfBot = bots.find(b => wolves(b.room.wolves.role));
  await command(wolfBot, { type: 'chat', channel: 'pack', text: 'A secret hunt' });
  for (const b of bots) assert.equal(b.room.wolves.chat.some(m => m.text === 'A secret hunt'), wolves(b.room.wolves.role));
  for (const bot of bots) await command(bot, { type: 'night', target: null, kill: null });
  await phase('dawn'); for (const bot of bots) if (bot.room.wolves.players[bot.room.you].alive) await command(bot, { type: 'ready' }); await click(180, 800); await phase('discussion');
  // Native phone text editor for the village channel; the UI sends the message.
  await click(195, 196); await click(120, 800); await p.locator('#jarcade-text-editor').fill('Who looks suspicious?');
  assert.equal(await p.locator('#jarcade-text-editor').getAttribute('aria-label'), 'Message');
  await p.locator('#jarcade-text-editor').press('Enter'); await click(340, 800);
  await p.waitForFunction(() => window.__room.wolves.chat.some(m => m.text === 'Who looks suspicious?'));
  await p.screenshot({ path: `${artifacts}/village-chat.png`, scale: 'css' });
  // Reconnect after a refresh preserves the private role and match phase.
  await p.reload(); await p.waitForFunction(() => !document.getElementById('loading')); await click(180, 645); await phase('discussion');
  assert.equal((await room()).wolves.role, hostRole);
  await click(180, 800); for (const bot of bots) if (bot.room.wolves.players[bot.room.you].alive) await command(bot, { type: 'ready' }); await phase('vote');
  assert.equal((await room()).wolves.ballots, null);
  const suspect = (await room()).wolves.players.findIndex((_, i) => i === 0 ? wolves(hostRole) : wolves(bots[i - 1].room.wolves.role));
  if (suspect !== 0 && (await room()).wolves.players[0].alive) await click(16 + (suspect % 4) * 91.5 + 40, 315 + Math.floor(suspect / 4) * 104);
  await click(180, 800);
  for (const bot of bots) if (bot.room.wolves.players[bot.room.you].alive) await command(bot, { type: 'vote', target: bot.room.you === suspect ? null : suspect });
  await p.waitForFunction(() => ['night', 'finished'].includes(window.__room.wolves.phase));
  assert((await room()).wolves.ballots !== null);
  // Finish the real match with legal actions from every seat.

  async function hostCommand(move) { await p.evaluate(move => window.__socket.send(JSON.stringify({ type: 'play', revision: window.__room.epoch, command: { kind: 'wolves', move } })), move); await pause(110); }
  for (let turns = 0; (await room()).wolves.phase !== 'finished'; turns++) {
    assert(turns < 200); const views = [await room(), ...bots.map(b => b.room)];
    const g = views[0].wolves; const wolfSeat = views.findIndex(v => v.wolves.players[v.you].alive && wolves(v.wolves.role));
    for (let i = 0; i < 6; i++) {
      const v = (i === 0 ? await room() : bots[i - 1].room).wolves;
      if (!v.players[i].alive || v.locked || v.phase !== g.phase) continue;
      const move = g.phase === 'night' ? { type: 'night', target: null, kill: null } : g.phase === 'vote' ? { type: 'vote', target: wolfSeat < 0 || wolfSeat === i ? null : wolfSeat } : { type: 'ready' };
      if (i === 0) await hostCommand(move); else await command(bots[i - 1], move);
    }
  }
  assert.equal((await room()).wolves.winner, 'village'); assert((await room()).wolves.players.every(v => v.role));
  await p.screenshot({ path: `${artifacts}/finished.png`, scale: 'css' });
  await p.waitForTimeout(350); frames = await p.evaluate(() => window.__frames); await p.waitForTimeout(400); assert.equal(await p.evaluate(() => window.__frames), frames, 'Finished screen must stop rendering, with FPS enabled');
  for (const bot of bots) bot.socket.close(); await c.close();
  // A custom sixteen-role room exercises the maximum roster and every ability preview.
  const sixteen = [await seat({ type: 'create', game: 'wolves', name: 'P0' })];
  const roles = ['villager','werewolf','seer','doctor','bodyguard','gunner','fool','wolf_seer','serial_killer','aura_seer','medium','witch','avenger','alpha_wolf','junior_wolf','tough_guy'];
  await command(sixteen[0], { preset: 'custom', roles }, 'wolves_setup');
  for (let i = 1; i < 16; i++) sixteen.push(await seat({ type: 'join', room: sixteen[0].room.code, name: `Player ${i}` }));
  for (let i = 1; i < 16; i++) await command(sixteen[i], true, 'ready'); await command(sixteen[0], undefined, 'start');
  const session = sixteen[0].session;
  for (const saver of [false, true]) for (const [width, height] of [[280,360],[320,480],[390,844],[568,320],[768,1024],[1440,900]]) {
    const { c, p } = await context(width, height, saver, session);
    // Create control connects the browser; the test adapter resumes its existing token.
    const wide = width - 32 >= 800, compact = height < 650, w = wide ? 400 : Math.min(width - 32, 440);
    const fx = wide ? (width - Math.min(width - 32,1100)) / 2 + Math.min(width - 32,1100) * .54 : (width - w) / 2;
    const hero = wide ? 64 : compact ? 146 : 248, targetY = 66 + hero + 130;
    let offset = Math.ceil(Math.max(0, targetY - height + 100) / 80) * 80;
    for (let step = 0; step < offset / 80; step++) { await p.keyboard.press('ArrowDown'); await p.waitForTimeout(80); }
    console.log(`Layout ${width}x${height} ${saver ? 'black' : 'white'}`);
    await p.mouse.click(fx + w / 2, targetY - offset); await p.waitForFunction(() => !!window.__room?.wolves);
    const data = await p.evaluate(() => ({ buffer: [document.querySelector('canvas').width, document.querySelector('canvas').height], background: getComputedStyle(document.body).backgroundColor }));
    assert.deepEqual(data.buffer, [width * 3, height * 3]); assert.equal(data.background, saver ? 'rgb(0, 0, 0)' : 'rgb(255, 255, 255)');
    await p.screenshot({ path: `${artifacts}/village-${width}x${height}-${saver ? 'black' : 'white'}.png`, scale: 'css' });
    await p.keyboard.press('Tab'); await p.keyboard.press('Tab'); // Keyboard focus must be supported.
    await c.close();
  }
  for (const client of sixteen) client.socket.close();
  assert.deepEqual(errors, []); await browser.close();
  console.log('Wolvesville: custom setup UI, advanced roles, hidden projections/chats/votes, complete match, reconnect, 1 Hz countdown, idle FPS, DPR 3 and twelve layouts passed.');
})().catch(async e => { console.error(e); if (activePage && !activePage.isClosed()) { await activePage.screenshot({path: `${artifacts}/failure.png`, scale: 'css'}).catch(() => {}); console.error(await activePage.locator('canvas').getAttribute('aria-label').catch(() => '')); } await qaBrowser?.close().catch(() => {}); process.exit(1); });
