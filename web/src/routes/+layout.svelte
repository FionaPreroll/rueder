<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import { applySettings, settings } from '#lib/settings.svelte.ts';
	import { keepAwake } from '#lib/wakelock.ts';

	let { children } = $props();

	// Theme, accent color, language and the rest follow the settings as they change.
	$effect(() => applySettings({ ...settings }));

	// The screen stays on while the page is in sight, where the reader asked for it.
	$effect(() => {
		if (settings.wakeLock) return keepAwake();
	});

	// "Like the system" follows the system while the page is open.
	onMount(() => {
		const system = matchMedia('(prefers-color-scheme: dark)');
		const follow = () => applySettings({ ...settings });
		system.addEventListener('change', follow);
		return () => system.removeEventListener('change', follow);
	});
</script>

{@render children()}
