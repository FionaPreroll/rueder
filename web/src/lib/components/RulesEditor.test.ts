import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { DemoReader } from '#lib/demo/reader.svelte.ts';
import { layerOf } from '#lib/layers.ts';
import RulesEditor from './RulesEditor.svelte';

const now = Date.parse('2026-11-10T14:30:00Z');
const setup = () => {
	const reader = new DemoReader(() => now);
	render(RulesEditor, { reader, book: reader });
	return reader;
};

describe('RulesEditor', () => {
	it('lists the rules in words, with where they apply and what they find', () => {
		const reader = setup();
		const rows = screen.getAllByRole('listitem');
		expect(rows).toHaveLength(3);
		expect(rows[0]).toHaveTextContent('Title has the word “Anzeige”: hide the article');
		expect(rows[0]).toHaveTextContent(`All feeds · ${reader.countMatches(reader.rules[0])} hits`);
		expect(rows[2]).toHaveTextContent('Folder: Nachrichten');
		expect(within(rows[2]).getByRole('checkbox')).not.toBeChecked();
	});

	it('switches a rule off and on, and the counts follow', async () => {
		const reader = setup();
		const hidden = reader.hiddenCount;
		const found = reader.countMatches(reader.rules[0]);
		const [first] = screen.getAllByRole('checkbox');
		await userEvent.click(first);
		expect(reader.rules[0].enabled).toBe(false);
		expect(reader.hiddenCount).toBe(hidden - found);
		await userEvent.click(first);
		expect(reader.hiddenCount).toBe(hidden);
	});

	it('adds a rule and says what it would catch while it is written', async () => {
		const reader = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		const save = screen.getByRole('button', { name: 'Save' });
		expect(save).toBeDisabled();
		expect(screen.getByRole('status')).toHaveTextContent('Enter the text to look for.');

		await userEvent.selectOptions(screen.getByLabelText('Look at'), 'author');
		await userEvent.type(screen.getByLabelText('Text to look for'), 'Mara Lindqvist');
		const total = Object.values(reader.articles).flat();
		const hers = total.filter((article) => article.author === 'Mara Lindqvist').length;
		expect(hers).toBeGreaterThan(0);
		expect(screen.getByRole('status')).toHaveTextContent(
			`Applies to ${hers} of ${total.length} articles here now.`
		);

		// Narrowed to a feed she does not write for, it finds nothing.
		await userEvent.selectOptions(screen.getByLabelText('In'), 'feed:feed-tagesbote');
		expect(screen.getByRole('status')).toHaveTextContent(
			`Applies to 0 of ${reader.articles['feed-tagesbote'].length} articles here now.`
		);
		await userEvent.selectOptions(screen.getByLabelText('In'), 'folder:folder-rust');
		await userEvent.click(save);

		expect(reader.rules).toHaveLength(4);
		expect(reader.rules[3]).toMatchObject({
			enabled: true,
			scope: { kind: 'folder', id: 'folder-rust' },
			field: 'author',
			match: 'contains',
			pattern: 'Mara Lindqvist',
			action: 'hide'
		});
		expect(screen.getAllByRole('listitem')[3]).toHaveTextContent('Folder: Rust');
		expect(screen.queryByRole('status')).toBeNull();
	});

	it('does not save a broken regular expression', async () => {
		const reader = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		await userEvent.selectOptions(screen.getByLabelText('Match'), 'regex');
		await userEvent.type(screen.getByLabelText('Text to look for'), '(Hafen');
		expect(screen.getByRole('status')).toHaveTextContent('This is not a valid regular expression.');
		expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
		expect(reader.rules).toHaveLength(3);
		expect(screen.getByRole('button', { name: 'Add a rule' })).toBeInTheDocument();
	});

	it('points to a builder for regular expressions, with the pattern so far', async () => {
		setup();
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		expect(screen.queryByRole('link', { name: /regex101/ })).toBeNull();
		await userEvent.selectOptions(screen.getByLabelText('Match'), 'regex');
		await userEvent.type(screen.getByLabelText('Text to look for'), '^Hafen');
		const link = screen.getByRole('link', { name: 'Build and try one out on regex101' });
		expect(link).toHaveAttribute('target', '_blank');
		expect(new URL(link.getAttribute('href') ?? '').searchParams.get('regex')).toBe('^Hafen');
	});

	it('adds a rule for all articles, which needs no text', async () => {
		const reader = setup();
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		await userEvent.type(screen.getByLabelText('Text to look for'), 'left over');
		await userEvent.selectOptions(screen.getByLabelText('In'), 'feed:feed-tagesbote');
		await userEvent.selectOptions(screen.getByLabelText('Match'), 'all');
		expect(screen.getByLabelText('Text to look for')).toBeDisabled();
		expect(screen.getByLabelText('Look at')).toBeDisabled();
		const all = reader.articles['feed-tagesbote'].length;
		expect(screen.getByRole('status')).toHaveTextContent(
			`Applies to ${all} of ${all} articles here now.`
		);
		await userEvent.selectOptions(screen.getByLabelText('Then'), 'read');
		await userEvent.click(screen.getByRole('button', { name: 'Save' }));
		expect(reader.rules[3]).toMatchObject({ field: 'any', match: 'all', pattern: '' });
		expect(screen.getAllByRole('listitem')[3]).toHaveTextContent('Every article: mark it as read');
	});

	it('edits and deletes a rule', async () => {
		const reader = setup();
		await userEvent.click(screen.getAllByRole('button', { name: 'Edit' })[1]);
		const pattern = screen.getByLabelText('Text to look for');
		expect(pattern).toHaveValue('Sponsored Desk');
		expect(screen.getByLabelText('Look at')).toHaveValue('author');
		await userEvent.clear(pattern);
		await userEvent.type(pattern, 'Werbeteam');
		await userEvent.selectOptions(screen.getByLabelText('Then'), 'read');
		await userEvent.click(screen.getByRole('button', { name: 'Save' }));
		expect(reader.rules[1]).toMatchObject({ id: 'rule-2', pattern: 'Werbeteam', action: 'read' });
		expect(reader.rules).toHaveLength(3);

		await userEvent.click(screen.getAllByRole('button', { name: /^Delete: / })[0]);
		expect(reader.rules.map((rule) => rule.id)).toEqual(['rule-2', 'rule-3']);
		await userEvent.click(screen.getAllByRole('button', { name: /^Delete: / })[0]);
		await userEvent.click(screen.getAllByRole('button', { name: /^Delete: / })[0]);
		expect(screen.getByText('No rules yet.')).toBeInTheDocument();
		expect(reader.hiddenCount).toBe(0);
	});

	it('brings the form into sight, names what it does, and closes it on back', async () => {
		const into = vi.spyOn(HTMLElement.prototype, 'scrollIntoView').mockImplementation(() => {});
		setup();
		await userEvent.click(screen.getAllByRole('button', { name: 'Edit' })[0]);
		expect(screen.getByRole('heading', { name: 'Change the rule' })).toBeInTheDocument();
		expect(into.mock.contexts.map((element) => (element as Element).tagName)).toEqual(['FORM']);
		expect(layerOf(history.state)).toBe(1);

		history.back();
		await waitFor(() => expect(screen.queryByRole('heading', { level: 3 })).toBeNull());
		await userEvent.click(screen.getByRole('button', { name: 'Add a rule' }));
		expect(screen.getByRole('heading', { name: 'New rule' })).toBeInTheDocument();
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
	});
});
