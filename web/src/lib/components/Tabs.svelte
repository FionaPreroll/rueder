<script lang="ts" generics="Name extends string">
	// The tabs of a dialog. On a narrow screen they stay in one line, which scrolls
	// sideways, rather than breaking into two; the open one is scrolled into view.
	// The line under them is a shadow inside the row, so that the line of the open tab
	// covers it without reaching out of the row, which scrolls.
	let {
		tabs,
		open,
		names,
		onpick
	}: {
		tabs: readonly Name[];
		open: Name;
		names: Record<Name, string>;
		onpick: (name: Name) => void;
	} = $props();

	const shown = (tab: HTMLElement, chosen: boolean) => {
		if (chosen) tab.scrollIntoView?.({ block: 'nearest', inline: 'nearest' });
	};
</script>

<div
	class="-mt-1 flex [scrollbar-width:none] gap-1 overflow-x-auto shadow-[inset_0_-1px_0_var(--color-line)]"
	role="tablist"
>
	{#each tabs as name (name)}
		<button
			type="button"
			role="tab"
			aria-selected={open === name}
			class={[
				'shrink-0 cursor-pointer border-b-2 px-3 py-2 text-sm whitespace-nowrap',
				open === name
					? 'border-accent-line font-semibold'
					: 'border-transparent text-soft hover:text-ink'
			]}
			onclick={() => onpick(name)}
			{@attach (tab) => shown(tab, open === name)}
		>
			{names[name]}
		</button>
	{/each}
</div>
