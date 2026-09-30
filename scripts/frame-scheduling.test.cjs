const { test } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');

function scheduler(blocking = true) {
  const source = fs.readFileSync('web/vendor/mq_js_bundle.js', 'utf8');
  const animation = source.match(/function animation\(\)\{[^}]+\}/)[0];
  const schedule = source.match(/sapp_schedule_update:(function\(\)\{[^}]+\})/)[1];
  const pending = new Map();
  let nextId = 0, cancels = 0, frames = 0;
  const context = {
    animation_frame_timeout: undefined,
    window: {
      blocking_event_loop: blocking,
      requestAnimationFrame(callback) { pending.set(++nextId, callback); return nextId; },
      cancelAnimationFrame(id) { cancels++; pending.delete(id); },
    },
    wasm_exports: { frame() { frames++; } },
  };
  vm.createContext(context);
  vm.runInContext(`${animation}; var schedule = ${schedule};`, context);
  return {
    context, pending, get cancels() { return cancels; }, get frames() { return frames; },
    schedule: () => context.schedule(),
    displayFrame() { const [id, callback] = pending.entries().next().value; pending.delete(id); callback(); },
  };
}

test('rapid touch and key events preserve the earliest pending frame', () => {
  const app = scheduler();
  app.schedule();
  const original = [...app.pending.keys()][0];
  for (let event = 0; event < 1000; event++) app.schedule();
  assert.deepEqual([...app.pending.keys()], [original]);
  assert.equal(app.cancels, 0);
  app.displayFrame();
  assert.equal(app.frames, 1);
  assert.equal(app.pending.size, 0);
});

test('display-paced gameplay renders once per frame and idle stops', () => {
  const app = scheduler();
  app.context.wasm_exports.frame = () => app.schedule();
  app.schedule();
  for (let frame = 0; frame < 240; frame++) {
    app.displayFrame();
    assert.equal(app.pending.size, 1);
  }
  app.context.wasm_exports.frame = () => {};
  app.displayFrame();
  assert.equal(app.pending.size, 0);
  app.schedule();
  app.displayFrame();
  assert.equal(app.pending.size, 0);
});

test('nonblocking runtime keeps a single frame even when requested during drawing', () => {
  const app = scheduler(false);
  app.context.wasm_exports.frame = () => app.schedule();
  app.schedule();
  for (let frame = 0; frame < 120; frame++) {
    app.displayFrame();
    assert.equal(app.pending.size, 1);
  }
  assert.equal(app.cancels, 0);
});
