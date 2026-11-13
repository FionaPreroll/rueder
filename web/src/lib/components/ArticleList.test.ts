import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { labelColors, type Stream } from '#lib/model.ts';
import { defaults, settings } from '#lib/settings.svelte.ts';
import ArticleBody from './ArticleBody.svelte';
import ArticleList from './ArticleList.svelte';
import ArticleRow from './ArticleRow.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');

function setup(stream: Stream = { kind: 'feed', id: 'feed-tagesbote' }) {
	const reader = new DemoReader(() => now);
	reader.show(stream);
	const onversions = vi.fn();
	const { container } = render(ArticleList, { reader, now, onversions });
	const row = (id: string) => container.querySelector<HTMLElement>(`[data-article="${id}"]`)!;
	const scroller = container.querySelector<HTMLElement>('.overflow-y-auto')!;
	return { reader, onversions, row, container, scroller };
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
		await reader.subscribe('blog.example', null);
		await vi.waitFor(() =>
			expect(screen.getByText('This feed has no articles yet.')).toBeInTheDocument()
		);
		reader.deleteRule('rule-1');
		reader.deleteRule('rule-2');
		reader.show({ kind: 'hidden' });
		await vi.waitFor(() => expect(screen.getByText('No articles here.')).toBeInTheDocument());
	});

	it('says when the operator blocked the feed', async () => {
		const { reader } = setup({ kind: 'feed', id: 'feed-borrow' });
		reader.feeds = {
			...reader.feeds,
			'feed-borrow': { ...reader.feeds['feed-borrow']!, blocked: true }
		};
		reader.query = 'nothing says this';
		await vi.waitFor(() =>
			expect(screen.getByRole('status')).toHaveTextContent('The operator of this rueder blocked')
		);
	});
});

