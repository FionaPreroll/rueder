/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />
// The service worker: it keeps the files of this version of rueder, so that the
// page starts at once and without a network, and answers with them (see
// src/lib/offline.ts for which requests). A new version waits until the reader
// chooses to reload (src/lib/updates.svelte.ts); the very first takes over by
// itself.

import { self } from '$app/service-worker';
import { assets, immutable } from '$app/manifest';
import { version } from '$app/env';
import { CACHE_PREFIX, SHELL, filesToKeep, way } from '#lib/offline.ts';

const CACHE = `${CACHE_PREFIX}${version}`;
const FILES = filesToKeep([...immutable, ...assets].map((file) => file.path));

self.addEventListener('install', (event) => {
	// Past the browser's HTTP cache, so that no file of an older version slips in.
	event.waitUntil(
		caches
			.open(CACHE)
			.then((cache) => cache.addAll(FILES.map((file) => new Request(file, { cache: 'reload' }))))
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) =>
				Promise.all(
					keys
						.filter((key) => key.startsWith(CACHE_PREFIX) && key !== CACHE)
						.map((key) => caches.delete(key))
				)
			)
			.then(() => self.clients.claim())
	);
});

self.addEventListener('fetch', (event) => {
	const request = event.request;
	const answer = way(request, self.location.origin);
	if (answer === 'network') return;
	event.respondWith(
		caches
			.open(CACHE)
			.then((cache) => cache.match(answer === 'shell' ? SHELL : request, { ignoreVary: true }))
			.then((kept) => kept ?? fetch(request))
	);
});

self.addEventListener('message', (event) => {
	if (event.data === 'skipWaiting') void self.skipWaiting();
});
