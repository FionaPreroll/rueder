import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import { update } from '#lib/updates.svelte.ts';
import Counter from './Counter.svelte';
import Enclosures from './Enclosures.svelte';
import Menu from './Menu.svelte';
import Shortcuts from './Shortcuts.svelte';
import UpdateNotice from './UpdateNotice.svelte';
import MenuHarness from './test/MenuHarness.svelte';

afterEach(() => {
	Object.assign(settings, defaults);
	useLanguage(null);
});

describe('Enclosures', () => {
	const enclosures = [
		{ url: 'https://pod.example/shows/Folge%2012.mp3?token=1', type: 'audio/mpeg', length: 1500 },
		{ url: 'https://pod.example/film.webm', type: 'VIDEO/webm', length: null },
		{ url: 'https://pod.example/', type: null, length: 0 },
		{ url: 'data:audio/wav;base64,AAAA', type: 'audio/wav', length: 3 }
	];

	it('lists the files with a link, their kind and their size', () => {
		render(Enclosures, { enclosures });
		expect(screen.getByRole('heading', { name: 'Attachments' })).toBeInTheDocument();
		const links = screen.getAllByRole('link');
		expect(links.map((link) => link.textContent?.trim())).toEqual([
			'Folge 12.mp3',
			'film.webm',
			'pod.example',
			'audio/wav'
		]);
		expect(links[0]).toHaveAttribute('href', enclosures[0].url);
		expect(links[0]).toHaveAttribute('rel', 'noopener noreferrer');
		expect(links[0].closest('li')).toHaveTextContent('audio/mpeg · 1.5 kB');
		expect(links[2].closest('li')).not.toHaveTextContent('·');
	});

	it('fetches nothing until the reader asks to play, and plays sound and film only', async () => {
		const { container } = render(Enclosures, { enclosures });
		expect(container.querySelector('audio, video')).toBeNull();
		expect(screen.getAllByRole('button').length).toBe(3);

		await userEvent.click(screen.getByRole('button', { name: 'Play: Folge 12.mp3' }));
		expect(container.querySelector('audio')).toHaveAttribute('src', enclosures[0].url);
		expect(container.querySelector('video')).toBeNull();
		expect(screen.queryByRole('button', { name: 'Play: Folge 12.mp3' })).toBeNull();

		await userEvent.click(screen.getByRole('button', { name: 'Play: film.webm' }));
		expect(container.querySelector('video')).toHaveAttribute('src', enclosures[1].url);
	});
});

describe('Counter', () => {
	it('shows nothing for zero', () => {
		const { container } = render(Counter, { count: 0 });
		expect(container).toHaveTextContent('');
	});

	it('is in parentheses, as in Google Reader', () => {
		render(Counter, { count: 30 });
		expect(screen.getByText('(30)')).toHaveAttribute('title', '30 unread');
	});

	it('is a badge when the settings say so', () => {
		settings.counters = 'badge';
		render(Counter, { count: 30 });
		expect(screen.getByText('30')).toHaveAttribute('title', '30 unread');
	});

	it('stops counting at a thousand', () => {
		const { rerender } = render(Counter, { count: 999 });
		expect(screen.getByText('(999)')).toBeInTheDocument();
		rerender({ count: 1000 });
		expect(screen.getByText('(1000+)')).toHaveAttribute('title', '1,000 unread');
	});
});

describe('Menu', () => {
	it('opens under its button and closes on a pick, on Escape and on a click elsewhere', async () => {
		const picked: string[] = [];
		render(MenuHarness, { onpick: (what: string) => picked.push(what) });
		const button = screen.getByRole('button', { name: 'More' });
		expect(button).toHaveAttribute('aria-expanded', 'false');
		expect(screen.queryByRole('menu')).toBeNull();

		await userEvent.click(button);
		expect(button).toHaveAttribute('aria-expanded', 'true');
		await userEvent.click(screen.getByRole('menuitem', { name: 'First' }));
		expect(picked).toEqual(['first']);
		expect(screen.queryByRole('menu')).toBeNull();

		await userEvent.click(button);
		await userEvent.keyboard('{Escape}');
		expect(screen.queryByRole('menu')).toBeNull();
		expect(button).toHaveFocus();

		await userEvent.click(button);
		await userEvent.click(screen.getByText('Elsewhere'));
		expect(screen.queryByRole('menu')).toBeNull();
	});

	it('stays within the window, sideways and at the bottom', async () => {
		vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(360);
		vi.spyOn(window, 'innerHeight', 'get').mockReturnValue(640);
		vi.spyOn(document.documentElement, 'clientWidth', 'get').mockReturnValue(360);
		// Where the menu would be drawn, as a narrow phone would lay it out.
		let drawn = { left: -150, right: 58, top: 100, bottom: 200 };
		let under = { left: 10, right: 50, top: 60, bottom: 92 };
		vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
			this: HTMLElement
		) {
			if (this.getAttribute('role') === 'menu') return drawn as DOMRect;
			return under as DOMRect;
		});
		render(MenuHarness, { onpick: () => {} });
		const button = screen.getByRole('button', { name: 'More' });

		await userEvent.click(button);
		let menu = screen.getByRole('menu');
		expect(menu.style.translate).toBe('158px 0');
		expect(menu).toHaveClass('mt-1');

		await userEvent.click(button);
		under = { left: 300, right: 340, top: 460, bottom: 492 };
		drawn = { left: 300, right: 508, top: 500, bottom: 700 };
		await userEvent.click(button);
		menu = screen.getByRole('menu');
		expect(menu.style.translate).toBe('-156px 0');
		// No room below, more above: it opens upward.
		expect(menu).toHaveClass('bottom-full');

		await userEvent.click(button);
		drawn = { left: 10, right: 218, top: 100, bottom: 200 };
		await userEvent.click(button);
		expect(screen.getByRole('menu').style.translate).toBe('');
		vi.restoreAllMocks();
	});

	it('is a component', () => {
		expect(Menu).toBeTypeOf('function');
	});
});

