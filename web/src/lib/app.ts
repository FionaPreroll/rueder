// The page inside rueder's Android app (app/ in the repository): the app adds
// "rueder-android" to the browser's name, and keeps its own start page, where the
// server and the app's updates are chosen, at this address.

/** The start page of the app, asked to stay rather than open the server. */
export const APP_SETTINGS = 'https://localhost/index.html?setup';

/** Whether the page runs inside the Android app. */
export const inAndroidApp = (
	userAgent = typeof navigator === 'undefined' ? '' : navigator.userAgent
) => /\brueder-android\b/.test(userAgent);
