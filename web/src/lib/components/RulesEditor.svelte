<script lang="ts">
	// The filter rules of the account: a list to switch them on and off, and a form to
	// add or change one, which says how many articles the rule would catch right now.
	import type { Reader } from '#lib/demo/reader.svelte.ts';
	import { compile, MAX_PATTERN_LENGTH, type FilterRule, type RuleScope } from '#lib/filters.ts';
	import { m } from '#lib/i18n/index.ts';
	import type { TreeNode } from '#lib/model.ts';
	import { ruleText, scopeText } from '#lib/rule-text.ts';
	import Icon from './Icon.svelte';

	let { reader }: { reader: Reader } = $props();

	const fields = ['any', 'title', 'content', 'author', 'tag'] as const;
	const matches = ['contains', 'word', 'regex'] as const;
	const actions = ['hide', 'read'] as const;

	/** The rule in the form, or null while the form is closed. */
	let draft: FilterRule | null = $state(null);

	/** The folders and feeds as options, indented by their depth: "folder:id" or "feed:id". */
	const scopes = $derived.by(() => {
		const options: { value: string; label: string }[] = [{ value: 'all', label: m.rules.scopeAll }];
		const walk = (nodes: readonly TreeNode[], depth: number) => {
			for (const node of nodes) {
				const title = node.kind === 'folder' ? node.title : (reader.feeds[node.id]?.title ?? '');
				options.push({
					value: `${node.kind}:${node.id}`,
					label: `${'  '.repeat(depth)}${node.kind === 'folder' ? '▸ ' : ''}${title}`
				});
				if (node.kind === 'folder') walk(node.children, depth + 1);
			}
		};
		walk(reader.tree, 0);
		return options;
	});

	const scopeValue = (scope: RuleScope) =>
		scope.kind === 'all' ? 'all' : `${scope.kind}:${scope.id}`;

	function setScope(value: string) {
		if (!draft) return;
		const [kind, id] = value.split(':');
		draft.scope = kind === 'folder' || kind === 'feed' ? { kind, id } : { kind: 'all' };
	}

	const valid = $derived(draft !== null && compile(draft) !== null);
	const hits = $derived(draft && valid ? reader.countMatches(draft) : 0);

	function add() {
		draft = {
			id: reader.newRuleId(),
			enabled: true,
			scope: { kind: 'all' },
			field: 'title',
			match: 'contains',
			pattern: '',
			action: 'hide'
		};
	}

	function save(event: SubmitEvent) {
		event.preventDefault();
		if (!draft || !valid) return;
		reader.saveRule({ ...draft, scope: { ...draft.scope }, pattern: draft.pattern.trim() });
		draft = null;
	}
</script>

<p class="text-sm text-soft">{m.rules.intro}</p>

<ul class="mt-3 divide-y divide-line rounded-md border border-line">
	{#each reader.rules as rule (rule.id)}
		<li class="flex items-center gap-3 px-3 py-2 text-sm">
			<label class="flex cursor-pointer items-center gap-1.5" title={m.rules.on}>
				<input
					type="checkbox"
					class="rounded text-accent"
					checked={rule.enabled}
					onchange={(event) => reader.saveRule({ ...rule, enabled: event.currentTarget.checked })}
				/>
				<span class="sr-only">{m.rules.on}: {ruleText(rule)}</span>
			</label>
			<div class={['min-w-0 flex-1', !rule.enabled && 'text-soft']}>
				<p class="truncate">{ruleText(rule)}</p>
				<p class="text-xs text-soft">
					{scopeText(reader, rule.scope)} · {m.rules.hits(reader.countMatches(rule))}
				</p>
			</div>
			<button
				type="button"
				class="plain px-2 py-1"
				onclick={() => (draft = { ...rule, scope: { ...rule.scope } })}
			>
				{m.common.edit}
			</button>
			<button
				type="button"
				class="plain size-7"
				aria-label="{m.common.delete}: {ruleText(rule)}"
				title={m.common.delete}
				onclick={() => reader.deleteRule(rule.id)}
			>
				<Icon name="close" />
			</button>
		</li>
	{:else}
		<li class="px-3 py-2 text-sm text-soft">{m.rules.none}</li>
	{/each}
</ul>

{#if draft}
	<form class="mt-4 rounded-md border border-line bg-panel p-3" onsubmit={save}>
		<h3 class="text-sm font-semibold">{m.rules.new}</h3>
		<div class="mt-2 grid gap-3 text-sm sm:grid-cols-2">
			<label class="block">
				<span class="font-medium">{m.rules.scope}</span>
				<select
					class="mt-1 block w-full rounded py-1 text-sm"
					value={scopeValue(draft.scope)}
					onchange={(event) => setScope(event.currentTarget.value)}
				>
					{#each scopes as scope (scope.value)}
						<option value={scope.value}>{scope.label}</option>
					{/each}
				</select>
			</label>
			<label class="block">
				<span class="font-medium">{m.rules.field}</span>
				<select class="mt-1 block w-full rounded py-1 text-sm" bind:value={draft.field}>
					{#each fields as field (field)}
						<option value={field}>{m.rules.fields[field]}</option>
					{/each}
				</select>
			</label>
			<label class="block">
				<span class="font-medium">{m.rules.match}</span>
				<select class="mt-1 block w-full rounded py-1 text-sm" bind:value={draft.match}>
					{#each matches as match (match)}
						<option value={match}>{m.rules.matches[match]}</option>
					{/each}
				</select>
			</label>
			<label class="block">
				<span class="font-medium">{m.rules.pattern}</span>
				<input
					class="mt-1 block w-full rounded py-1 text-sm"
					maxlength={MAX_PATTERN_LENGTH}
					bind:value={draft.pattern}
				/>
			</label>
			<label class="block sm:col-span-2">
				<span class="font-medium">{m.rules.action}</span>
				<select class="mt-1 block w-full rounded py-1 text-sm" bind:value={draft.action}>
					{#each actions as action (action)}
						<option value={action}>{m.rules.actions[action]}</option>
					{/each}
				</select>
			</label>
		</div>
		<p class="mt-3 text-sm" role="status">
			{#if valid}
				{m.rules.preview(hits, reader.countIn(draft.scope))}
			{:else if draft.pattern.trim() === ''}
				<span class="text-soft">{m.rules.empty}</span>
			{:else}
				<span class="text-warn">{m.rules.invalid}</span>
			{/if}
		</p>
		<div class="mt-3 flex gap-2">
			<button
				type="submit"
				class="cursor-pointer rounded bg-accent px-3 py-1 text-sm font-semibold text-on-accent hover:brightness-110 disabled:cursor-default disabled:opacity-50"
				disabled={!valid}
			>
				{m.common.save}
			</button>
			<button type="button" class="tool" onclick={() => (draft = null)}>{m.common.cancel}</button>
		</div>
	</form>
{:else}
	<button type="button" class="tool mt-3" onclick={add}>
		<Icon name="plus" />
		{m.rules.add}
	</button>
{/if}
