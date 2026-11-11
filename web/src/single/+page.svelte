<script lang="ts">
	// The page of the builds that run without a server (`pnpm build:single`,
	// `pnpm dev:demo`): the reader with made-up data that lives in the browser.
	import ReaderPage from '#lib/components/ReaderPage.svelte';
	import { DemoReader } from '#lib/demo/reader.svelte.ts';

	const reader = new DemoReader();
</script>

<ReaderPage {reader} />
