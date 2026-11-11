<script lang="ts">
	// The settings: how the reader looks, how it behaves while reading, the filter
	// rules and the keyboard shortcuts. Everything applies as it is changed.
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
	import Choice from './Choice.svelte';
	import Dialog from './Dialog.svelte';
	import RulesEditor from './RulesEditor.svelte';
	import Shortcuts from './Shortcuts.svelte';

	const allTabs = ['look', 'reading', 'rules', 'keys'] as const;
	type Tab = (typeof allTabs)[number];

	let {
		reader,
		tab = $bindable('look'),
		onclose
	}: { reader: Reader; tab?: Tab; onclose: () => void } = $props();

	// The filter rules have a tab only where the reader has rules.
	const tabs = $derived(allTabs.filter((name) => name !== 'rules' || reader.filters));

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
	<div class="-mt-1 flex flex-wrap gap-1 border-b border-line" role="tablist">
		{#each tabs as name (name)}
			<button
				type="button"
				role="tab"
				aria-selected={tab === name}
				class={[
					'-mb-px cursor-pointer border-b-2 px-3 py-2 text-sm',
					tab === name
						? 'border-accent font-semibold'
						: 'border-transparent text-soft hover:text-ink'
				]}
				onclick={() => (tab = name)}
			>
				{m.settings.tabs[name]}
			</button>
		{/each}
	</div>

	<div class="pt-4" role="tabpanel">
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
								class="flex cursor-pointer items-center gap-1.5 rounded border border-line py-1 pr-3 pl-1.5 text-sm hover:bg-hover has-checked:border-accent has-checked:bg-chosen has-focus-visible:outline-2 has-focus-visible:outline-link"
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
			</div>
		{:else if tab === 'rules' && reader.filters}
			<RulesEditor {reader} book={reader.filters} />
		{:else}
			<Shortcuts />
		{/if}

		{#if tab === 'look' || tab === 'reading'}
			<p class="mt-5 text-xs text-soft">{m.settings.note}</p>
		{/if}
	</div>
</Dialog>
