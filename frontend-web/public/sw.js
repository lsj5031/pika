// PWA Service Worker for offline capability

const CACHE_NAME = 'pika-v1';
const urlsToCache = [
  '/',
  '/index.html',
  '/manifest.json',
];

self.addEventListener('install', (event) => {
  // Take over immediately
  self.skipWaiting();
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(urlsToCache))
  );
});

self.addEventListener('fetch', (event) => {
  // Only handle GET requests
  if (event.request.method !== 'GET') return;

  // Only handle same-origin requests
  const url = new URL(event.request.url);
  if (url.origin !== self.location.origin) return;

  // Skip API and WebSocket requests
  if (url.pathname.startsWith('/api') || url.pathname === '/ws') return;

  event.respondWith(
    caches.match(event.request).then((response) => {
      if (response) {
        return response;
      }
      return fetch(event.request);
    })
  );
});

self.addEventListener('activate', (event) => {
  // Claim all clients immediately
  event.waitUntil(
    Promise.all([
      clients.claim(),
      caches.keys().then((cacheNames) =>
        Promise.all(
          cacheNames.map((cacheName) => {
            if (cacheName !== CACHE_NAME) {
              return caches.delete(cacheName);
            }
          })
        )
      ),
    ])
  );
});
