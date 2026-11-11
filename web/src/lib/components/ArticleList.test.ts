import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import type { Stream } from '#lib/model.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import ArticleList from './ArticleList.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

function setup(stream: Stream = { kind: 'feed', id: 'feed-tagesbote' }) {
	const reader = new DemoReader(() => now);
	reader.show(stream);
	const onversions = vi.fn();
	const { container } = render(ArticleList, { reader, now, onversions });
	const row = (id: string) => container.querySelector<HTMLElement>(`[data-article="${id}"]`)!;
	return { reader, onversions, row, container };
}

afterEach(() => Object.assign(settings, defaults));

describe('the list view', () => {
	it('has a row per article: unread bold, read quiet', () => {
		const { reader, row } = setup();
		expect(screen.getAllByRole('listitem')).toHaveLength(Math.min(40, reader.list.length));
		const unread = reader.list.find((article) => !reader.isRead(article))!;
		const read = reader.list.find((article) => reader.isRead(article))!;
		expect(within(row(unread.id)).getByText(unread.title)).toHaveClass('font-semibold');
		expect(within(row(read.id)).getByText(read.title)).not.toHaveClass('font-semibold');
		expect(row(unread.id)).toHaveTextContent(unread.teaser.slice(0, 30));
		// In a single feed the rows do not repeat its name.
		expect(row(unread.id)).not.toHaveTextContent('Tagesbote');
	});

	it('names the feed in a list of several, and leaves the teasers out when asked', async () => {
		settings.teasers = false;
		const { reader, row } = setup({ kind: 'folder', id: 'folder-news' });
		const article = reader.list[0];
		expect(row(article.id)).toHaveTextContent(reader.feeds[article.feedId].title);
		expect(row(article.id)).not.toHaveTextContent(article.teaser.slice(0, 30));
	});

	it('opens an article in place and marks it as read', async () => {
		const { reader, row } = setup();
		const article = reader.list.find((entry) => !reader.isRead(entry))!;
		const unread = reader.unread(article.feedId);
		const line = within(row(article.id)).getByRole('button', { expanded: false });
		await userEvent.click(line);
		expect(line).toHaveAttribute('aria-expanded', 'true');
		expect(within(row(article.id)).getByRole('link', { name: article.title })).toHaveAttribute(
			'href',
			article.link
		);
		expect(reader.unread(article.feedId)).toBe(unread - 1);
		await userEvent.click(line);
		expect(within(row(article.id)).queryByRole('link')).toBeNull();
	});

	it('keeps an article unread, stars it, and opens the original safely', async () => {
		const { reader, row } = setup();
		const article = reader.list.find((entry) => !reader.isRead(entry))!;
		reader.select(article);
		const open = await vi.waitFor(() => within(row(article.id)).getByRole('checkbox'));
		expect(open).not.toBeChecked();
		await userEvent.click(open);
		expect(reader.isRead(article)).toBe(false);

		const [star] = within(row(article.id)).getAllByRole('button', { name: 'Add star' });
		await userEvent.click(star);
		expect(reader.isStarred(article)).toBe(true);
		expect(within(row(article.id)).getAllByRole('button', { name: /Remove star/ })).toHaveLength(2);

		const original = within(row(article.id)).getByRole('link', { name: 'Open the original' });
		expect(original).toHaveAttribute('target', '_blank');
		expect(original).toHaveAttribute('rel', 'noopener noreferrer');
	});

	it('marks updated articles and offers to compare the versions', async () => {
		const { reader, onversions, row } = setup();
		const article = reader.list.find((entry) => entry.versions.length > 0)!;
		expect(within(row(article.id)).getByText('updated')).toHaveAttribute(
			'title',
			expect.stringMatching(/^Changed since it was first fetched; \d earlier version/)
		);
		reader.select(article);
		const compare = await vi.waitFor(() =>
			within(row(article.id)).getByRole('button', { name: 'Compare versions' })
		);
		await userEvent.click(compare);
		expect(onversions).toHaveBeenCalledWith(article);
	});

	it('holds new articles back behind a button', async () => {
		const { reader } = setup({ kind: 'feed', id: 'feed-labnotes' });
		const length = reader.list.length;
		expect(screen.queryByRole('button', { name: /new article/ })).toBeNull();
		reader.arrive('feed-labnotes');
		reader.arrive('feed-labnotes');
		const pill = await vi.waitFor(() => screen.getByRole('button', { name: '2 new articles' }));
		expect(screen.getAllByRole('listitem')).toHaveLength(length);
		await userEvent.click(pill);
		expect(screen.getAllByRole('listitem')).toHaveLength(length + 2);
		expect(screen.queryByRole('button', { name: /new article/ })).toBeNull();
	});

	it('shows more on demand', async () => {
		const { reader } = setup({ kind: 'all' });
		expect(screen.getAllByRole('listitem')).toHaveLength(40);
		await userEvent.click(screen.getByRole('button', { name: 'Show more' }));
		expect(screen.getAllByRole('listitem')).toHaveLength(80);
		expect(reader.limit).toBe(80);
	});

	it('says why an article is hidden', async () => {
		const { reader, row } = setup({ kind: 'hidden' });
		const article = reader.list[0];
		reader.select(article);
		await vi.waitFor(() =>
			expect(row(article.id)).toHaveTextContent(/Hidden by the rule: (Title|Author) /)
		);
	});
});

describe('an empty list', () => {
	it('says there is nothing unread, and offers everything', async () => {
		const { reader } = setup({ kind: 'feed', id: 'feed-borrow' });
		reader.unreadOnly = true;
		await vi.waitFor(() => expect(screen.getByText('Nothing unread here.')).toBeInTheDocument());
		await userEvent.click(screen.getByRole('button', { name: 'Show all items' }));
		expect(reader.unreadOnly).toBe(false);
		expect(screen.getAllByRole('listitem').length).toBeGreaterThan(0);
	});

	it('says when the search finds nothing, and when a feed is new', async () => {
		const { reader } = setup();
		reader.query = 'nothing says this';
		await vi.waitFor(() =>
			expect(screen.getByText('No article matches the search.')).toBeInTheDocument()
		);
		reader.subscribe('blog.example');
		await vi.waitFor(() =>
			expect(screen.getByText('This feed has no articles yet.')).toBeInTheDocument()
		);
		reader.deleteRule('rule-1');
		reader.deleteRule('rule-2');
		reader.show({ kind: 'hidden' });
		await vi.waitFor(() => expect(screen.getByText('No articles here.')).toBeInTheDocument());
	});
});

describe('the expanded view', () => {
	it('shows every article in full, and a click picks one', async () => {
		settings.view = 'expanded';
		const { reader, row } = setup();
		const articles = screen.getAllByRole('article');
		expect(articles).toHaveLength(Math.min(40, reader.list.length));
		const second = reader.list[1];
		expect(within(row(second.id)).getByRole('link', { name: second.title })).toBeInTheDocument();
		await userEvent.click(within(row(second.id)).getByText(/^from/));
		expect(reader.cursor).toBe(second.id);
		expect(reader.isRead(second)).toBe(true);
		expect(row(second.id)).toHaveClass('border-accent');
	});

	it('goes to the feed of an article', async () => {
		settings.view = 'expanded';
		const { reader, row } = setup({ kind: 'all' });
		const article = reader.list[0];
		const feed = reader.feeds[article.feedId];
		await userEvent.click(within(row(article.id)).getByRole('button', { name: feed.title }));
		expect(reader.stream).toEqual({ kind: 'feed', id: feed.id });
	});
});
