// The start page's own logic: which addresses it takes, and its texts.

import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { MAX_ADDRESS, serverAddress } from '../www/address.js';
import { MAX_URL, MAX_WAYS, legalLinks } from '../www/legal.js';
import { catalogs, textsFor } from '../www/texts.js';

test('takes the address of an https page, as its origin', () => {
	assert.deepEqual(serverAddress(' reader.example '), { address: 'https://reader.example' });
	assert.deepEqual(serverAddress('https://reader.example:8443/feeds/12?x=1#y'), {
		address: 'https://reader.example:8443'
	});
	assert.deepEqual(serverAddress('HTTPS://Reader.Example/'), { address: 'https://reader.example' });
});

test('says why an address is none', () => {
	assert.deepEqual(serverAddress(''), { problem: 'empty' });
	assert.deepEqual(serverAddress('   '), { problem: 'empty' });
	assert.deepEqual(serverAddress(undefined), { problem: 'empty' });
	assert.deepEqual(serverAddress('http://reader.example'), { problem: 'insecure' });
	for (const wrong of [
		'ftp://reader.example',
		'javascript://alert(1)',
		'file:///etc/passwd',
		'https://ada:secret@reader.example',
		'reader example',
		'https://',
		`https://${'a'.repeat(MAX_ADDRESS)}.example`
	]) {
		assert.deepEqual(serverAddress(wrong), { problem: 'invalid' }, wrong);
	}
});

test('has every text in English and in German', () => {
	const shape = (catalog) =>
		Object.entries(catalog)
			.map(
				([key, value]) =>
					`${key}:${typeof value}:${typeof value === 'function' ? value.length : ''}`
			)
			.sort();
	assert.deepEqual(shape(catalogs.de), shape(catalogs.en));
	for (const catalog of Object.values(catalogs)) {
		for (const value of Object.values(catalog)) {
			const text = typeof value === 'function' ? value('x', 'y') : value;
			assert.ok(text.trim().length > 0);
		}
	}
	assert.equal(textsFor('de-AT'), catalogs.de);
	assert.equal(textsFor('fr'), catalogs.en);
	assert.equal(textsFor(undefined), catalogs.en);
});

test('links to the legal texts the server names, and to nothing else', () => {
	const ways = (legal) => JSON.stringify({ oidc: false, registration: false, legal });
	assert.deepEqual(
		legalLinks(
			ways({
				imprint: 'https://operator.example/imprint',
				privacy: null,
				terms: 'http://operator.example/terms?v=2',
				terms_version: '2026-11'
			})
		),
		[
			{ kind: 'imprint', href: 'https://operator.example/imprint' },
			{ kind: 'terms', href: 'http://operator.example/terms?v=2' }
		]
	);
	// A server without them, or one from before them.
	assert.deepEqual(legalLinks(ways({ imprint: null, privacy: null, terms: null })), []);
	assert.deepEqual(legalLinks(JSON.stringify({ oidc: false, registration: false })), []);
	for (const wrong of [
		'',
		'not json',
		'null',
		'[]',
		JSON.stringify({ legal: 'https://operator.example/' }),
		'x'.repeat(MAX_WAYS + 1),
		undefined
	]) {
		assert.deepEqual(legalLinks(wrong), [], String(wrong).slice(0, 40));
	}
	// What is no web address is left out.
	assert.deepEqual(
		legalLinks(
			ways({
				imprint: 'javascript:alert(1)',
				privacy: 'file:///etc/passwd',
				terms: `https://operator.example/${'a'.repeat(MAX_URL)}`
			})
		),
		[]
	);
	assert.deepEqual(
		legalLinks(ways({ imprint: 42, privacy: ['x'], terms: 'operator.example' })),
		[]
	);
});
