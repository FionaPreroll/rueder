// Inviting others, against a made-up server.

import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ServerReader } from '#lib/api/reader.svelte.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import Invitations from './Invitations.svelte';
import SettingsDialog from './SettingsDialog.svelte';

let server: FakeServer;

beforeEach(() => {
	server = new FakeServer();
	server.admin = false;
	vi.stubGlobal('fetch', server.fetch);
});

afterEach(() => {
	vi.unstubAllGlobals();
	useLanguage(null);
});

const reader = () =>
	new ServerReader({ name: 'fiona', admin: server.admin }, { signOut() {}, expired() {} });
const makeLink = () => screen.getByRole('button', { name: 'Make a link' });

describe('Invitations', () => {
	it('says how many a reader can give, and makes a link that is shown once', async () => {
		server.inviteQuota = 3;
		server.inviteDays = 14;
		render(Invitations, { reader: reader() });
		expect(await screen.findByText(/You can invite 3 more people\./)).toBeInTheDocument();
		expect(screen.getByText('A link is good for 14 days.')).toBeInTheDocument();
		expect(screen.getByText('You have made no links yet.')).toBeInTheDocument();
		expect(screen.getByLabelText('For how many accounts')).toHaveAttribute('max', '3');

		await userEvent.clear(screen.getByLabelText('For how many accounts'));
		await userEvent.type(screen.getByLabelText('For how many accounts'), '2');
		await userEvent.click(makeLink());
		const link = await screen.findByLabelText('The link, to pass on now');
		expect(link).toHaveValue(`${location.origin}/?invite=${server.invites[0].token}`);
		expect(link).toHaveAttribute('readonly');
		expect(await screen.findByText(/You can invite 1 more person\./)).toBeInTheDocument();
		const row = screen.getByText('0 of 2 used').closest('li')!;
		expect(row).toHaveTextContent(/good until /);
		// The list does not show the link again.
		expect(row).not.toHaveTextContent('token');
	});

	it('copies the link', async () => {
		server.inviteQuota = 1;
		// The clipboard of the tests' browser.
		const user = userEvent.setup();
		render(Invitations, { reader: reader() });
		await user.click(await screen.findByRole('button', { name: 'Make a link' }));
		await user.click(await screen.findByRole('button', { name: 'Copy' }));
		expect(await screen.findByRole('button', { name: 'Copied' })).toBeInTheDocument();
		expect(await navigator.clipboard.readText()).toBe(
			`${location.origin}/?invite=${server.invites[0].token}`
		);
	});

	it('takes a link back, which gives its place back', async () => {
		server.inviteQuota = 1;
		render(Invitations, { reader: reader() });
		await userEvent.click(await screen.findByRole('button', { name: 'Make a link' }));
		expect(await screen.findByText('You have no invitations to give.')).toBeInTheDocument();
		// Nothing left: nothing to make.
		expect(screen.queryByRole('button', { name: 'Make a link' })).toBeNull();

		await userEvent.click(screen.getByRole('button', { name: /^Take back the link made/ }));
		expect(await screen.findByText('taken back')).toBeInTheDocument();
		expect(server.invites[0].revoked).toBe(true);
		expect(await screen.findByText(/You can invite 1 more person\./)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: /^Take back/ })).toBeNull();
	});

	it('shows what became of the links', async () => {
		server.inviteQuota = 10;
		const at = (days: number) => new Date(Date.now() + days * 86_400_000).toISOString();
		const link = { token: 't', created_at: at(-3), revoked: false };
		server.invites = [
			{ ...link, id: 1, max_uses: 2, uses: 2, expires_at: at(4) },
			{ ...link, id: 2, max_uses: 3, uses: 1, expires_at: at(-1) }
		];
		render(Invitations, { reader: reader() });
		const rows = await screen.findAllByRole('listitem');
		expect(rows[0]).toHaveTextContent('2 of 2 used');
		expect(rows[0]).toHaveTextContent('used up');
		expect(rows[1]).toHaveTextContent('1 of 3 used');
		expect(rows[1]).toHaveTextContent(/ran out /);
		// The one that ran out gave back the two places it did not use.
		expect(screen.getByText(/You can invite 7 more people\./)).toBeInTheDocument();
	});

	it('lets an admin always invite', async () => {
		server.admin = true;
		render(Invitations, { reader: reader() });
		expect(await screen.findByText(/As an admin you can always invite\./)).toBeInTheDocument();
		expect(screen.getByLabelText('For how many accounts')).toHaveAttribute('max', '1000');
		await userEvent.click(makeLink());
		expect(await screen.findByText('0 of 1 used')).toBeInTheDocument();
	});

	it('says when the invitations ran out in the meantime', async () => {
		server.inviteQuota = 2;
		render(Invitations, { reader: reader() });
		await screen.findByText(/You can invite 2 more people\./);
		// Another tab used them up.
		server.inviteQuota = 0;
		await userEvent.click(makeLink());
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'You have no invitations left for that many.'
		);
	});

	it('says when making a link did not work', async () => {
		server.inviteQuota = 2;
		render(Invitations, { reader: reader() });
		await screen.findByText(/You can invite 2 more people\./);
		server.refuse('POST', /^\/invites$/, 500);
		await userEvent.click(makeLink());
		expect(await screen.findByRole('alert')).toHaveTextContent('That did not work.');
		expect(screen.queryByLabelText('The link, to pass on now')).toBeNull();
	});

	it('says when the list did not come', async () => {
		server.down = true;
		render(Invitations, { reader: reader() });
		expect(await screen.findByRole('alert')).toHaveTextContent('That did not work.');
	});

	it('speaks German', async () => {
		useLanguage('de');
		server.inviteQuota = 1;
		render(Invitations, { reader: reader() });
		expect(await screen.findByText(/Du kannst noch 1 Person einladen\./)).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Link erzeugen' }));
		expect(await screen.findByText('0 von 1 benutzt')).toBeInTheDocument();
	});
});

describe('the settings', () => {
	it('have a tab for invitations where there is a server', async () => {
		server.inviteQuota = 1;
		render(SettingsDialog, { reader: reader(), onclose: () => {} });
		await userEvent.click(screen.getByRole('tab', { name: 'Invitations' }));
		const panel = screen.getByRole('tabpanel');
		expect(await within(panel).findByText(/You can invite 1 more person\./)).toBeInTheDocument();
	});

	it('have none with made-up data, where there is nobody to invite to', () => {
		render(SettingsDialog, { reader: new DemoReader(), onclose: () => {} });
		expect(screen.queryByRole('tab', { name: 'Invitations' })).toBeNull();
	});
});
