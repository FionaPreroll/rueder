// A made-up mail relay for the end-to-end tests: it speaks as much SMTP as
// rueder needs to hand a mail over, keeps every mail, and lets the tests read
// them over HTTP, as a reader would open their inbox.
//
//   node e2e/harness/mail.js
//
//   GET    /mails?to=<address>   the mails to that address, oldest first
//   DELETE /mails                forgets them all

import { createServer as createHttp } from 'node:http';
import { createServer as createSmtp } from 'node:net';
import { MAILBOX_PORT, SMTP_PORT } from './ports.js';

/** Every mail handed over: to whom, its subject and its text. */
const mails = [];

/** The text of a mail as it was before it was put quoted-printable (RFC 2045). */
function unquote(text) {
	const joined = text.replace(/=\r?\n/g, '');
	const bytes = [];
	for (let i = 0; i < joined.length; i++) {
		const hex = joined.slice(i + 1, i + 3);
		if (joined[i] === '=' && /^[0-9A-Fa-f]{2}$/.test(hex)) {
			bytes.push(parseInt(hex, 16));
			i += 2;
		} else {
			bytes.push(...Buffer.from(joined[i]));
		}
	}
	return Buffer.from(bytes).toString('utf8');
}

/** A header's value, with RFC 2047 words such as `=?utf-8?b?…?=` read. */
function header(head, name) {
	const line = new RegExp(`^${name}: (.*(?:\\r?\\n[ \\t].*)*)`, 'im').exec(head)?.[1] ?? '';
	return line
		.replace(/\r?\n[ \t]/g, ' ')
		.replace(/=\?utf-8\?([bq])\?([^?]*)\?=/gi, (_, kind, word) =>
			kind.toLowerCase() === 'b'
				? Buffer.from(word, 'base64').toString('utf8')
				: unquote(word.replace(/_/g, ' '))
		)
		.replace(/\?= =\?/g, '');
}

function keep(data, recipients) {
	const [head, ...rest] = data.split(/\r?\n\r?\n/);
	const raw = rest.join('\n\n');
	const quoted = /content-transfer-encoding: quoted-printable/i.test(head);
	const base64 = /content-transfer-encoding: base64/i.test(head);
	const body = quoted
		? unquote(raw)
		: base64
			? Buffer.from(raw.replace(/\s/g, ''), 'base64').toString('utf8')
			: raw;
	for (const to of recipients) mails.push({ to, subject: header(head, 'Subject'), body });
}

createSmtp((socket) => {
	socket.setEncoding('utf8');
	socket.write('220 mail.test ESMTP\r\n');
	let buffer = '';
	let data = null;
	let recipients = [];
	socket.on('data', (chunk) => {
		buffer += chunk;
		let end;
		while ((end = buffer.indexOf('\r\n')) !== -1) {
			const line = buffer.slice(0, end);
			buffer = buffer.slice(end + 2);
			if (data !== null) {
				if (line === '.') {
					keep(data, recipients);
					data = null;
					recipients = [];
					socket.write('250 2.0.0 kept\r\n');
				} else {
					data += (line.startsWith('..') ? line.slice(1) : line) + '\r\n';
				}
				continue;
			}
			const word = line.split(' ')[0].toUpperCase();
			if (word === 'EHLO' || word === 'HELO') socket.write('250-mail.test\r\n250 8BITMIME\r\n');
			else if (word === 'MAIL') socket.write('250 2.1.0 ok\r\n');
			else if (word === 'RCPT') {
				recipients.push(/<([^>]*)>/.exec(line)?.[1] ?? '');
				socket.write('250 2.1.5 ok\r\n');
			} else if (word === 'DATA') {
				data = '';
				socket.write('354 go ahead\r\n');
			} else if (word === 'RSET' || word === 'NOOP') socket.write('250 ok\r\n');
			else if (word === 'QUIT') socket.end('221 bye\r\n');
			else socket.write('502 5.5.2 no\r\n');
		}
	});
	socket.on('error', () => {});
}).listen(SMTP_PORT, '127.0.0.1');

createHttp((request, response) => {
	const url = new URL(request.url ?? '/', 'http://localhost');
	if (url.pathname !== '/mails') {
		response.writeHead(404).end();
		return;
	}
	if (request.method === 'DELETE') {
		mails.length = 0;
		response.writeHead(204).end();
		return;
	}
	const to = url.searchParams.get('to')?.toLowerCase();
	const found = to ? mails.filter((mail) => mail.to.toLowerCase() === to) : mails;
	response.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify(found));
}).listen(MAILBOX_PORT, '127.0.0.1');
