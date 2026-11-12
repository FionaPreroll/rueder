// The start page's own logic: which addresses it takes, and its texts.

import { strict as assert } from 'node:assert';
import { test } from 'node:test';
import { MAX_ADDRESS, serverAddress } from '../www/address.js';
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
