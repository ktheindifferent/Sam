// ███████     █████     ███    ███    
// ██         ██   ██    ████  ████    
// ███████    ███████    ██ ████ ██    
//      ██    ██   ██    ██  ██  ██    
// ███████ ██ ██   ██ ██ ██      ██ ██ 
// Copyright 2021-2026 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (ktheindifferent, PixelCoda, p0indexter)
// Licensed under GPLv3....see LICENSE file.

// SAM Service Worker for Offline Functionality
const CACHE_NAME = 'sam-cache-v5';
const STATIC_ASSETS = [
    '/',
    '/index.html',
    '/assets/css/core.css',
    '/assets/css/vendor/bootstrap.min.css',
    '/assets/css/vendor/toastr.min.css',
    '/assets/css/vendor/black-dashboard.min.css',
    '/assets/js/core.js',
    '/assets/js/vendor/jquery.min.js',
    '/assets/js/vendor/bootstrap.min.js',
    '/assets/js/vendor/toastr.min.js',
    '/assets/js/widgets/clock.js',
    '/assets/vendor/fontawesome/css/all.css'
];

// Install event - cache static assets
self.addEventListener('install', event => {
    console.log('[ServiceWorker] Installing...');
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then(cache => {
                console.log('[ServiceWorker] Caching static assets');
                return cache.addAll(STATIC_ASSETS);
            })
            .then(() => {
                console.log('[ServiceWorker] Installation complete');
                return self.skipWaiting();
            })
            .catch(error => {
                console.error('[ServiceWorker] Installation failed:', error);
            })
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', event => {
    console.log('[ServiceWorker] Activating...');
    event.waitUntil(
        caches.keys().then(cacheNames => {
            return Promise.all(
                cacheNames.map(cacheName => {
                    if (cacheName !== CACHE_NAME) {
                        console.log('[ServiceWorker] Deleting old cache:', cacheName);
                        return caches.delete(cacheName);
                    }
                })
            );
        }).then(() => {
            console.log('[ServiceWorker] Activation complete');
            return self.clients.claim();
        })
    );
});

// Fetch event - serve from cache when available
self.addEventListener('fetch', event => {
    const url = new URL(event.request.url);
    
    // Skip caching for API calls and WebSocket connections
    if (url.pathname.startsWith('/api/') || 
        url.pathname.startsWith('/ws') ||
        event.request.method !== 'GET') {
        return;
    }
    
    event.respondWith(
        caches.match(event.request)
            .then(response => {
                // Skip cache for core.js to ensure updates are loaded
                if (event.request.url.includes('/assets/js/core.js') || event.request.url.includes('/assets/js/widgets/notifications.js')) {
                    console.log('[ServiceWorker] Bypassing cache for:', event.request.url);
                    return fetch(event.request);
                }
                
                // Return cached version if available
                if (response) {
                    console.log('[ServiceWorker] Serving from cache:', event.request.url);
                    return response;
                }
                
                // Fetch from network and cache for future use
                return fetch(event.request)
                    .then(response => {
                        // Don't cache if response is not valid
                        if (!response || response.status !== 200 || response.type !== 'basic') {
                            return response;
                        }
                        
                        // Clone the response for caching
                        const responseToCache = response.clone();
                        
                        caches.open(CACHE_NAME)
                            .then(cache => {
                                console.log('[ServiceWorker] Caching new resource:', event.request.url);
                                cache.put(event.request, responseToCache);
                            });
                        
                        return response;
                    })
                    .catch(error => {
                        console.error('[ServiceWorker] Fetch failed:', error);
                        
                        // Return fallback for HTML requests when offline
                        if (event.request.headers.get('accept').includes('text/html')) {
                            return caches.match('/index.html');
                        }
                        
                        throw error;
                    });
            })
    );
});

// Handle messages from the main thread
self.addEventListener('message', event => {
    console.log('[ServiceWorker] Message received:', event.data);
    
    if (event.data && event.data.type === 'SKIP_WAITING') {
        self.skipWaiting();
    }
    
    if (event.data && event.data.type === 'UPDATE_CACHE') {
        // Force update cache
        caches.open(CACHE_NAME)
            .then(cache => {
                return cache.addAll(STATIC_ASSETS);
            })
            .then(() => {
                event.ports[0].postMessage({ success: true });
            })
            .catch(error => {
                console.error('[ServiceWorker] Cache update failed:', error);
                event.ports[0].postMessage({ success: false, error: error.message });
            });
    }
});

// Handle background sync (if needed)
self.addEventListener('sync', event => {
    console.log('[ServiceWorker] Background sync:', event.tag);
    
    if (event.tag === 'background-sync') {
        event.waitUntil(
            // Perform background sync operations
            Promise.resolve()
                .then(() => {
                    console.log('[ServiceWorker] Background sync completed');
                })
                .catch(error => {
                    console.error('[ServiceWorker] Background sync failed:', error);
                })
        );
    }
});

// Handle push notifications (if needed)
self.addEventListener('push', event => {
    console.log('[ServiceWorker] Push received');
    
    if (event.data) {
        const data = event.data.json();
        
        const options = {
            body: data.body || 'New notification from SAM',
            icon: '/assets/favicon-32x32.png',
            badge: '/assets/favicon-16x16.png',
            tag: data.tag || 'sam-notification',
            requireInteraction: data.requireInteraction || false,
            data: data.data || {}
        };
        
        event.waitUntil(
            self.registration.showNotification(data.title || 'SAM', options)
        );
    }
});

// Handle notification clicks
self.addEventListener('notificationclick', event => {
    console.log('[ServiceWorker] Notification click:', event.notification.tag);
    
    event.notification.close();
    
    event.waitUntil(
        clients.openWindow('/')
    );
});