describe('the stacked rows beside the article', () => {
	function stacked(author: string) {
		const reader = new DemoReader(() => now);
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		const article = { ...reader.list[0], author };
		const { container } = render(ArticleRow, {
			reader,
			article,
			now,
			showFeed: false,
			stacked: true
		});
		const title = within(container).getByText(article.title);
		const time = container.querySelector('time')!;
		return { title, time };
	}

	it('puts the author and the time over the title', () => {
		const { title, time } = stacked('Ruth Vogel');
		expect(time.parentElement).toHaveTextContent('Ruth Vogel');
		expect(time.parentElement).not.toContainElement(title);
	});

	it('puts the time beside the title when there is no author, leaving no empty line', () => {
		for (const author of ['', '  ']) {
			const { title, time } = stacked(author);
			expect(time.parentElement).toContainElement(title);
			document.body.innerHTML = '';
		}
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

	it('brings a row that opens or closes above the view back into it', async () => {
		const { reader, row } = setup();
		const [long, next] = reader.list;
		// The row that matters is above the view, as after reading a long article.
		let above = '';
		vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (
			this: HTMLElement
		) {
			return { top: this.dataset.article === above ? -800 : 0 } as DOMRect;
		});
		const scrolled: (string | undefined)[] = [];
		const into = vi.spyOn(HTMLElement.prototype, 'scrollIntoView').mockImplementation(function (
			this: HTMLElement
		) {
			scrolled.push(this.dataset.article);
		});
		const open = (id: string) =>
			userEvent.click(within(row(id)).getByRole('button', { expanded: reader.open === id }));

		await open(long.id);
		expect(scrolled).toEqual([]);

		// Opening the next one closes the long one, which takes the next row up out of sight.
		above = next.id;
		await open(next.id);
		await new Promise((done) => setTimeout(done, 0));
		expect(reader.open).toBe(next.id);
		expect(scrolled).toEqual([next.id]);
		expect(into).toHaveBeenLastCalledWith({ block: 'start' });

		// Closing an article whose row scrolled out at the top goes back to that row.
		await open(next.id);
		await new Promise((done) => setTimeout(done, 0));
		expect(reader.open).toBeNull();
		expect(scrolled).toEqual([next.id, next.id]);
		vi.restoreAllMocks();
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

describe('a long list', () => {
	/** Lays the page out as a browser would: a view of 600 pixels, rows of 50. */
	function laidOut() {
		const box = (name: 'clientHeight' | 'offsetHeight', value: (element: HTMLElement) => number) =>
			vi.spyOn(HTMLElement.prototype, name, 'get').mockImplementation(function (this: HTMLElement) {
				return value(this);
			});
		box('clientHeight', () => 600);
		box('offsetHeight', (element) =>
			element.dataset.article || element.tagName === 'DIV' ? 50 : 0
		);
	}

	afterEach(() => vi.restoreAllMocks());

	async function longList() {
		laidOut();
		const view = setup({ kind: 'all' });
		for (let i = 0; i < 3; i++) {
			await userEvent.click(screen.getByRole('button', { name: 'Show more' }));
		}
		await new Promise((done) => requestAnimationFrame(done));
		return view;
	}

	it('draws only the rows near the view, and the rest as space', async () => {
		const { reader, container } = await longList();
		expect(reader.visible.length).toBe(160);
		const drawn = screen.getAllByRole('listitem');
		// The view of 600 pixels and 1200 more below it: 36 rows of 50, and one more.
		expect(drawn.length).toBeLessThan(60);
		expect(drawn[0]).toHaveAttribute('data-article', reader.visible[0].id);
		const space = [...container.querySelectorAll<HTMLElement>('div[style]')].map(
			(div) => div.style.height
		);
		expect(space).toContain(`${(160 - drawn.length) * 50}px`);
	});

	it('scrolls to an article the keyboard steps to, though it was not drawn', async () => {
		const { reader, row, scroller } = await longList();
		const target = reader.visible[120];
		reader.select(reader.visible[119], false);
		reader.step(1);
		await new Promise((done) => setTimeout(done, 0));
		expect(scroller.scrollTop).toBeGreaterThan(0);
		await new Promise((done) => requestAnimationFrame(done));
		expect(row(target.id)).not.toBeNull();
		expect(row(reader.visible[0].id)).toBeNull();
	});

	it('offers a way back to the top once the reader is a screen down', async () => {
		const { scroller } = await longList();
		const toTop = () => screen.queryByRole('button', { name: 'Back to the top' });
		expect(toTop()).toBeNull();

		scroller.scrollTop = 1500;
		scroller.dispatchEvent(new Event('scroll'));
		await new Promise((done) => requestAnimationFrame(done));
		const scrollTo = vi.fn();
		scroller.scrollTo = scrollTo;
		await userEvent.click(toTop()!);
		expect(scrollTo).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' });

		scroller.scrollTop = 0;
		scroller.dispatchEvent(new Event('scroll'));
		await new Promise((done) => requestAnimationFrame(done));
		expect(toTop()).toBeNull();
	});
});

describe('an open article', () => {
	it('is reported from its menu where the server takes reports', async () => {
		const reader = new DemoReader(() => now);
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		const article = reader.list[0]!;
		const { unmount } = render(ArticleBody, { reader, article, now, onversions: vi.fn() });
		expect(screen.queryByRole('button', { name: 'More actions for the article' })).toBeNull();
		unmount();

		Object.assign(reader.can, { reportContent: true });
		const reportContent = vi.spyOn(reader, 'reportContent').mockResolvedValue('sent');
		render(ArticleBody, { reader, article, now, onversions: vi.fn() });
		await userEvent.click(screen.getByRole('button', { name: 'More actions for the article' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'Report the article …' }));
		const dialog = screen.getByRole('dialog', { name: 'Report the article' });
		await userEvent.type(within(dialog).getByRole('textbox'), 'Ads');
		await userEvent.click(within(dialog).getByRole('button', { name: 'Send the report' }));
		expect(reportContent).toHaveBeenCalledWith(
			{ feedId: article.feedId, seq: article.seq },
			'spam',
			'Ads'
		);
	});

	it('offers labels when the account has none yet, and makes the first on the spot', async () => {
		const reader = new DemoReader(() => now);
		for (const label of [...reader.labels]) reader.removeLabel(label.id);
		reader.show({ kind: 'feed', id: 'feed-tagesbote' });
		const article = reader.list[0];
		render(ArticleBody, { reader, article, now, onversions: vi.fn() });
		await userEvent.click(screen.getByRole('button', { name: 'Labels' }));
		expect(screen.queryAllByRole('menuitemcheckbox')).toEqual([]);
		await userEvent.click(screen.getByRole('menuitem', { name: 'New label' }));
		// Escape lets it be.
		await userEvent.keyboard('{Escape}');
		expect(screen.queryByLabelText('Name of the label')).toBeNull();
		expect(reader.labels).toEqual([]);

		await userEvent.click(screen.getByRole('button', { name: 'Labels' }));
		await userEvent.click(screen.getByRole('menuitem', { name: 'New label' }));
		await userEvent.type(screen.getByLabelText('Name of the label'), '  Erstes  {Enter}');
		expect(reader.labels).toMatchObject([{ title: 'Erstes', color: labelColors[0] }]);
		expect(reader.labelsOf(article)).toEqual([reader.labels[0].id]);
		expect(screen.getByLabelText('Label: Erstes')).toBeInTheDocument();
	});
});
