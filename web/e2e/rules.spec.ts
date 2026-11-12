// Filter rules: the server says what a rule would catch, and hides it.

import { expect, test } from '@playwright/test';
import { article, articles, logIn, node, sidebar, subscribe } from './harness/helpers.ts';

test.describe.configure({ mode: 'serial' });

const openRules = async (page: import('@playwright/test').Page) => {
	await page.getByRole('button', { name: 'Settings' }).click();
	const dialog = page.getByRole('dialog', { name: 'Settings' });
	await dialog.getByRole('tab', { name: 'Filter rules' }).click();
	return dialog;
};

test('writes a rule with the server saying what it would catch, and the articles go', async ({
	page
}) => {
	await logIn(page, 'rules');
	await subscribe(page, 'market-rules', 'Market Courier');
	await expect(node(page, 'Market Courier')).toContainText('(4)');

	const dialog = await openRules(page);
	await expect(dialog.getByText('No rules yet.')).toBeVisible();
	await dialog.getByRole('button', { name: 'Add a rule' }).click();
	const save = dialog.getByRole('button', { name: 'Save' });
	await expect(save).toBeDisabled();

	// The server's regular expressions decide what a pattern is.
	await dialog.getByLabel('Match').selectOption('regex');
	await dialog.getByLabel('Text to look for').fill('^(Sponsored');
	await expect(dialog.getByRole('status')).toHaveText('This is not a valid regular expression.');
	await expect(save).toBeDisabled();
	await dialog.getByLabel('Text to look for').fill('^Sponsored:');
	await expect(dialog.getByRole('status')).toHaveText('Applies to 2 of 4 articles here now.');
	await save.click();
	await expect(
		dialog.getByText('Title matches the regular expression “^Sponsored:”: hide the article').first()
	).toBeVisible();
	await dialog.getByRole('button', { name: 'Close' }).click();

	// What it hides is neither unread nor in the list.
	await expect(node(page, 'Market Courier')).toContainText('(2)');
	await node(page, 'Market Courier').click();
	await expect(articles(page)).toHaveCount(2);
	await expect(article(page, 'Sponsored')).toHaveCount(0);
});

test('keeps what a rule hid under "Hidden by filters", until the rule goes', async ({ page }) => {
	await logIn(page, 'rules');
	// The sidebar counts them.
	await expect(sidebar(page).getByRole('button', { name: /Hidden by filters/ })).toContainText('2');
	await sidebar(page)
		.getByRole('button', { name: /Hidden by filters/ })
		.click();
	await expect(articles(page)).toHaveCount(2);
	await article(page, 'Sponsored: a week of offers')
		.getByRole('button', { name: /Sponsored: a week of offers/ })
		.click();
	await expect(
		page.getByText('Hidden by the rule: Title matches the regular expression')
	).toBeVisible();

	const dialog = await openRules(page);
	await dialog.getByRole('button', { name: /^Delete: Title matches/ }).click();
	await expect(dialog.getByText('No rules yet.')).toBeVisible();
	await dialog.getByRole('button', { name: 'Close' }).click();
	await expect(articles(page)).toHaveCount(0);
	await node(page, 'Market Courier').click();
	await expect(articles(page)).toHaveCount(4);
});
