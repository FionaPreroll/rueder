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
});
