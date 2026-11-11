import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import SettingsDialog from './SettingsDialog.svelte';
import SubscribeDialog from './SubscribeDialog.svelte';
import VersionCompare from './VersionCompare.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

afterEach(() => {
	Object.assign(settings, defaults);
	useLanguage(null);
});

describe('SettingsDialog', () => {
	it('changes how the reader looks', async () => {
		render(SettingsDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		expect(screen.getByRole('dialog', { name: 'Settings' })).toBeInTheDocument();
		expect(screen.getByRole('tab', { name: 'Appearance' })).toHaveAttribute(
			'aria-selected',
			'true'
		);
		await userEvent.click(screen.getByRole('radio', { name: 'Dark' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Green' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Comfortable' }));
		await userEvent.click(screen.getByRole('radio', { name: 'As badges' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Large' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Serif' }));
		expect(settings).toMatchObject({
			theme: 'dark',
			accent: 'green',
			density: 'comfortable',
			counters: 'badge',
			textSize: 'large',
			readingFont: 'serif'
		});
		await userEvent.selectOptions(screen.getByLabelText('Language'), 'de');
		expect(settings.language).toBe('de');
	});

	it('has a tab for reading, for the rules and for the keys', async () => {
		render(SettingsDialog, { reader: new DemoReader(() => now), onclose: () => {} });
		await userEvent.click(screen.getByRole('tab', { name: 'Reading' }));
		await userEvent.click(screen.getByRole('radio', { name: 'Expanded' }));
		await userEvent.click(screen.getByRole('checkbox', { name: /while scrolling past/ }));
		await userEvent.click(screen.getByRole('checkbox', { name: /first words/ }));
		expect(settings).toMatchObject({ view: 'expanded', markOnScroll: false, teasers: false });

		await userEvent.click(screen.getByRole('tab', { name: 'Filter rules' }));
		expect(screen.getByRole('button', { name: 'Add a rule' })).toBeInTheDocument();
		expect(screen.queryByText(/stay in this browser/)).toBeNull();

		await userEvent.click(screen.getByRole('tab', { name: 'Keyboard' }));
		expect(screen.getByText('Next article, opened')).toBeInTheDocument();
	});

	it('opens on the tab it is given, and closes', async () => {
		const onclose = vi.fn();
		render(SettingsDialog, { reader: new DemoReader(() => now), tab: 'keys', onclose });
		expect(screen.getByRole('tab', { name: 'Keyboard' })).toHaveAttribute('aria-selected', 'true');
		await userEvent.keyboard('{Escape}');
		expect(onclose).toHaveBeenCalledTimes(1);
		await userEvent.click(screen.getByRole('button', { name: 'Close' }));
		expect(onclose).toHaveBeenCalledTimes(2);
		// A click beside the window closes it too, a click in it does not.
		await userEvent.click(screen.getByRole('dialog'));
		expect(onclose).toHaveBeenCalledTimes(2);
		await userEvent.click(screen.getByRole('dialog').parentElement!);
		expect(onclose).toHaveBeenCalledTimes(3);
	});
});

describe('VersionCompare', () => {
	const reader = new DemoReader(() => now);
	const original = reader.articles['feed-ferris'].find((article) => article.versions.length === 0)!;

	it('marks what was added since the earlier version', () => {
		const article = reader.revise(original, 'Nachtrag mit neuen Zahlen.');
		const { container } = render(VersionCompare, {
			article,
			versions: article.versions,
			onclose: () => {}
		});
		expect(screen.getByRole('heading', { name: article.title })).toBeInTheDocument();
		const added = [...container.querySelectorAll('.article-body ins')].map((el) => el.textContent);
		expect(added.join('').trim()).toBe('Nachtrag mit neuen Zahlen.');
		expect(container.querySelector('.article-body del')).toBeNull();
		expect(screen.getByLabelText('Compare the current version with')).toHaveValue('1');
	});

	it('compares with any earlier version, and shows an earlier title', async () => {
		const once = reader.revise(original, 'First addition.');
		const twice = { ...reader.revise(once, 'Second addition.'), title: 'A new title' };
		const { container } = render(VersionCompare, {
			article: twice,
			versions: twice.versions,
			onclose: () => {}
		});
		const added = () =>
			[...container.querySelectorAll('.article-body ins')]
				.map((el) => el.textContent)
				.join('')
				.trim();
		// The latest change first.
		expect(added()).toBe('Second addition.');
		expect(screen.getByText('The title was:')).toHaveTextContent(original.title);
		const options = screen.getAllByRole('option').map((option) => option.textContent?.trim());
		expect(options[0]).toMatch(/^Version 2, fetched /);
		expect(options[1]).toMatch(/^Version 1, fetched /);
		await userEvent.selectOptions(screen.getByRole('combobox'), '1');
		expect(added()).toBe('First addition. Second addition.');
	});

	it('says when the text is the same', () => {
		const versions = [{ rev: 1, fetchedAt: now, title: original.title, html: original.html }];
		render(VersionCompare, { article: original, versions, onclose: () => {} });
		expect(
			screen.getByText('The text of this version is the same as the current one.')
		).toBeInTheDocument();
		expect(screen.queryByText('The title was:')).toBeNull();
	});
});

describe('SubscribeDialog', () => {
	it('subscribes to the address and closes', async () => {
		const reader = new DemoReader(() => now);
		const onclose = vi.fn();
		render(SubscribeDialog, { reader, onclose });
		const address = screen.getByLabelText('Address of the feed or the website');
		expect(address).toHaveFocus();
		const add = screen.getByRole('button', { name: 'Subscribe' });
		expect(add).toBeDisabled();
		await userEvent.type(address, 'https://blog.example/feed.xml{Enter}');
		expect(onclose).toHaveBeenCalledOnce();
		expect(reader.title()).toBe('blog.example');
	});

	it('can be cancelled', async () => {
		const reader = new DemoReader(() => now);
		const feeds = Object.keys(reader.feeds).length;
		const onclose = vi.fn();
		render(SubscribeDialog, { reader, onclose });
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(onclose).toHaveBeenCalledOnce();
		expect(Object.keys(reader.feeds)).toHaveLength(feeds);
	});
});
