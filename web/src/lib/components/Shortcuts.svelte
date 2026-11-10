<script lang="ts">
	// The overview of the keyboard shortcuts.
	import { m } from '#lib/i18n/index.ts';
	import { shortcutGroups } from '#lib/keys.ts';
</script>

<div class="grid gap-x-8 gap-y-5 sm:grid-cols-2">
	{#each shortcutGroups(m) as group (group.title)}
		<section>
			<h3 class="text-sm font-semibold">{group.title}</h3>
			<dl class="mt-1.5 space-y-1 text-sm">
				{#each group.shortcuts as shortcut (shortcut.text)}
					<div class="flex items-baseline gap-3">
						<dt class="w-24 shrink-0 whitespace-nowrap">
							{#each shortcut.keys as way, w (way)}
								{#if w > 0}<span class="text-soft">{' / '}</span>{/if}
								{#each way.split(' ') as chord, c (c)}
									{#if c > 0}<span class="text-xs text-soft">{` ${m.keys.then} `}</span>{/if}
									{#each chord.split('+') as key, k (k)}
										{#if k > 0}+{/if}<kbd>{key}</kbd>
									{/each}
								{/each}
							{/each}
						</dt>
						<dd class="text-soft">{shortcut.text}</dd>
					</div>
				{/each}
			</dl>
		</section>
	{/each}
</div>
<p class="mt-5 text-xs text-soft">{m.keys.later}</p>
