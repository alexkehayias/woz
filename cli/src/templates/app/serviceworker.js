var cacheName = 'version-{{ version }}';
var appShellFiles = [
  '/index.html',
];

// On install, download files to the cache
self.addEventListener('install', function(e) {
  console.log('[Service Worker] Installing');
  e.waitUntil(
    caches.open(cacheName).then(function(cache) {
      console.log('[Service Worker] Caching app shell and content');
      return cache.addAll(appShellFiles);
    })
  );
});

// Offline uses cache
self.addEventListener('fetch', function(e) {
  e.respondWith(
    caches.match(e.request).then(function(r) {
      console.log('[Service Worker] Fetching resource: ' + e.request.url);
      return r || fetch(e.request).then(function(response) {
        return caches.open(cacheName).then(function(cache) {
          console.log('[Service Worker] Caching new resource: ' + e.request.url);
          cache.put(e.request, response.clone());
          return response;
        });
      });
    })
  );
});

// Clear out existing cache
self.addEventListener('activate', function(e) {
  console.log('[Service Worker] Activating');
  e.waitUntil(
    caches.keys().then(function(keyList) {
      return Promise.all(keyList.map(function(key) {
        if (cacheName.indexOf(key) === -1) {
          console.log('[Service Worker] Deleting cache: ' + key);
          return caches.delete(key);
        }
      }));
    })
  );
});
