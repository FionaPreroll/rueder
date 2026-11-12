// The login page, against a made-up server.

import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Session } from '#lib/api/session.svelte.ts';
import { FakeServer } from '#lib/api/test/fake-server.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import Login from './Login.svelte';

let server: FakeServer;

beforeEach(() => {
	server = new FakeServer();
	server.signedIn = null;
	vi.stubGlobal('fetch', server.fetch);
});

afterEach(() => {
	vi.unstubAllGlobals();
	useLanguage(null);
	history.replaceState(null, '', '/');
});

async function loginPage() {
	const session = new Session();
	await session.check();
	render(Login, { session });
	return session;
}

const fill = async (name: string, password: string) => {
	await userEvent.type(screen.getByLabelText('Name'), name);
	await userEvent.type(screen.getByLabelText('Password'), password);
};
const submit = () => screen.getByRole('button', { name: 'Log in' });

describe('Login', () => {
	it('logs in with name and password', async () => {
		const session = await loginPage();
		expect(screen.getByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		expect(screen.getByLabelText('Name')).toHaveFocus();
		// Password managers find the fields by these.
		expect(screen.getByLabelText('Name')).toHaveAttribute('autocomplete', 'username');
		expect(screen.getByLabelText('Password')).toHaveAttribute('autocomplete', 'current-password');
		expect(screen.getByLabelText('Password')).toHaveAttribute('type', 'password');
		expect(submit()).toBeDisabled();
		expect(screen.queryByRole('alert')).toBeNull();

		await fill('fiona', 'correct horse');
		await userEvent.click(submit());
		await vi.waitFor(() => expect(session.state).toBe('in'));
	});

	it('asks for the code of the second factor, and takes a recovery code too', async () => {
		server.secondFactor = { code: '246810', recovery: ['AAAAA-BBBBB'] };
		const session = await loginPage();
		expect(screen.queryByLabelText('Code')).toBeNull();
		await fill('fiona', 'correct horse{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('This account has a second factor.');
		// The password stays, the code is asked for.
		expect(screen.getByLabelText('Password')).toHaveValue('correct horse');
		const code = screen.getByLabelText('Code');
		expect(code).toHaveFocus();
		expect(code).toHaveAttribute('autocomplete', 'one-time-code');
		expect(submit()).toBeDisabled();

		await userEvent.type(code, '135791{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('That code is not right.');
		expect(screen.getByLabelText('Code')).toHaveValue('');
		expect(session.state).toBe('out');

		await userEvent.type(screen.getByLabelText('Code'), 'aaaaa-bbbbb{Enter}');
		await vi.waitFor(() => expect(session.state).toBe('in'));
		expect(server.secondFactor.recovery).toEqual([]);
	});

	it('says when name or password is wrong, and empties the password', async () => {
		const session = await loginPage();
		await fill('fiona', 'wrong{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('Wrong name or password.');
		expect(screen.getByLabelText('Password')).toHaveValue('');
		expect(screen.getByLabelText('Name')).toHaveValue('fiona');
		expect(session.state).toBe('out');
	});

	it('says how long to wait after too many wrong tries', async () => {
		await loginPage();
		server.lockedFor = 14 * 60 + 5;
		await fill('fiona', 'correct horse{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Too many wrong tries. Try again in 15 minutes.'
		);
	});

	it('says when the server does not answer', async () => {
		await loginPage();
		server.down = true;
		await fill('fiona', 'correct horse{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('The server does not answer.');
	});

	it('offers to try again when the server could not even be asked', async () => {
		server.down = true;
		const session = await loginPage();
		expect(screen.getByRole('alert')).toHaveTextContent('The server does not answer.');
		expect(screen.queryByLabelText('Name')).toBeNull();
		server.down = false;
		await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
		await vi.waitFor(() => expect(session.state).toBe('out'));
		expect(await screen.findByLabelText('Name')).toBeInTheDocument();
		expect(screen.queryByRole('alert')).toBeNull();
	});

	it('offers single sign-on only where the server has it', async () => {
		await loginPage();
		expect(screen.queryByRole('link')).toBeNull();
	});

	it('sends the browser to the server for single sign-on', async () => {
		server.oidc = true;
		await loginPage();
		const link = await screen.findByRole('link', { name: 'Log in with single sign-on' });
		expect(link).toHaveAttribute('href', '/api/oidc/start');
	});

	it.each([
		['failed', 'The login through single sign-on did not work. Try again.'],
		['denied', 'The login through single sign-on was cancelled.'],
		['unknown', 'No account here belongs to that login.']
	])('says how single sign-on ended: %s', async (outcome, text) => {
		history.replaceState(null, '', `/?oidc=${outcome}`);
		await loginPage();
		expect(screen.getByRole('alert')).toHaveTextContent(text);
		// Said once: the address no longer carries it.
		expect(location.search).toBe('');
	});

	it('ignores an outcome it does not know', async () => {
		history.replaceState(null, '', '/?oidc=<script>');
		await loginPage();
		expect(screen.queryByRole('alert')).toBeNull();
		expect(location.search).toBe('');
	});

	it('speaks German', async () => {
		useLanguage('de');
		await loginPage();
		expect(screen.getByRole('heading', { name: 'Bei rueder anmelden' })).toBeInTheDocument();
		await userEvent.type(screen.getByLabelText('Name'), 'fiona');
		await userEvent.type(screen.getByLabelText('Passwort'), 'falsch{Enter}');
		expect(await screen.findByRole('alert')).toHaveTextContent('Name oder Passwort stimmt nicht.');
	});

	it('says that there is no sign-up where there is none', async () => {
		await loginPage();
		expect(screen.getByText(/There is no sign-up\./)).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Make an account' })).toBeNull();
	});
});

describe('registering', () => {
	const invited = () => {
		server.invites = [
			{
				id: 1,
				token: 'a-token',
				max_uses: 1,
				uses: 0,
				created_at: new Date().toISOString(),
				expires_at: new Date(Date.now() + 86_400_000).toISOString(),
				revoked: false
			}
		];
		history.replaceState(null, '', '/?invite=a-token');
	};
	const fillNew = async (name: string, password: string, again = password) => {
		await userEvent.type(screen.getByLabelText('Name'), name);
		await userEvent.type(screen.getByLabelText(/^Password/), password);
		await userEvent.type(screen.getByLabelText('The password once more'), again);
	};
	const make = () => screen.getByRole('button', { name: 'Make the account' });

	it('is a step from the login where the server lets anybody register', async () => {
		server.registrationOpen = true;
		const session = await loginPage();
		await userEvent.click(await screen.findByRole('button', { name: 'Make an account' }));
		expect(screen.getByRole('heading', { name: 'Make an account' })).toBeInTheDocument();
		// Password managers know these as a new password.
		expect(screen.getByLabelText(/^Password/)).toHaveAttribute('autocomplete', 'new-password');
		expect(screen.getByText('At least 8 characters.')).toBeInTheDocument();
		expect(make()).toBeDisabled();

		await fillNew('grace', 'a long password', 'another password');
		expect(screen.getByText('The two passwords differ.')).toBeInTheDocument();
		expect(make()).toBeDisabled();
		await userEvent.clear(screen.getByLabelText('The password once more'));
		await userEvent.type(screen.getByLabelText('The password once more'), 'a long password');
		await userEvent.click(make());
		await vi.waitFor(() => expect(session.state).toBe('in'));
		expect(server.accounts.get('grace')).toBe('a long password');
	});

	it('goes back to the login', async () => {
		server.registrationOpen = true;
		await loginPage();
		await userEvent.click(await screen.findByRole('button', { name: 'Make an account' }));
		await userEvent.click(screen.getByRole('button', { name: 'Log in to an account you have' }));
		expect(screen.getByRole('heading', { name: 'Log in to rueder' })).toBeInTheDocument();
		expect(screen.queryByLabelText('The password once more')).toBeNull();
	});

	it('opens for whoever comes with an invitation, though registration is closed', async () => {
		invited();
		const session = await loginPage();
		expect(await screen.findByText(/You were invited to this rueder\./)).toBeInTheDocument();
		await fillNew('grace', 'a long password');
		await userEvent.click(make());
		await vi.waitFor(() => expect(session.state).toBe('in'));
		expect(server.invites[0].uses).toBe(1);
		// The address no longer carries the invitation.
		expect(location.search).toBe('');
	});

	it('says when the invitation cannot be used any more', async () => {
		invited();
		server.invites[0].uses = 1;
		await loginPage();
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'This invitation cannot be used any more. Ask for a new one.'
		);
		// What is left is the login.
		expect(screen.getByRole('button', { name: 'Log in' })).toBeInTheDocument();
		expect(screen.queryByRole('button', { name: 'Make an account' })).toBeNull();
	});

	it.each([
		['full', 'There is no room for another account here.'],
		['today', 'Enough accounts were made here for one day. Try again tomorrow.']
	] as const)('says when there is no room: %s', async (why, text) => {
		invited();
		await loginPage();
		await screen.findByText(/You were invited/);
		server.noRoom = why;
		await fillNew('grace', 'a long password');
		await userEvent.click(make());
		expect(await screen.findByRole('alert')).toHaveTextContent(text);
	});

	it('says when the name is taken, in German too', async () => {
		useLanguage('de');
		server.registrationOpen = true;
		await loginPage();
		await userEvent.click(await screen.findByRole('button', { name: 'Konto anlegen' }));
		await userEvent.type(screen.getByLabelText('Name'), 'fiona');
		await userEvent.type(screen.getByLabelText(/^Passwort/), 'ein langes Passwort');
		await userEvent.type(screen.getByLabelText('Das Passwort noch einmal'), 'ein langes Passwort');
		await userEvent.click(screen.getByRole('button', { name: 'Konto anlegen' }));
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Ein Konto mit diesem Namen gibt es schon.'
		);
	});
});
