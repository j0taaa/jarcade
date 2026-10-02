/* Platform glue only: all game rules and UI live in shared Rust code. */
(() => {
  "use strict";
  let timer = null;
  let interrupted = false;
  let ready = false;
  const storageKey = "jarcade.settings.v1";

  function interrupt() {
    interrupted = true;
    clearTimeout(timer);
    timer = null;
    if (typeof navigator.vibrate === "function") { try { navigator.vibrate(0); } catch {} }
    if (ready) window.requestAnimationFrame(() => {
      if (!document.hidden) wasm_exports.jarcade_wake();
    });
  }

  miniquad_add_plugin({
    name: "jarcade_platform",
    version: 1,
    register_plugin(imports) {
      imports.env.jarcade_arm_timer = milliseconds => {
        clearTimeout(timer);
        timer = null;
        if (milliseconds >= 0 && !document.hidden) {
          // jarcade_wake already schedules requestAnimationFrame in Miniquad.
          // Another timer/rAF here would skip display refreshes.
          if (milliseconds === 0) {
            wasm_exports.jarcade_wake();
            return;
          }
          timer = setTimeout(() => {
            timer = null;
            wasm_exports.jarcade_wake();
          }, Math.max(1, milliseconds));
        }
      };
      imports.env.jarcade_haptics_supported = () => Number(typeof navigator.vibrate === "function");
      imports.env.jarcade_haptic = kind => {
        if (document.hidden || typeof navigator.vibrate !== "function") return;
        const patterns = [8, 18, [28, 35, 16], [12, 35, 12, 35, 22]];
        try { navigator.vibrate(patterns[kind] ?? 8); } catch { /* Unsupported hardware or policy. */ }
      };
      imports.env.jarcade_appearance = (saver, warm) => {
        const color = saver ? "#000000" : warm ? "#f5ebd9" : "#ffffff";
        document.documentElement.style.backgroundColor = color;
        document.body.style.backgroundColor = color;
        document.querySelector('meta[name="theme-color"]').setAttribute("content", color);
      };
      imports.env.jarcade_announce = (pointer, length) => {
        const text = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, pointer, length));
        document.getElementById("glcanvas").setAttribute("aria-label", text);
        document.getElementById("announcement").textContent = text;
      };
      imports.env.jarcade_interrupted = () => {
        const result = interrupted;
        interrupted = false;
        return Number(result);
      };
      imports.env.jarcade_load = (pointer, capacity) => {
        try {
          const bytes = new TextEncoder().encode(localStorage.getItem(storageKey) || "");
          const length = Math.min(bytes.length, capacity);
          new Uint8Array(wasm_memory.buffer, pointer, length).set(bytes.subarray(0, length));
          return length;
        } catch { return 0; }
      };
      imports.env.jarcade_save = (pointer, length) => {
        try {
          localStorage.setItem(storageKey, new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, pointer, length)));
          return 1;
        } catch { return 0; }
      };
      imports.env.jarcade_fih_load = (pointer, capacity) => {
        try {
          const bytes = new TextEncoder().encode(localStorage.getItem("jarcade.fih.v1") || "");
          const length = Math.min(bytes.length, capacity);
          new Uint8Array(wasm_memory.buffer, pointer, length).set(bytes.subarray(0, length));
          return length;
        } catch { return 0; }
      };
      imports.env.jarcade_fih_save = (pointer, length) => {
        try {
          localStorage.setItem("jarcade.fih.v1", new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, pointer, length)));
          return 1;
        } catch { return 0; }
      };
      imports.env.jarcade_wavelength_load = (pointer, capacity) => {
        try {
          const bytes = new TextEncoder().encode(localStorage.getItem("jarcade.wavelength.v1") || "");
          const length = Math.min(bytes.length, capacity);
          new Uint8Array(wasm_memory.buffer, pointer, length).set(bytes.subarray(0, length));
          return length;
        } catch { return 0; }
      };
      imports.env.jarcade_wavelength_save = (pointer, length) => {
        try {
          localStorage.setItem("jarcade.wavelength.v1", new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, pointer, length)));
          return 1;
        } catch { return 0; }
      };
    },
    on_init() {
      ready = true;
      document.getElementById("loading").remove();
      const canvas = document.getElementById("glcanvas");
      canvas.addEventListener("contextmenu", event => event.preventDefault());
      canvas.focus();
    },
  });
  window.addEventListener("blur", interrupt);
  window.addEventListener("pagehide", interrupt);
  document.addEventListener("visibilitychange", interrupt);
  window.addEventListener("focus", () => { if (ready) wasm_exports.jarcade_wake(); });
  window.addEventListener("error", () => {
    const loading = document.getElementById("loading");
    if (loading) loading.textContent = "Could not load Jarcade. Please reload.";
  });
})();
