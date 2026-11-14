<script lang="ts">
	// The page of the builds that run without a server (`pnpm build:single`,
	// `pnpm dev:demo`): the reader with made-up data that lives in the browser.
	// Its feeds are in the language the page starts in: the one picked in the
	// settings, or the browser's. Another language takes effect at the next start.
	import ReaderPage from '#lib/components/ReaderPage.svelte';
	import { demoData } from '#lib/demo/data.ts';
	import { DemoReader } from '#lib/demo/reader.svelte.ts';
	import { startLanguage } from '#lib/i18n/locale.svelte.ts';
	import { settings } from '#lib/settings.svelte.ts';

	const reader = new DemoReader(Date.now, demoData(Date.now(), startLanguage(settings.language)));
</script>

<ReaderPage {reader} />
