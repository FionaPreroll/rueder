<script lang="ts" generics="T extends string">
	// One of a few options, as a row of buttons that are really radio buttons.
	let {
		label,
		options,
		value = $bindable()
	}: { label: string; options: { value: T; label: string }[]; value: T } = $props();
</script>

<fieldset>
	<legend class="text-sm font-medium">{label}</legend>
	<div class="mt-1.5 flex flex-wrap gap-1.5">
		{#each options as option (option.value)}
			<label
				class="cursor-pointer rounded border border-line px-3 py-1 text-sm hover:bg-hover has-checked:border-accent has-checked:bg-chosen has-focus-visible:outline-2 has-focus-visible:outline-link"
			>
				<input type="radio" class="sr-only" bind:group={value} value={option.value} />
				{option.label}
			</label>
		{/each}
	</div>
</fieldset>
