<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import { applySettings, settings } from '#lib/settings.svelte.ts';

	let { children } = $props();

	// Theme, accent color, language and the rest follow the settings as they change.
	$effect(() => applySettings({ ...settings }));

	// "Like the system" follows the system while the page is open.
	onMount(() => {
		const system = matchMedia('(prefers-color-scheme: dark)');
		const follow = () => applySettings({ ...settings });
		system.addEventListener('change', follow);
		return () => system.removeEventListener('change', follow);
	});
</script>

{@render children()}
