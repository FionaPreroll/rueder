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
	import Passkeys from './Passkeys.svelte';
	import RemoveAccount from './RemoveAccount.svelte';
	import SecondFactor from './SecondFactor.svelte';
	import Tabs from './Tabs.svelte';

	let {
		reader,
		tab = $bindable<AccountTab>('password'),
		onclose
	}: { reader: Reader; tab?: AccountTab; onclose: () => void } = $props();

	/** A second factor or an app password is being set up: a click beside would lose it. */
	let busy = $state(false);
	/** The same for a passkey, which sits next to the second factor. */
	let addingKey = $state(false);
	/** Counts changes to the second factor, so that both parts of the tab show them. */
	let security = $state(0);

	const tabs = $derived(tabsOf(reader));
	/** The tab that is open, or the first one this reader has. */
	const open = $derived(tabs.includes(tab) ? tab : (tabs[0] ?? 'password'));
</script>

<Dialog title={m.account.title} wide holds={busy || addingKey} {onclose}>
	<Tabs {tabs} {open} names={m.account.tabs} onpick={(name) => (tab = name)} />
	<p class="mt-2 text-xs text-soft">{m.account.settingsNote}</p>

	<div class="pt-4 sm:min-h-[28rem]" role="tabpanel">
		{#if open === 'password'}
			<ChangePassword {reader} />
		{:else if open === 'security'}
			<SecondFactor {reader} bind:busy version={security} onchange={() => security++} />
			{#if reader.can.passkeys}
				<div class="mt-6 border-t border-line pt-4">
					<Passkeys {reader} bind:busy={addingKey} version={security} onchange={() => security++} />
				</div>
			{/if}
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
</Dialog>
