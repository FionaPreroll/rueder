<script lang="ts">
	// Inviting others: a link lets somebody make an account. A reader has as many
	// invitations as whoever runs the server gave them, an admin can always
	// invite. The link is shown once, right after it is made.
	import { onMount } from 'svelte';
	import { formatAgo, formatFull } from '#lib/format.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Invites, Reader } from '#lib/reader.ts';
	import Icon from './Icon.svelte';

	let { reader }: { reader: Reader } = $props();

	let list = $state.raw<Invites | null>(null);
	/** Asked and no answer came. */
	let failed = $state(false);
	let uses = $state(1);
	let busy = $state(false);
	/** The link that was just made, until another one is. */
	let link: string | null = $state(null);
	let copied = $state(false);
	let problem: 'none-left' | 'failed' | null = $state(null);
	const now = Date.now();

	/** The most accounts one link can be for. */
	const MAX_USES = 1000;
	const left = $derived(list?.left);
	const admin = $derived(list !== null && left === undefined);
	const most = $derived(admin ? MAX_USES : Math.min(MAX_USES, left ?? 0));

	async function load() {
		const answer = await reader.invites();
		failed = answer === null;
		if (answer) list = answer;
	}

	onMount(() => void load());

	async function make(event: SubmitEvent) {
		event.preventDefault();
		if (busy) return;
		busy = true;
		problem = null;
		copied = false;
		const made = await reader.invite(uses);
		busy = false;
		if (made === null) problem = 'failed';
		else if (made === 'none-left') problem = 'none-left';
		else link = made.link;
		await load();
	}

	async function revoke(id: number) {
		if (await reader.revokeInvite(id)) await load();
		else problem = 'failed';
	}

	async function copy() {
		if (!link) return;
		try {
			await navigator.clipboard.writeText(link);
			copied = true;
		} catch {
			// Without the clipboard the link is there to be marked and copied by hand.
		}
	}

	const select = (event: FocusEvent) => (event.currentTarget as HTMLInputElement).select();
</script>

<div class="space-y-4 text-sm">
	<p class="text-soft">{m.invites.intro}</p>

	{#if list}
		<p>
			{admin ? m.invites.admin : m.invites.left(left ?? 0)}
			<span class="text-soft">{m.invites.goodFor(list.days)}</span>
		</p>

		{#if most > 0}
			<form class="flex flex-wrap items-end gap-2" onsubmit={make}>
				<label class="block">
					<span class="font-medium">{m.invites.uses}</span>
					<input
						class="mt-1 block w-28 rounded py-1 text-sm"
						type="number"
						min="1"
						max={most}
						required
						bind:value={uses}
					/>
				</label>
				<button type="submit" class="primary" disabled={busy}>
					{m.invites.make}
				</button>
			</form>
		{/if}

		{#if problem}
			<p class="flex gap-2 rounded bg-panel px-3 py-2" role="alert">
				<span class="mt-0.5 text-warn"><Icon name="warning" /></span>
				<span>{problem === 'none-left' ? m.invites.noneLeft : m.invites.failed}</span>
			</p>
		{/if}

		{#if link}
			<div class="rounded-md border border-line bg-panel p-3">
				<label class="font-medium" for="invite-link">{m.invites.link}</label>
				<div class="mt-1 flex gap-2">
					<input
						id="invite-link"
						class="block min-w-0 flex-1 rounded py-1 font-mono text-xs"
						readonly
						value={link}
						onfocus={select}
					/>
					<button type="button" class="tool" onclick={copy}>
						{copied ? m.invites.copied : m.invites.copy}
					</button>
				</div>
			</div>
		{/if}

		<section>
			<h3 class="font-semibold">{m.invites.yours}</h3>
			<ul class="mt-2 divide-y divide-line rounded-md border border-line">
				{#each list.invites as invite (invite.id)}
					{@const made = formatAgo(Date.parse(invite.created_at), now)}
					<li class="flex items-center gap-3 px-3 py-2">
						<div class={['min-w-0 flex-1', !invite.usable && 'text-soft']}>
							<p>{m.invites.used(invite.uses, invite.max_uses)}</p>
							<p class="text-xs text-soft">
								{#if invite.revoked}
									{m.invites.revoked}
								{:else if invite.uses >= invite.max_uses}
									{m.invites.usedUp}
								{:else if invite.usable}
									{m.invites.until(formatFull(Date.parse(invite.expires_at)))}
								{:else}
									{m.invites.ran(formatFull(Date.parse(invite.expires_at)))}
								{/if}
							</p>
						</div>
						{#if invite.usable}
							<button
								type="button"
								class="plain px-2 py-1"
								aria-label={m.invites.revokeLink(made)}
								onclick={() => revoke(invite.id)}
							>
								{m.invites.revoke}
							</button>
						{/if}
					</li>
				{:else}
					<li class="px-3 py-2 text-soft">{m.invites.none}</li>
				{/each}
			</ul>
		</section>
	{:else if failed}
		<p class="text-warn" role="alert">{m.invites.failed}</p>
	{:else}
		<p class="text-soft" role="status">{m.list.loadingList}</p>
	{/if}
</div>
