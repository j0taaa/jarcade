/* Generated with a coherent, verified shell. Updates wait for existing tabs. */
"use strict";
const VERSION = "__BUILD__";
const CACHE_PREFIX = "jarcade-shell-v1-";
const CACHE = CACHE_PREFIX + VERSION;
const PRECACHE = __PRECACHE__;
const ROUTES = new Set(__ROUTES__);
const ASSETS = new Set(PRECACHE.map(asset => new URL(asset.url, self.location.origin).href));

self.addEventListener("install", event => {
  event.waitUntil((async () => {
    const cache = await caches.open(CACHE);
    // Wait for every write before deleting a failed installation's cache.
    const results = await Promise.allSettled(PRECACHE.map(async asset => {
      const response = await fetch(new Request(new URL(asset.url, self.location.origin), { cache: "reload" }));
      if (!response.ok || response.redirected) throw new Error("Incomplete Jarcade download");
      const digest = await crypto.subtle.digest("SHA-256", await response.clone().arrayBuffer());
      const hash = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("");
      if (hash !== asset.sha256) throw new Error("Mixed Jarcade versions");
      await cache.put(asset.url, response);
    }));
    const failed = results.find(result => result.status === "rejected");
    if (failed) {
      await caches.delete(CACHE);
      throw failed.reason;
    }
    // No skipWaiting: a new version must not interrupt an open game.
  })());
});

self.addEventListener("activate", event => {
  event.waitUntil((async () => {
    await Promise.all((await caches.keys())
      .filter(name => name.startsWith(CACHE_PREFIX) && name !== CACHE)
      .map(name => caches.delete(name)));
    await self.clients.claim();
  })());
});

self.addEventListener("fetch", event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin) return;
  const path = url.pathname.replace(/\/+$/, "") || "/";
  if (request.mode === "navigate" && ROUTES.has(path)) {
    // Keep the URL (including invitations) while serving this worker's shell.
    event.respondWith(caches.open(CACHE).then(async cache => (await cache.match("/index.html")) || fetch(request)));
  } else if (ASSETS.has(url.href)) {
    event.respondWith(caches.open(CACHE).then(async cache => (await cache.match(request)) || fetch(request)));
  }
  // Rooms, health checks, unrelated paths and lazy online art are never cached.
});
