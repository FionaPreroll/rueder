// App passwords in the page with made-up data, to show how an app of another
// maker is set up. They open nothing, as there is no server for an app to
// reach; the rest of the account's settings needs one (see #lib/account.ts).

import type { AppPassword, AppPasswordKeeper, AppPasswords, NewAppPassword } from '#lib/account.ts';

export class DemoApps implements AppPasswordKeeper {
	private apps = $state.raw<AppPassword[]>([]);
	readonly appsVersion = 0;
	private now: () => number;

	constructor(now: () => number = Date.now) {
		this.now = now;
	}

	async appPasswords(): Promise<AppPasswords> {
		return { list: this.apps, max: 20 };
	}

	async createAppPassword(name: string): Promise<NewAppPassword> {
		const letters = 'abcdefghijklmnopqrstuvwxyz234567';
		const random = crypto.getRandomValues(new Uint8Array(24));
		const groups = [0, 4, 8, 12, 16, 20].map((at) =>
			[...random.slice(at, at + 4)].map((byte) => letters[byte & 31]).join('')
		);
		const made = {
			id: Math.max(0, ...this.apps.map((app) => app.id)) + 1,
			name: name.trim(),
			createdAt: this.now(),
			usedAt: null
		};
		this.apps = [...this.apps, made];
		return { ...made, secret: groups.join('-') };
	}

	async revokeAppPassword(id: number) {
		this.apps = this.apps.filter((app) => app.id !== id);
		return true;
	}
}
