const { test } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');

function adapter({ supported = true, blocked = false } = {}) {
  const pulses = [];
  const timers = new Map();
  let nextTimer = 0;
  let wakes = 0;
  const elements = new Map(['glcanvas', 'announcement', 'loading', 'meta'].map(key => [key, {
    events: new Map(), addEventListener(name, fn) { this.events.set(name, fn); },
    attrs: {}, style: {}, textContent: '', setAttribute(k, v) { this.attrs[k] = v; }, remove() {}, focus() {},
  }]));
  const events = new Map();
  let plugin;
  const memory = new WebAssembly.Memory({ initial: 1 });
  const context = {
    TextDecoder, TextEncoder, Uint8Array, Number, Math,
    miniquad_add_plugin(value) { plugin = value; },
    navigator: supported ? { vibrate(pattern) { pulses.push(pattern); if (blocked) throw new Error('blocked'); return true; } } : {},
    document: { hidden: false, body: { style: {} }, documentElement: { style: {} },
      getElementById(id) { return elements.get(id); }, querySelector() { return elements.get('meta'); },
      addEventListener(name, fn) { events.set(name, fn); } },
    window: { addEventListener(name, fn) { events.set(name, fn); }, requestAnimationFrame() {} },
    localStorage: { getItem() { throw new Error('denied'); }, setItem() { throw new Error('denied'); } },
    wasm_memory: memory, wasm_exports: { jarcade_wake() { wakes++; } },
    setTimeout(fn, delay) { timers.set(++nextTimer, { fn, delay }); return nextTimer; }, clearTimeout(id) { timers.delete(id); },
  };
  vm.runInNewContext(fs.readFileSync('web/platform.js', 'utf8'), context);
  const imports = { env: {} };
  plugin.register_plugin(imports);
  return { init: () => plugin.on_init(), env: imports.env, pulses, context, elements, memory, events, timers, get wakes() { return wakes; } };
}

test('haptics map gameplay events to short patterns and stop in background', () => {
  const app = adapter();
  assert.equal(app.env.jarcade_haptics_supported(), 1);
  app.env.jarcade_haptic(0);
  app.env.jarcade_haptic(1);
  app.env.jarcade_haptic(2);
  app.env.jarcade_haptic(3);
  assert.equal(JSON.stringify(app.pulses), JSON.stringify([8,18,[28,35,16],[12,35,12,35,22]]));
  app.context.document.hidden = true;
  app.env.jarcade_haptic(0);
  assert.equal(app.pulses.length, 4);
  app.events.get('blur')();
  assert.equal(app.pulses.at(-1), 0);
});

test('unsupported or blocked vibration never interrupts gameplay', () => {
  const unsupported = adapter({ supported: false });
  assert.equal(unsupported.env.jarcade_haptics_supported(), 0);
  assert.doesNotThrow(() => unsupported.env.jarcade_haptic(1));
  assert.doesNotThrow(() => adapter({ blocked: true }).env.jarcade_haptic(2));
});

test('power saver matches the canvas surround and browser theme color', () => {
  const app = adapter();
  for (const [enabled, color] of [[0, '#ffffff'], [1, '#000000']]) {
    app.env.jarcade_appearance(enabled);
    assert.equal(app.context.document.body.style.backgroundColor, color);
    assert.equal(app.context.document.documentElement.style.backgroundColor, color);
    assert.equal(app.elements.get('meta').attrs.content, color);
  }
});

test('accessible status reflects gameplay without dependence on storage availability', () => {
  const app = adapter();
  const bytes = new TextEncoder().encode('Jarcade. Snake. Paused. Score 3.');
  new Uint8Array(app.memory.buffer).set(bytes);
  app.env.jarcade_announce(0, bytes.length);
  assert.equal(app.elements.get('glcanvas').attrs['aria-label'], 'Jarcade. Snake. Paused. Score 3.');
  assert.equal(app.elements.get('announcement').textContent, 'Jarcade. Snake. Paused. Score 3.');
  assert.equal(app.env.jarcade_load(0, 100), 0);
  assert.equal(app.env.jarcade_save(0, bytes.length), 0);
});


