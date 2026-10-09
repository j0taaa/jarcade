/* Native browser installation; no prompts, timers or forced game reloads. */
(() => {
  "use strict";
  if (!window.isSecureContext || !("serviceWorker" in navigator)) return;
  const register = () => {
    navigator.serviceWorker.register("/sw.js", { scope: "/", updateViaCache: "none" })
      .catch(() => { /* Storage or worker restrictions never prevent playing. */ });
  };
  if (document.readyState === "complete") register();
  else window.addEventListener("load", register, { once: true });
})();
