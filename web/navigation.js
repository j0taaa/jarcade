/* Browser URLs/history only. Sidebar and page rendering remain in Rust. */
(() => {
  "use strict";
  const aliases = { snake:"snake", minesweeper:"minesweeper", fih:"fih", court:"coupe", coupe:"coupe", reverie:"dicksit", dicksit:"dicksit", wolves:"wolvesville", wolvesville:"wolvesville", codenames:"codenames", wavelength:"wavelength", "table-tennis":"table-tennis", nonograms:"nonograms", sudoku:"sudoku" };
  const mobile = window.matchMedia("(pointer: coarse)").matches;
  let ready = false, initialized = false, route = initialRoute(), serial = 0;
  const events = [];
  function initialRoute() {
    const path = location.pathname.replace(/\/+$/, "") || "/";
    const game = new URLSearchParams(location.search).get("game");
    return path === "/" && typeof aliases[game] === "string" ? `/games/${aliases[game]}` : path;
  }
  const wake = () => { if (ready && !document.hidden) wasm_exports.jarcade_wake(); };
  function emit(drawer) {
    // Keep the final navigation state if several Back gestures arrive before a frame.
    events.length = 0;
    events.push(JSON.stringify({route, drawer}));
    wake();
  }
  function url(path) {
    const value = new URL(location.href);
    value.pathname = path;
    value.searchParams.delete("game");
    // Room codes only belong to the game their invite addressed.
    if (path !== route) value.searchParams.delete("room");
    value.hash = "";
    return value;
  }
  function state(path, drawer) { return {jarcade:1, route:path, drawer, index:++serial}; }
  function sync(path, replace) {
    if (initialized && path === route && !replace) return;
    const target = url(path);
    route = path;
    if ((initialized && replace) || (!initialized && history.state?.jarcade === 1 && history.state.route === path && !history.state.drawer)) {
      history.replaceState({...history.state, route:path}, "", target);
      initialized = true;
      return;
    }
    if (!initialized || replace) {
      history.replaceState(state(path, mobile), "", target);
    } else {
      history.pushState(state(path, mobile), "", target);
    }
    // A single extra entry on touch devices: Back opens navigation; another
    // Back traverses to the preceding page (or leaves). Never reinsert on popstate.
    if (mobile) history.pushState(state(path, false), "", target);
    initialized = true;
  }
  window.addEventListener("popstate", event => {
    route = event.state?.jarcade === 1 ? event.state.route : initialRoute();
    emit(Boolean(event.state?.jarcade === 1 && event.state.drawer));
  });
  miniquad_add_plugin({name:"jarcade_navigation", version:1,
    register_plugin(imports) {
      const copy = (p,n,value) => {
        const bytes = new TextEncoder().encode(value), count = Math.min(n, bytes.length);
        new Uint8Array(wasm_memory.buffer,p,count).set(bytes.subarray(0,count));
        return count;
      };
      imports.env.jarcade_route_load = (p,n) => copy(p,n,route);
      imports.env.jarcade_route_poll = (p,n) => events.length ? copy(p,n,events.shift()) : 0;
      imports.env.jarcade_route_sync = (p,n,replace) => sync(new TextDecoder().decode(new Uint8Array(wasm_memory.buffer,p,n)), Boolean(replace));
      imports.env.jarcade_route_drawer = open => {
        if (mobile && initialized) {
          if (Boolean(open) !== Boolean(history.state?.drawer)) {
            if (open) history.back(); else history.forward();
          }
        } else emit(Boolean(open));
      };
    },
    on_init() { ready = true; },
  });
})();