test('display-rate rendering wakes directly without a timeout or extra animation frame', () => {
  const app = adapter();
  for (let frame = 0; frame < 240; frame++) app.env.jarcade_arm_timer(0);
  assert.equal(app.wakes, 240);
  assert.equal(app.timers.size, 0);
});

test('power saver waits for ticks; idle and hidden pages cancel pending work', () => {
  const app = adapter();
  app.env.jarcade_arm_timer(140);
  assert.equal(app.wakes, 0);
  assert.equal(app.timers.size, 1);
  const timer = [...app.timers.values()][0];
  assert.equal(timer.delay, 140);
  timer.fn();
  assert.equal(app.wakes, 1);
  app.timers.clear();
  app.env.jarcade_arm_timer(140);
  app.env.jarcade_arm_timer(0);
  assert.equal(app.wakes, 2);
  assert.equal(app.timers.size, 0);
  app.env.jarcade_arm_timer(140);
  app.env.jarcade_arm_timer(-1);
  assert.equal(app.timers.size, 0);
  app.context.document.hidden = true;
  app.env.jarcade_arm_timer(0);
  app.env.jarcade_arm_timer(140);
  assert.equal(app.wakes, 2);
  assert.equal(app.timers.size, 0);
});


test('right-click belongs to the game canvas instead of opening a browser menu', () => {
  const app = adapter();
  app.init();
  let prevented = false;
  app.elements.get('glcanvas').events.get('contextmenu')({ preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
});

test('Fih storage is separate from settings and bounded by the supplied buffer', () => {
  const app = adapter();
  const store = new Map();
  app.context.localStorage = { getItem(key) { return store.get(key); }, setItem(key, value) { store.set(key, value); } };
  const pet = new TextEncoder().encode('1 78 80 82 76 100 80 0 0');
  new Uint8Array(app.memory.buffer).set(pet);
  assert.equal(app.env.jarcade_fih_save(0, pet.length), 1);
  assert.equal(store.get('jarcade.fih.v1'), new TextDecoder().decode(pet));
  assert(!store.has('jarcade.settings.v1'));
  const bytes = new TextEncoder().encode('3 0 1 27 0');
  new Uint8Array(app.memory.buffer).set(bytes);
  assert.equal(app.env.jarcade_save(0, bytes.length), 1);
  assert.equal(app.env.jarcade_fih_load(100, 6), 6);
  assert.equal(new TextDecoder().decode(new Uint8Array(app.memory.buffer, 100, 6)), '1 78 8');
  assert.equal(store.get('jarcade.settings.v1'), '3 0 1 27 0');
});

test('Fih remains playable when its storage is unavailable', () => {
  const app = adapter();
  assert.equal(app.env.jarcade_fih_load(0, 1024), 0);
  assert.equal(app.env.jarcade_fih_save(0, 10), 0);
});


test('Wavelength saves are isolated, UTF-8 safe, and buffer bounded', () => {
  const app = adapter();
  const store = new Map([['jarcade.fih.v1', 'pet'], ['jarcade.settings.v1', 'settings']]);
  app.context.localStorage = { getItem(key) { return store.get(key); }, setItem(key, value) { store.set(key, value); } };
  const data = JSON.stringify({ custom: ['Frio', 'Quente ☀'], phase: 'Handoff' });
  const bytes = new TextEncoder().encode(data);
  new Uint8Array(app.memory.buffer).set(bytes);
  assert.equal(app.env.jarcade_wavelength_save(0, bytes.length), 1);
  assert.equal(store.get('jarcade.wavelength.v1'), data);
  assert.equal(store.get('jarcade.fih.v1'), 'pet');
  assert.equal(store.get('jarcade.settings.v1'), 'settings');
  assert.equal(app.env.jarcade_wavelength_load(1024, 4096), bytes.length);
  assert.equal(new TextDecoder().decode(new Uint8Array(app.memory.buffer, 1024, bytes.length)), data);
  assert.equal(app.env.jarcade_wavelength_load(1024, 4), 4);
  assert.equal(new TextDecoder().decode(new Uint8Array(app.memory.buffer, 1024, 4)), '{"cu');
});

test('Wavelength remains playable when browser storage is unavailable', () => {
  const app = adapter();
  assert.equal(app.env.jarcade_wavelength_load(0, 4096), 0);
  assert.equal(app.env.jarcade_wavelength_save(0, 10), 0);
});