describe('Shortcuts', () => {
	it('lists the keys with what they do', () => {
		render(Shortcuts);
		expect(screen.getByRole('heading', { name: 'Articles' })).toBeInTheDocument();
		const next = screen.getByText('Next article, opened');
		expect(next.previousElementSibling).toHaveTextContent('j');
		const all = screen.getByText('Mark all as read').previousElementSibling;
		expect(all).toHaveTextContent('Shift+A');
		expect(screen.getByText('Home').previousElementSibling).toHaveTextContent(/g\s+then\s+h/);
		expect(screen.getByText('Open or close the article').previousElementSibling).toHaveTextContent(
			/o\s*\/\s*Enter/
		);
	});

	it('takes a key of ones own, and refuses one that is taken or fixed', async () => {
		render(Shortcuts);
		const change = (what: string) =>
			userEvent.click(screen.getByRole('button', { name: `Change the key for: ${what}` }));
		await change('Next article, opened');
		expect(screen.getByRole('status')).toHaveTextContent('Press a key');
		await userEvent.keyboard('l');
		expect(settings.keys).toEqual({ next: 'l' });
		expect(screen.getByText('Next article, opened').previousElementSibling).toHaveTextContent('l');

		await change('Previous article, opened');
		await userEvent.keyboard('l');
		expect(screen.getByRole('alert')).toHaveTextContent(
			'l is already the key for: Next article, opened'
		);
		await change('Previous article, opened');
		await userEvent.keyboard('g');
		expect(screen.getByRole('alert')).toHaveTextContent('g cannot be given to a shortcut.');
		// Escape gives up, and Shift alone waits for the key it goes with.
		await change('Previous article, opened');
		await userEvent.keyboard('{Escape}');
		expect(settings.keys).toEqual({ next: 'l' });
		await change('Mark all as read');
		await userEvent.keyboard('{Shift>}M{/Shift}');
		expect(settings.keys).toEqual({ next: 'l', markAll: 'M' });

		// After g, the second key is what changes.
		await change('Home');
		await userEvent.keyboard('x');
		expect(settings.keys.goHome).toBe('g x');
		expect(screen.getByText('Home').previousElementSibling).toHaveTextContent(/g\s+then\s+x/);

		await userEvent.click(screen.getByRole('button', { name: "Back to rueder's keys" }));
		expect(settings.keys).toEqual({});
		expect(screen.queryByRole('button', { name: "Back to rueder's keys" })).toBeNull();
	});

	it('speaks German', () => {
		useLanguage('de');
		render(Shortcuts);
		expect(screen.getByText('Startseite').previousElementSibling).toHaveTextContent(/g\s+dann\s+h/);
	});
});

describe('UpdateNotice', () => {
	afterEach(() => {
		update.apply = null;
	});

	it('says a new version is there, reloads into it, or waits', async () => {
		const { unmount } = render(UpdateNotice);
		expect(screen.queryByRole('status')).toBeNull();
		const apply = vi.fn();
		update.apply = apply;
		flushSync();
		expect(screen.getByRole('status')).toHaveTextContent('A new version of rueder is ready.');
		await userEvent.click(screen.getByRole('button', { name: 'Reload' }));
		expect(apply).toHaveBeenCalledTimes(1);
		await userEvent.click(screen.getByRole('button', { name: 'Later' }));
		expect(screen.queryByRole('status')).toBeNull();
		unmount();
	});
});
