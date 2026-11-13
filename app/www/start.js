// The start page of the app: it asks for the address of the reader's rueder, opens
// it, and holds whether the app looks for new versions of itself. The page of
// rueder itself comes from the server, as in a browser; only this page talks to the
// app's own plugin (RuederPlugin.java), which answers nothing else.

import { serverAddress } from './address.js';
import { legalLinks } from './legal.js';
import { textsFor } from './texts.js';

const t = textsFor(navigator.language);
const call = (method, options = {}) => window.Capacitor.nativePromise('Rueder', method, options);

const $ = (id) => document.getElementById(id);

/** Shows a text below the form; an alert for a problem. */
function say(text, problem = false) {
	const line = $('message');
	line.textContent = text;
	line.setAttribute('role', problem ? 'alert' : 'status');
	line.classList.toggle('problem', problem);
}

function fill(state) {
	document.documentElement.lang = navigator.language.toLowerCase().startsWith('de') ? 'de' : 'en';
	$('intro').textContent = t.intro;
	$('address-label').textContent = t.address;
	$('open').textContent = t.open;
	$('address').value = state.server ?? '';
	$('version').textContent = t.version(state.version, state.build);
	// Only builds that may update themselves offer it; the Play Store's do not.
	$('updates').hidden = !state.updatesAvailable;
	$('updates-label').textContent = t.updates;
	$('updates-hint').textContent = t.updatesHint;
	$('look').textContent = t.lookNow;
	$('check-updates').checked = state.checkUpdates;
}

async function open(event) {
	event?.preventDefault();
	const typed = serverAddress($('address').value);
	if (!typed.address) {
		say(t[typed.problem], true);
		$('address').focus();
		return;
	}
	say(t.opening);
	$('open').disabled = true;
	try {
		await call('save', { server: typed.address, checkUpdates: $('check-updates').checked });
		await call('open');
	} catch {
		$('open').disabled = false;
		say(t.invalid, true);
	}
}

async function look() {
	$('look').disabled = true;
	say(t.looking);
	try {
		const { newer } = await call('lookForUpdate');
		// A newer build is offered by the app itself, in its own dialog.
		if (!newer) say(t.current);
		else say('');
	} catch {
		say(t.lookFailed, true);
	} finally {
		$('look').disabled = false;
	}
}

/**
 * Links to the legal texts of the server's operator, where the saved server
 * names them. A server that does not answer simply has none here.
 */
async function showLegal() {
	let links;
	try {
		links = legalLinks((await call('legal')).body);
	} catch {
		return;
	}
	const nav = $('legal');
	nav.replaceChildren(
		...links.map(({ kind, href }) => {
			const link = document.createElement('a');
			link.href = href;
			link.textContent = t[kind];
			// Pages of other sites open in the browser, not in the app.
			link.rel = 'noopener noreferrer';
			return link;
		})
	);
	nav.hidden = links.length === 0;
}

async function start() {
	const state = await call('state');
	fill(state);
	$('form').addEventListener('submit', open);
	$('look').addEventListener('click', () => void look());
	$('check-updates').addEventListener('change', (event) => {
		void call('save', { checkUpdates: event.currentTarget.checked });
	});
	const asked = new URLSearchParams(location.search);
	if (asked.has('unreachable') && state.server) {
		say(t.unreachable(state.server), true);
	} else if (state.server && !asked.has('setup') && !state.setup) {
		// A known server opens at once; the launcher's "Server and updates" and the
		// menu of rueder's page come here to change it.
		say(t.opening);
		await call('open');
		return;
	}
	$('form').hidden = false;
	if (!state.server) $('address').focus();
	else void showLegal();
}

void start();
