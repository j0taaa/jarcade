# Web runtime

`mq_js_bundle.js` is vendored from the `macroquad` 0.4.16 crate's `js/` folder.
Keep it aligned with the exact Macroquad dependency in `Cargo.toml`.

Local fixes: declare `var register_plugin;` inside the quad-net IIFE, before
its assignment. The upstream bundle assigns this undeclared identifier under
strict mode, causing a ReferenceError even when networking is unused.
Coalesce `sapp_schedule_update` calls into one pending animation frame, clearing
its handle at the beginning of `animation`. Repeated input must not cancel and
postpone a frame already waiting for the display. Keep frames scheduled from
inside the callback, and preserve idle blocking behavior. Regression coverage
is in `scripts/frame-scheduling.test.cjs`.

Check updates with `node --check` and an actual browser launch.

Upstream: https://github.com/not-fl3/macroquad
License: `macroquad-LICENSE-MIT` (the crate also offers Apache-2.0).
