/**
 * FYI dashboard service-worker stub (issue #125 / offline PWA design).
 *
 * Phase 0 → light Phase 1 prep:
 * - Lifecycle logging + skipWaiting / clients.claim
 * - Optional no-op message bridge for future outbox flush
 * - Does NOT intercept fetch (network behaviour unchanged)
 * - Does NOT precache app shell yet
 *
 * See docs/bleeding-edge/offline-pwa-design.md
 *
 * Registration (feature-flagged; not default):
 *   navigator.serviceWorker.register('/sw-stub.js')
 *
 * Manifest companion (installability prep):
 *   dashboard/public/manifest.webmanifest
 */
/* eslint-disable no-restricted-globals */

const SW_VERSION = "fyi-dashboard-sw-stub-0.2.0";
const TRUSTED_ORIGIN = self.location.origin;

self.addEventListener("install", (event) => {
  console.info(`[${SW_VERSION}] install`);
  // Activate immediately so a future full SW can replace this stub cleanly.
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  console.info(`[${SW_VERSION}] activate`);
  event.waitUntil(
    (async () => {
      await self.clients.claim();
      // Placeholder for Cache Storage version cleanup in Phase 1.
      // const keys = await caches.keys();
      // await Promise.all(keys.filter((k) => k !== SHELL_CACHE).map((k) => caches.delete(k)));
    })()
  );
});

/**
 * Message protocol (forward-compatible):
 * - { type: 'FYI_SW_PING' } → posts { type: 'FYI_SW_PONG', version }
 * - { type: 'FYI_SW_SKIP_WAITING' } → skipWaiting()
 * Unknown types are ignored.
 */
self.addEventListener("message", (event) => {
  if (event.origin !== TRUSTED_ORIGIN) {
    return;
  }
  const data = event.data;
  if (!data || typeof data !== "object") {
    return;
  }
  if (data.type === "FYI_SW_PING") {
    const port = event.ports && event.ports[0];
    const payload = { type: "FYI_SW_PONG", version: SW_VERSION };
    if (port) {
      port.postMessage(payload);
    } else if (event.source) {
      event.source.postMessage(payload);
    }
    return;
  }
  if (data.type === "FYI_SW_SKIP_WAITING") {
    self.skipWaiting();
  }
});

// Intentionally no `fetch` listener — network behaviour is unchanged in Phase 0.
// Phase 1 will add cache-first shell + offline fallback only for same-origin GETs.
