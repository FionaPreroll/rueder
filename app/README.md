# rueder for Android

The Android app: [Capacitor](https://capacitorjs.com/) around a small start page
that asks for the address of your rueder and then opens it. The page of rueder
itself comes from your server, as in a browser, so the server, its session cookie
and its Content-Security-Policy stay as they are.

## How it works

- **Start page** (`www/`): at the first start it asks for the address of your
  rueder (https only, as rueder logs in only over a secure connection) and whether
  the app may look for new versions of itself. Later starts open the server at once.
  A long press on the app's icon (*Server and updates*), or *Server and app updates*
  in the account menu of rueder's page, comes back here. If the server does not
  answer, the start page says so.
- **Native side** (`android/app/src/main/java/…`): `MainActivity` with the WebView,
  `ServerClient`, which keeps the server's pages in the app and opens every other
  address in the browser, and `RuederPlugin`, which answers only the start page: the
  server's page runs in the same WebView and must not change where the app goes.
- **Updates**, for the build from GitHub only: if you agreed, the app reads
  `version.json` from the release
  [`android-latest`](https://github.com/FionaPreroll/rueder/releases/tag/android-latest)
  at every start (`Updates.java` in `src/github/`). A newer build is offered in a
  dialog that says its size and whether the network may cost money; *Download*
  hands it to Android's download manager and then to Android's installer, which asks
  you. Android installs it only if it is signed with the same key as the app you
  have. Nothing else is sent anywhere.
- **Two builds** (product flavors in `android/app/build.gradle`): `github`, released
  by CI, with the update check; `play`, for the Play Store, which brings updates
  itself, without the update check and without the permission to install apps
  (`src/play/`).

## Building

You need Node 22 with pnpm, Java 21 and the Android SDK (platform 36). Then:

```sh
pnpm install
pnpm test                  # the start page's own logic
pnpm run sync              # copies www/ into the Android project
cd android
./gradlew testGithubDebugUnitTest assembleGithubDebug assemblePlayDebug
```

The debug APKs are in `android/app/build/outputs/apk/`.

## Releases and signing

CI (`app` in `.github/workflows/ci.yml`) builds both on every push and pull
request. On `main`, once the checks have passed, `app-release` replaces the release
`android-latest` with `rueder.apk` and a `version.json` of the build number (CI's
run number, also the APK's `versionCode`), the commit and the size.

The release is signed with a key that is never in the repository. CI takes it from
the repository's secrets: `ANDROID_KEYSTORE_BASE64` (the keystore, base64-encoded),
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD`. Without
them CI builds a debug APK and releases nothing. To make a key once:

```sh
keytool -genkeypair -v -keystore rueder.keystore -alias rueder \
  -keyalg RSA -keysize 4096 -validity 36500
base64 -w0 rueder.keystore   # the value of ANDROID_KEYSTORE_BASE64
```

Keep the keystore and its passwords safe and out of the repository: every update
must be signed with the same key, or Android refuses to install it over the app.
A release on the Play Store is signed by Google's Play App Signing and needs its
own upload key.

`RUEDER_UPDATES_URL` names another release (for a fork) at build time.
