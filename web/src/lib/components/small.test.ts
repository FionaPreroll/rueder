import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useLanguage } from '#lib/i18n/locale.svelte.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import Counter from './Counter.svelte';
import Enclosures from './Enclosures.svelte';
import Menu from './Menu.svelte';
import Shortcuts from './Shortcuts.svelte';
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

	it('speaks German', () => {
		useLanguage('de');
		render(Shortcuts);
		expect(screen.getByText('Startseite').previousElementSibling).toHaveTextContent(/g\s+dann\s+h/);
	});
});
