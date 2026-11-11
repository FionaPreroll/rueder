<script lang="ts" module>
	// Line icons on a 24 grid, each a single path.
	const paths = {
		star: 'M12 3.5l2.6 5.4 5.9.8-4.3 4.2 1 5.9L12 17l-5.2 2.8 1-5.9-4.3-4.2 5.9-.8z',
		refresh: 'M20 12a8 8 0 1 1-2.34-5.66M20 4v5h-5',
		right: 'M9 6l6 6-6 6',
		down: 'M6 9l6 6 6-6',
		up: 'M6 15l6-6 6 6',
		list: 'M4 6h16M4 12h16M4 18h16',
		expanded: 'M4 5h16v5H4zM4 14h16v5H4z',
		settings:
			'M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1M15 4v4M9 10v4M17 16v4M13 6a2 2 0 1 0 4 0 2 2 0 0 0-4 0M7 12a2 2 0 1 0 4 0 2 2 0 0 0-4 0M15 18a2 2 0 1 0 4 0 2 2 0 0 0-4 0',
		search: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-3.5-3.5',
		close: 'M6 6l12 12M18 6L6 18',
		folder: 'M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z',
		plus: 'M12 5v14M5 12h14',
		warning: 'M12 4l9 16H3zM12 10v4M12 17.2v.1',
		check: 'M5 12.5l4.5 4.5L19 7.5',
		external: 'M14 5h5v5M19 5l-8 8M11 7H6a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h10a1 1 0 0 0 1-1v-5',
		home: 'M4 11l8-7 8 7M6 10v9h12v-9',
		clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3 2',
		hidden:
			'M3 3l18 18M10.6 6.2A9.6 9.6 0 0 1 12 6c5 0 9 6 9 6a15 15 0 0 1-2.6 3M6.5 7.6A15 15 0 0 0 3 12s4 6 9 6a9 9 0 0 0 3.4-.7M9.9 9.9a3 3 0 0 0 4.2 4.2',
		more: 'M5 12h.01M12 12h.01M19 12h.01',
		keyboard: 'M3 7h18v10H3zM7 11h.01M11 11h.01M15 11h.01M8 14h8',
		versions: 'M12 7v5l3 2M12 21a9 9 0 1 0-9-9M3 5v4h4',
		user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 20a8 8 0 0 1 16 0',
		logout: 'M10 5H6a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h4M15 8l4 4-4 4M19 12H9',
		pause: 'M8 5v14M16 5v14',
		play: 'M7 5l12 7-12 7z'
	} as const;

	export type IconName = keyof typeof paths;
</script>

<script lang="ts">
	let {
		name,
		filled = false,
		class: size = 'size-4'
	}: { name: IconName; filled?: boolean; class?: string } = $props();
</script>

<svg
	class={['shrink-0', size]}
	viewBox="0 0 24 24"
	fill={filled ? 'currentColor' : 'none'}
	stroke="currentColor"
	stroke-width="2"
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<path d={paths[name]} />
</svg>
