<script lang="ts">
	// The account itself, apart from the settings of how the page looks and
	// behaves (SettingsDialog.svelte): the password, the second factor, apps of
	// other makers, invitations, moving the account to another rueder, whether
	// this browser reports its errors, and deleting the account. It opens from the
	// menu of the account, where logging out is.
	import { tabsOf, type AccountTab } from '#lib/account.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { Reader } from '#lib/reader.ts';
	import Apps from './Apps.svelte';
	import ChangePassword from './ChangePassword.svelte';
	import Dialog from './Dialog.svelte';
	import Invitations from './Invitations.svelte';
	import MoveAccount from './MoveAccount.svelte';
	import RemoveAccount from './RemoveAccount.svelte';
	import SecondFactor from './SecondFactor.svelte';

	let {
		reader,
		tab = $bindable<AccountTab>('password'),
		onclose
	}: { reader: Reader; tab?: AccountTab; onclose: () => void } = $props();

	/** A second factor or an app password is being set up: a click beside would lose it. */
	let busy = $state(false);

	const tabs = $derived(tabsOf(reader));
	/** The tab that is open, or the first one this reader has. */
	const open = $derived(tabs.includes(tab) ? tab : (tabs[0] ?? 'password'));
</script>

<Dialog title={m.account.title} wide holds={busy} {onclose}>
	<div class="-mt-1 flex flex-wrap gap-1 border-b border-line" role="tablist">
		{#each tabs as name (name)}
			<button
				type="button"
				role="tab"
				aria-selected={open === name}
				class={[
					'-mb-px cursor-pointer border-b-2 px-3 py-2 text-sm',
					open === name
						? 'border-accent font-semibold'
						: 'border-transparent text-soft hover:text-ink'
				]}
				onclick={() => (tab = name)}
			>
				{m.account.tabs[name]}
			</button>
		{/each}
	</div>

	<div class="pt-4" role="tabpanel">
		{#if open === 'password'}
			<ChangePassword {reader} />
		{:else if open === 'security'}
			<SecondFactor {reader} bind:busy />
		{:else if open === 'apps'}
			<Apps {reader} bind:busy />
		{:else if open === 'invites'}
			<Invitations {reader} />
		{:else if open === 'move'}
			<MoveAccount {reader} />
		{:else if open === 'privacy'}
			<div class="space-y-3 text-sm">
				<label class="flex cursor-pointer items-start gap-2 font-medium">
					<input
						type="checkbox"
						class="mt-0.5 rounded text-accent"
						checked={reader.reportsConsent?.on ?? false}
						onchange={(event) => reader.setReportsConsent(event.currentTarget.checked)}
					/>
					{m.settings.reports.send}
				</label>
				<p class="text-soft">{m.settings.reports.intro}</p>
				<div>
					<p class="text-soft">{m.settings.reports.what}</p>
					<ul class="mt-1 list-disc pl-5 text-soft">
						{#each m.settings.reports.holds as line (line)}
							<li>{line}</li>
						{/each}
					</ul>
				</div>
				<p class="text-soft">{m.settings.reports.never}</p>
				<p class="text-xs text-soft">{m.settings.reports.off}</p>
			</div>
		{:else if open === 'remove'}
			<RemoveAccount {reader} />
		{/if}
	</div>

	<p class="mt-5 text-xs text-soft">{m.account.settingsNote}</p>
</Dialog>
