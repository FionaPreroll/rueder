<script lang="ts">
	// The settings: how the reader looks, how it behaves while reading, the filter
	// rules and the keyboard shortcuts. Everything applies as it is changed, and
	// the server keeps it with the account, apart from what belongs to the device.
	// What belongs to the account itself - its password, the second factor, apps,
	// invitations, the move and deleting it - is in AccountDialog.svelte.
	import type { Reader } from '#lib/reader.ts';
	import { languageNames, m } from '#lib/i18n/index.ts';
	import {
		accents,
		counterStyles,
		densities,
		readingFonts,
		settings,
		textSizes,
		themes,
		views
	} from '#lib/settings.svelte.ts';
	import { canKeepAwake } from '#lib/wakelock.ts';
	import Choice from './Choice.svelte';
	import Dialog from './Dialog.svelte';
	import RulesEditor from './RulesEditor.svelte';
	import Shortcuts from './Shortcuts.svelte';
	import Tabs from './Tabs.svelte';

	const tabs = ['look', 'reading', 'rules', 'keys'] as const;
	type Tab = (typeof tabs)[number];

	/** Only a touch screen, where a keyboard is unlikely: its keys are left out, unless asked for. */
	const touchOnly =
		typeof matchMedia === 'function' && matchMedia('(hover: none) and (pointer: coarse)').matches;

	let {
		reader,
		tab = $bindable('look'),
		onclose
	}: { reader: Reader; tab?: Tab; onclose: () => void } = $props();

	const asked = tab;
	const shown = $derived(tabs.filter((name) => name !== 'keys' || !touchOnly || asked === 'keys'));

	const swatch: Record<(typeof accents)[number], string> = {
		red: '#c8402e',
		blue: '#1a62d0',
		green: '#1a7a40',
		orange: '#b84c0c',
		purple: '#7a3fc0',
		graphite: '#4b5563'
	};
</script>

<Dialog title={m.settings.title} wide {onclose}>
	<Tabs tabs={shown} open={tab} names={m.settings.tabs} onpick={(name) => (tab = name)} />
	{#if reader.account}
		<!-- Where to find what used to be here, for whoever looks for it. -->
		<p class="mt-2 text-xs text-soft">{m.settings.accountNote}</p>
	{/if}

	<div class="pt-4 sm:min-h-[28rem]" role="tabpanel">
		{#if tab === 'look'}
			<div class="space-y-4">
				<Choice
					label={m.settings.theme}
					options={themes.map((value) => ({ value, label: m.settings.themes[value] }))}
					bind:value={settings.theme}
				/>
				<fieldset>
					<legend class="text-sm font-medium">{m.settings.accent}</legend>
					<div class="mt-1.5 flex flex-wrap gap-2">
						{#each accents as accent (accent)}
							<label
								class="flex cursor-pointer items-center gap-1.5 rounded border border-line py-1 pr-3 pl-1.5 text-sm hover:bg-hover has-checked:border-accent-line has-checked:bg-chosen has-focus-visible:outline-2 has-focus-visible:outline-link"
							>
								<input type="radio" class="sr-only" bind:group={settings.accent} value={accent} />
								<span class="size-4 rounded-full" style:background-color={swatch[accent]}></span>
								{m.settings.accents[accent]}
							</label>
						{/each}
					</div>
				</fieldset>
				<Choice
					label={m.settings.density}
					options={densities.map((value) => ({ value, label: m.settings.densities[value] }))}
					bind:value={settings.density}
				/>
				<Choice
					label={m.settings.counters}
					options={counterStyles.map((value) => ({
						value,
						label: m.settings.counterStyles[value]
					}))}
					bind:value={settings.counters}
				/>
				<Choice
					label={m.settings.textSize}
					options={textSizes.map((value) => ({ value, label: m.settings.textSizes[value] }))}
					bind:value={settings.textSize}
				/>
				<Choice
					label={m.settings.readingFont}
					options={readingFonts.map((value) => ({
						value,
						label: m.settings.readingFonts[value]
					}))}
					bind:value={settings.readingFont}
				/>
				<label class="block text-sm">
					<span class="font-medium">{m.settings.language}</span>
					<select class="mt-1.5 block rounded py-1 text-sm" bind:value={settings.language}>
						<option value="auto">{m.settings.languageAuto}</option>
						{#each languageNames() as { code, name } (code)}
							<option value={code}>{name}</option>
						{/each}
					</select>
				</label>
			</div>
		{:else if tab === 'reading'}
			<div class="space-y-4">
				<Choice
					label={m.settings.view}
					options={views.map((value) => ({ value, label: m.settings.views[value] }))}
					bind:value={settings.view}
				/>
				<label class="flex cursor-pointer items-start gap-2 text-sm">
					<input
						type="checkbox"
						class="mt-0.5 rounded text-accent"
						bind:checked={settings.markOnScroll}
					/>
					{m.settings.markOnScroll}
				</label>
				<label class="flex cursor-pointer items-start gap-2 text-sm">
					<input
						type="checkbox"
						class="mt-0.5 rounded text-accent"
						bind:checked={settings.teasers}
					/>
					{m.settings.teasers}
				</label>
				<label class="flex cursor-pointer items-start gap-2 text-sm">
					<input
						type="checkbox"
						class="mt-0.5 rounded text-accent"
						bind:checked={settings.wakeLock}
					/>
					<span>
						{m.settings.wakeLock}
						{#if !canKeepAwake()}
							<span class="block text-xs text-soft">{m.settings.wakeLockMissing}</span>
						{/if}
					</span>
				</label>
			</div>
		{/if}
		{#if tab === 'look' || tab === 'reading'}
			<p class="mt-5 text-xs text-soft">{reader.demo ? m.settings.noteDemo : m.settings.note}</p>
		{/if}
		{#if tab === 'rules'}
			<RulesEditor {reader} book={reader.filters} />
		{:else if tab === 'keys'}
			<Shortcuts />
		{/if}
	</div>
</Dialog>
