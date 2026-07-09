/**
 * FYI dashboard service-worker stub (issue #125 / offline PWA design).
 *
 * Phase 0: lifecycle logging only. Does not intercept fetch, does not precache.
 * See docs/bleeding-edge/offline-pwa-design.md for full architecture.
 *
 * Registration is optional and should be feature-flagged from the Next.js app:
 *   navigator.serviceWorker.register('/sw-stub.js')
 */
/* eslint-disable no-restricted-globals */

const SW_VERSION = "fyi-dashboard-sw-stub-0.1.0";

self.addEventListener("install", (event) => {
  // Activate immediately so a future full SW can replace this stub cleanly.
  console.info(`[${SW_VERSION}] install`);
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  console.info(`[${SW_VERSION}] activate`);
  event.waitUntil(self.clients.claim());
});

// Intentionally no `fetch` listener — network behaviour is unchanged in Phase 0.
