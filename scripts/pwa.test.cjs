const { test } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const { createHash, webcrypto } = require('node:crypto');
const origin = 'https://jarcade.test';
const hash = body => createHash('sha256').update(body).digest('hex');

function worker({ fail = false, mixed = false } = {}) {
  const events = new Map(), stores = new Map(), calls = [];
  const key = request => new URL(typeof request === 'string' ? request : request.url, origin).href;
  const caches = {
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const store = stores.get(name);
      return { async put(request, response) { store.set(key(request), response.clone()); },
        async match(request) { return store.get(key(request))?.clone(); } };
    },
    async keys() { return [...stores.keys()]; },
    async delete(name) { return stores.delete(name); },
  };
  const bodies = new Map([['/index.html', 'old coherent shell'], ['/jarcade.wasm?v=test', 'binary']]);
  const assets = [...bodies].map(([url, body]) => ({ url, sha256: hash(body) }));
  const context = { caches, crypto: webcrypto, URL, Request, Uint8Array,
    self: { location: { origin }, clients: { async claim() { calls.push('claim'); } },
      addEventListener(name, fn) { events.set(name, fn); } },
    async fetch(request) {
      const url = new URL(key(request)); calls.push(url.pathname + url.search);
      if (fail && url.pathname === '/jarcade.wasm') throw Error('network lost');
      return new Response(mixed ? 'wrong release' : bodies.get(url.pathname + url.search) || 'network');
    },
  };
  const source = fs.readFileSync('web/sw.js', 'utf8').replace('__BUILD__', 'test')
    .replace('__PRECACHE__', JSON.stringify(assets)).replace('__ROUTES__', JSON.stringify(['/', '/games/sudoku/play', '/games/coupe']));
  vm.runInNewContext(source, context);
  const lifetime = name => new Promise((resolve, reject) => events.get(name)({ waitUntil: p => p.then(resolve, reject) }));
  function fetch(url, { method = 'GET', mode = 'navigate' } = {}) {
    let promise;
    events.get('fetch')({ request: { url: new URL(url, origin).href, method, mode }, respondWith(value) { promise = value; } });
    return promise;
  }
  return { stores, calls, lifetime, fetch, caches };
}

test('verified shell installs atomically; failed or mixed downloads retain the previous worker cache', async () => {
  const good = worker(); await good.lifetime('install');
  assert.equal(good.stores.get('jarcade-shell-v1-test').size, 2);
  for (const options of [{fail:true}, {mixed:true}]) {
    const bad = worker(options);bad.stores.set('jarcade-shell-v1-previous', new Map());
    await assert.rejects(bad.lifetime('install'));
    assert.equal(bad.stores.has('jarcade-shell-v1-test'), false);
    assert.equal(bad.stores.has('jarcade-shell-v1-previous'), true);
  }
});
test('activation removes only older Jarcade shells and never forces an active game to update', async () => {
  const app = worker();await app.lifetime('install');
  app.stores.set('jarcade-shell-v1-previous', new Map());app.stores.set('unrelated-app', new Map());
  await app.lifetime('activate');
  assert.deepEqual([...app.stores.keys()], ['jarcade-shell-v1-test', 'unrelated-app']);
  assert.equal(app.calls.at(-1), 'claim');
  // The worker has no skipWaiting method: a forced update would fail this test.
});
test('offline routes preserve invites and trailing slashes; APIs, unknown paths and other origins bypass the worker', async () => {
  const app = worker();await app.lifetime('install');const before=app.calls.length;
  for (const path of ['/', '/games/sudoku/play/', '/games/coupe?room=ABC123']) {
    assert.equal(await (await app.fetch(path)).text(), 'old coherent shell');
  }
  assert.equal(await (await app.fetch('/jarcade.wasm?v=test', {mode:'cors'})).text(), 'binary');
  assert.equal(app.calls.length, before);
  for (const path of ['/health','/ws','/rooms.json','/not-a-game','https://another.test/']) assert.equal(app.fetch(path), undefined);
  assert.equal(app.fetch('/games/coupe', {method:'POST'}), undefined);
  assert.equal(app.fetch('/jarcade.wasm?v=other', {mode:'cors'}), undefined);
});
test('evicted caches fall back to network instead of trapping an online user', async () => {
  const app=worker();assert.equal(await (await app.fetch('/games/sudoku/play')).text(), 'network');
});
test('registration is event-driven, tolerates unsupported or restricted browsers, and bypasses HTTP cache for updates', async () => {
  for (const supported of [false,true]) for (const secure of [false,true]) {
    const events=new Map(), calls=[];
    const context={window:{isSecureContext:secure,addEventListener(name,fn,options){events.set(name,{fn,options})}},document:{readyState:'loading'},navigator:{}};
    if(supported)context.navigator.serviceWorker={register(...args){calls.push(args);return Promise.reject(Error('restricted'))}};
    vm.runInNewContext(fs.readFileSync('web/pwa.js','utf8'),context);
    assert.equal(calls.length,0);
    if(supported&&secure){assert.equal(events.get('load').options.once,true);events.get('load').fn();await Promise.resolve();
      assert.equal(JSON.stringify(calls),JSON.stringify([['/sw.js',{scope:'/',updateViaCache:'none'}]]));}
    else assert.equal(events.size,0);
  }
});
test('installation manifest and opaque PNG icons have the required dimensions and stable root identity', () => {
  const manifest=JSON.parse(fs.readFileSync('web/manifest.webmanifest','utf8'));
  assert.equal(manifest.id,'/');assert.equal(manifest.scope,'/');assert.equal(manifest.start_url,'/');assert.equal(manifest.display,'standalone');
  for(const [file,size]of [['icon-192',192],['icon-512',512],['apple-touch-icon',180]]){
    const png=fs.readFileSync(`web/icons/${file}.png`);assert.equal(png.readUInt32BE(16),size);assert.equal(png.readUInt32BE(20),size);
  }
  assert(manifest.icons.some(icon=>icon.sizes==='512x512'&&icon.purpose.includes('maskable')));
});
