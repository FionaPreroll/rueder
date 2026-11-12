<script lang="ts">
	// The labels in the sidebar: each a list of its articles, with a menu to rename
	// it, give it another color or delete it, and a field to make a new one.
	import { m } from '#lib/i18n/index.ts';
	import { labelColors, nextLabelColor, sameStream, type Label } from '#lib/model.ts';
	import type { Reader } from '#lib/reader.ts';
	import Icon from './Icon.svelte';
	import Menu from './Menu.svelte';

	let { reader, onpick }: { reader: Reader; onpick?: () => void } = $props();

	let naming = $state(false);
	let name = $state('');
	/** The label being renamed. */
	let renaming = $state<string | null>(null);
	let newTitle = $state('');

	function create(event: SubmitEvent) {
		event.preventDefault();
		const title = name.trim();
		if (title) reader.addLabel(title, nextLabelColor(reader.labels.length));
		naming = false;
		name = '';
	}

	function rename(event: SubmitEvent, label: Label) {
		event.preventDefault();
		const title = newTitle.trim();
		if (title && title !== label.title) reader.changeLabel(label.id, { title });
		renaming = null;
	}

	function pick(label: Label) {
		reader.show({ kind: 'label', id: label.id });
		onpick?.();
	}

	const focus = (field: HTMLInputElement) => {
		field.focus();
		field.select();
	};
</script>

<div class="mt-4 flex items-center px-3 pb-1">
	<h2 class="text-xs font-semibold tracking-wide text-soft uppercase">{m.labels.title}</h2>
	<button
		type="button"
		class="plain ml-auto size-6"
		aria-label={m.labels.add}
		title={m.labels.add}
		onclick={() => (naming = true)}
	>
		<Icon name="plus" />
	</button>
</div>

{#if naming}
	<form class="px-3 pb-1" onsubmit={create}>
		<input
			class="w-full rounded px-2 py-1 text-sm"
			placeholder={m.labels.name}
			aria-label={m.labels.name}
			bind:value={name}
			use:focus
			onblur={() => (naming = false)}
			onkeydown={(event) => event.key === 'Escape' && (naming = false)}
		/>
	</form>
{/if}

{#if reader.labels.length === 0 && !naming}
	<p class="px-3 text-xs text-soft">{m.labels.none}</p>
{/if}

<ul>
	{#each reader.labels as label (label.id)}
		{@const chosen = sameStream(reader.stream, { kind: 'label', id: label.id })}
		<li class={['group flex items-center pr-1', chosen ? 'bg-chosen' : 'hover:bg-hover']}>
			{#if renaming === label.id}
				<form class="min-w-0 flex-1 px-3 py-0.5" onsubmit={(event) => rename(event, label)}>
					<input
						class="w-full rounded px-1.5 py-0.5 text-sm"
						aria-label={m.labels.name}
						bind:value={newTitle}
						use:focus
						onblur={() => (renaming = null)}
						onkeydown={(event) => event.key === 'Escape' && (renaming = null)}
					/>
				</form>
			{:else}
				<button
					type="button"
					class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 px-3 py-1 text-left text-sm"
					aria-current={chosen ? 'page' : undefined}
					onclick={() => pick(label)}
				>
					<span class="size-2.5 shrink-0 rounded-full" style:background-color={label.color}></span>
					<span class="truncate">{label.title}</span>
				</button>
				<div class="opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
					<Menu class="plain size-6" align="right" label={m.labels.actions(label.title)}>
						{#snippet button()}
							<Icon name="more" />
						{/snippet}
						<button
							type="button"
							role="menuitem"
							class="menu-item"
							onclick={() => {
								newTitle = label.title;
								renaming = label.id;
							}}
						>
							{m.labels.rename}
						</button>
						<div
							class="flex flex-wrap gap-1.5 px-3 py-1.5"
							role="group"
							aria-label={m.labels.color}
						>
							{#each labelColors as color, index (color)}
								<button
									type="button"
									role="menuitemradio"
									aria-checked={label.color === color}
									aria-label={m.labels.colors[index]}
									title={m.labels.colors[index]}
									class={[
										'size-5 cursor-pointer rounded-full',
										label.color === color && 'ring-2 ring-ink ring-offset-1 ring-offset-paper'
									]}
									style:background-color={color}
									onclick={() => reader.changeLabel(label.id, { color })}
								></button>
							{/each}
						</div>
						<button
							type="button"
							role="menuitem"
							class="menu-item"
							onclick={() => reader.removeLabel(label.id)}
						>
							{m.labels.remove}
						</button>
					</Menu>
				</div>
			{/if}
		</li>
	{/each}
</ul>
