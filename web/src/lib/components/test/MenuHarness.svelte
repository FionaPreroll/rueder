<script lang="ts">
	// A menu with two entries and something beside it, for the tests.
	import Menu from '../Menu.svelte';

	let { onpick }: { onpick: (what: string) => void } = $props();
</script>

<Menu label="More">
	{#snippet button()}
		…
	{/snippet}
	<button type="button" role="menuitem" class="menu-item" onclick={() => onpick('first')}>
		First
	</button>
	<button type="button" role="menuitem" class="menu-item" onclick={() => onpick('second')}>
		Second
	</button>
</Menu>
<p>Elsewhere</p>
