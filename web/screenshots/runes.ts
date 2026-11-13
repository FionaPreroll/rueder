// The page's language is kept in a rune of Svelte (src/lib/i18n/locale.svelte.ts).
// Outside of Svelte, as here where the screenshots read the texts of the page,
// a rune is the plain value it starts with.

Object.assign(globalThis, { $state: <T>(value: T) => value });
