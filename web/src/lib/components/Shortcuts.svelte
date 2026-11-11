<script lang="ts">
	// The keyboard shortcuts, each with a button to give it a key of one's own. The
	// keys go into the settings, and with them to the account.
	import { m } from '#lib/i18n/index.ts';
	import {
		canBeKey,
		isGo,
		keyName,
		keyOf,
		shortcutGroups,
		takenBy,
		type Action
	} from '#lib/keys.ts';
	import { settings } from '#lib/settings.svelte.ts';

	/** The shortcut that waits for its new key. */
	let changing = $state<Action | null>(null);
	/** Why the last key was not taken, and for which shortcut. */
	let refused = $state<{ action: Action; text: string } | null>(null);

	const texts = $derived(
		new Map(shortcutGroups(m).flatMap((group) => group.shortcuts.map((s) => [s.action, s.text])))
	);

	/** The ways to press it, as the overview writes them. */
	const ways = (action: Action) => {
		const key = keyOf(action, settings.keys);
		const chords = isGo(action) ? ['g', key.slice(2)] : [key];
		return action === 'toggle' ? [chords, ['Enter']] : [chords];
	};

	function listen(event: KeyboardEvent) {
		const action = changing;
		if (!action || event.ctrlKey || event.metaKey || event.altKey) return;
		// Shift alone is only the start of a key.
		if (['Shift', 'Control', 'Alt', 'Meta', 'CapsLock'].includes(event.key)) return;
		event.preventDefault();
		event.stopPropagation();
		changing = null;
		if (event.key === 'Escape') return;
		const key = isGo(action) ? `g ${event.key}` : event.key;
		if (!canBeKey(action, key)) {
			refused = { action, text: m.keys.fixed(keyName(event.key)) };
			return;
		}
		const other = takenBy(action, key, settings.keys);
		if (other) {
			refused = { action, text: m.keys.taken(keyName(event.key), texts.get(other) ?? other) };
			return;
		}
		refused = null;
		settings.keys = { ...settings.keys, [action]: key };
	}

	const own = $derived(Object.keys(settings.keys).length > 0);
</script>

<svelte:window onkeydowncapture={listen} />

<div class="grid gap-x-8 gap-y-5 sm:grid-cols-2">
	{#each shortcutGroups(m) as group (group.title)}
		<section>
			<h3 class="text-sm font-semibold">{group.title}</h3>
			<dl class="mt-1.5 space-y-1 text-sm">
				{#each group.shortcuts as shortcut (shortcut.action)}
					<div class="flex items-baseline gap-3">
						<dt class="w-24 shrink-0 whitespace-nowrap">
							{#if changing === shortcut.action}
								<span class="text-xs text-soft" role="status">{m.keys.press}</span>
							{:else}
								{#each ways(shortcut.action) as chords, w (w)}
									{#if w > 0}<span class="text-soft">{' / '}</span>{/if}
									{#each chords as chord, c (c)}
										{#if c > 0}<span class="text-xs text-soft">{` ${m.keys.then} `}</span>{/if}
										{#each keyName(chord).split(/\+(?=.)/) as key, k (k)}
											{#if k > 0}+{/if}<kbd>{key}</kbd>
										{/each}
									{/each}
								{/each}
							{/if}
						</dt>
						<dd class="min-w-0 flex-1 text-soft">
							{shortcut.text}
							{#if refused?.action === shortcut.action}
								<span class="block text-xs text-warn" role="alert">{refused.text}</span>
							{/if}
						</dd>
						<button
							type="button"
							class="plain px-1.5 text-xs"
							aria-label={m.keys.change(shortcut.text)}
							onclick={() => {
								refused = null;
								changing = changing === shortcut.action ? null : shortcut.action;
							}}
						>
							{m.keys.changeShort}
						</button>
					</div>
				{/each}
			</dl>
		</section>
	{/each}
</div>
<div class="mt-5 flex flex-wrap items-center gap-3">
	<p class="text-xs text-soft">{m.keys.note}</p>
	{#if own}
		<button
			type="button"
			class="tool"
			onclick={() => {
				settings.keys = {};
				refused = null;
			}}
		>
			{m.keys.reset}
		</button>
	{/if}
</div>
