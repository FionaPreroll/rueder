<script lang="ts">
	// The routes of `pnpm build:single` (see vite.config.ts): the same layout and page
	// as src/routes, without the page options the hash router does not take.
	import Layout from '../routes/+layout.svelte';

	let { children } = $props();
</script>

<Layout>{@render children()}</Layout>
