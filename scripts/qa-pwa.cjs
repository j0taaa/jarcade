// Real worker installation, cold offline launch, updates and saved gameplay.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const http = require('node:http');
const { createHash } = require('node:crypto');
const base = process.env.JARCADE_QA_URL || 'http://127.0.0.1:8092';
const chrome = process.env.JARCADE_CHROME || '/usr/bin/google-chrome';
const artifacts = '/tmp/jarcade-pwa-qa'; fs.mkdirSync(artifacts, {recursive:true});
const wait = (p, text) => p.waitForFunction(text => document.querySelector('canvas')?.getAttribute('aria-label')?.includes(text), text);
async function start(profile) {
  const context = await chromium.launchPersistentContext(profile, {executablePath:chrome,
    args:['--no-sandbox','--enable-unsafe-swiftshader'], viewport:{width:390,height:844}, deviceScaleFactor:3, hasTouch:true});
  await context.addInitScript(() => {
    if(!localStorage.getItem('jarcade.settings.v1')) localStorage.setItem('jarcade.settings.v1','3 0 1 0 1');
    window.__frames=0;const raf=requestAnimationFrame;
    window.requestAnimationFrame=cb=>raf.call(window,t=>{window.__frames++;cb(t)});
  });
  return context;
}
async function disableHttpCache(context,p) {
  const cdp=await context.newCDPSession(p);await cdp.send('Network.enable');await cdp.send('Network.setCacheDisabled',{cacheDisabled:true});return cdp;
}
async function idle(p) {
  await p.waitForTimeout(600);const frames=await p.evaluate(()=>window.__frames);
  await p.waitForTimeout(300);assert.equal(await p.evaluate(()=>window.__frames),frames,'PWA must not wake idle gameplay');
}
async function keys(p,sequence) {for(const k of sequence){await p.keyboard.press(k);await p.waitForTimeout(90)}}
async function installationAndOffline() {
  const profile=fs.mkdtempSync(path.join(os.tmpdir(),'jarcade-pwa-profile-'));
  let context;
  try {
    context=await start(profile);const p=await context.newPage();const errors=[];p.on('pageerror',e=>errors.push(e.message));
    await p.goto(`${base}/games/tic-tac-toe`);await wait(p,'Jogo da Velha.');
    await p.evaluate(()=>navigator.serviceWorker.ready);
    await p.waitForFunction(()=>Boolean(navigator.serviceWorker.controller));
    const cdp=await disableHttpCache(context,p);
    const app=await cdp.send('Page.getAppManifest');assert.deepEqual(app.errors,[]);
    const manifest=JSON.parse(app.data);assert.equal(manifest.display,'standalone');assert.equal(manifest.id,'/');
    const install=await cdp.send('Page.getInstallabilityErrors');assert.deepEqual(install.installabilityErrors,[]);
    const cache=await p.evaluate(async()=>{
      const names=(await caches.keys()).filter(n=>n.startsWith('jarcade-shell-v1-'));
      return {names,urls:(await (await caches.open(names[0])).keys()).map(r=>r.url)};
    });
    assert.equal(cache.names.length,1);assert(cache.urls.some(url=>url.includes('jarcade.wasm?v=')));
    assert(!cache.urls.some(url=>url.includes('/health')||url.includes('/ws')||url.includes('assets/reverie')));
    await keys(p,['1','4','2']);await wait(p,'Tabuleiro: [1, 1, 0, 2');await idle(p);
    await p.screenshot({path:`${artifacts}/installed.png`,scale:'css'});
    await context.close();context=await start(profile);await context.setOffline(true);
    const offline=await context.newPage();offline.on('pageerror',e=>errors.push(e.message));await disableHttpCache(context,offline);
    const response=await offline.goto(`${base}/games/tic-tac-toe`);assert(response.fromServiceWorker());
    await wait(offline,'Tabuleiro: [1, 1, 0, 2');await keys(offline,['5','3']);await wait(offline,'Won(0)');await idle(offline);
    await offline.screenshot({path:`${artifacts}/cold-offline-win.png`,scale:'css'});
    // These routes were never loaded online in this profile.
    for(const [route,text]of [['/settings','Jarcade. Settings.'],['/games/sudoku','Jarcade. Sudoku.'],
      ['/games/nonograms','Jarcade. Nonograms.'],['/games/fih/bathroom','Jarcade. Fih.'],
      ['/games/mastermind','CodeCover.'],['/games/guess-who','Cover { player: 0'],['/games/wavelength','Jarcade. Wavelength.']]) {
      assert((await offline.goto(base+route)).fromServiceWorker());await wait(offline,text);
    }
    await offline.goto(`${base}/games/coupe?room=ABC123`);await wait(offline,'Jarcade. Coupe.');
    assert(offline.url().includes('room=ABC123'));
    assert.equal(await offline.evaluate(async()=>{try{await fetch('/health');return true}catch{return false}}),false);
    await context.setOffline(false);assert.equal((await offline.request.get(`${base}/health`)).status(),200);
    await offline.goto(`${base}/games/tic-tac-toe`);await wait(offline,'Won(0)');assert.deepEqual(errors,[]);
    console.log('Manifest installability, worker-controlled cold offline launch after browser restart, saved win, seven unvisited routes, private covers, invites, live APIs and idle FPS passed.');
  } finally {if(context)await context.close();fs.rmSync(profile,{recursive:true,force:true});}
}
async function updates() {
  const html=fs.readFileSync('dist/index.html','utf8'), worker=fs.readFileSync('dist/sw.js','utf8');
  const old=html.match(/name="jarcade-build" content="([a-f0-9]+)"/)[1], next='000000000001';
  const nextHtml=html.replaceAll(old,next);
  const nextWorker=worker.replaceAll(old,next).replace(createHash('sha256').update(html).digest('hex'),createHash('sha256').update(nextHtml).digest('hex'));
  const routes=new Set(JSON.parse(worker.match(/const ROUTES = new Set\((.*)\);/)[1]));let newer=false;
  const server=http.createServer((req,res)=>{
    const url=new URL(req.url,'http://localhost');let file=url.pathname;
    if(routes.has(file))file='/index.html';
    if(file==='/sw.js'){res.setHeader('Content-Type','application/javascript');res.end(newer?nextWorker:worker);return;}
    if(file==='/index.html'){res.setHeader('Content-Type','text/html');res.end(newer?nextHtml:html);return;}
    const target=path.resolve('dist','.'+file);
    if(!target.startsWith(path.resolve('dist')+path.sep)||!fs.existsSync(target)){res.writeHead(404);res.end();return;}
    const types={'.js':'application/javascript','.wasm':'application/wasm','.webmanifest':'application/manifest+json','.png':'image/png','.svg':'image/svg+xml'};
    res.setHeader('Content-Type',types[path.extname(target)]||'application/octet-stream');res.end(fs.readFileSync(target));
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const url=`http://127.0.0.1:${server.address().port}`;
  const profile=fs.mkdtempSync(path.join(os.tmpdir(),'jarcade-pwa-update-'));let context;
  try{
    context=await start(profile);const p=await context.newPage();await p.goto(`${url}/games/tic-tac-toe`);await wait(p,'Jogo da Velha.');
    await p.evaluate(()=>navigator.serviceWorker.ready);await p.waitForFunction(()=>Boolean(navigator.serviceWorker.controller));
    await keys(p,['1','4','2']);newer=true;
    await p.evaluate(async()=>{const reg=await navigator.serviceWorker.getRegistration();await reg.update()});
    await p.waitForFunction(async()=>Boolean((await navigator.serviceWorker.getRegistration()).waiting));
    assert.equal(await p.locator('meta[name=jarcade-build]').getAttribute('content'),old);
    await keys(p,['5','3']);await wait(p,'Won(0)');
    const second=await context.newPage();assert((await second.goto(`${url}/games/mastermind`)).fromServiceWorker());
    assert.equal(await second.locator('meta[name=jarcade-build]').getAttribute('content'),old);
    await p.close();await second.close();await new Promise(resolve=>setTimeout(resolve,500));
    const fresh=await context.newPage();await fresh.goto(`${url}/games/tic-tac-toe`);await wait(fresh,'Won(0)');
    assert.equal(await fresh.locator('meta[name=jarcade-build]').getAttribute('content'),next);
    assert.deepEqual(await fresh.evaluate(()=>caches.keys()),['jarcade-shell-v1-'+next]);
    console.log('Real worker upgrade waits for all game tabs, stays coherent across a second tab, activates on close, removes old caches and preserves score.');
  }finally{if(context)await context.close();await new Promise(resolve=>server.close(resolve));fs.rmSync(profile,{recursive:true,force:true});}
}
(async()=>{await installationAndOffline();if(!process.env.JARCADE_QA_PUBLIC_ONLY)await updates()})().catch(e=>{console.error(e);process.exitCode=1});
