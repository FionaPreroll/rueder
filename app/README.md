# rueder for Android

The Android app: [Capacitor](https://capacitorjs.com/) around a small start page
that asks for the address of your rueder and then opens it. The page of rueder
itself comes from your server, as in a browser, so the server, its session cookie
and its Content-Security-Policy stay as they are.

## How it works

- **Start page** (`www/`): at the first start it asks for the address of your
  rueder (https only, as rueder logs in only over a secure connection) and whether
  the app may look for new versions of itself. Later starts open the server at once.
  A long press on the app's icon (_Server and updates_), or _Server and app updates_
  in the account menu of rueder's page, comes back here. If the server does not
  answer, the start page says so.
- **Native side** (`android/app/src/main/java/…`): `MainActivity` with the WebView,
  `ServerClient`, which keeps the server's pages in the app and opens every other
  address in the browser, and `RuederPlugin`, which answers only the start page: the
  server's page runs in the same WebView and must not change where the app goes.
  It also asks the saved server where its operator's imprint, privacy policy and
  terms of use are, which the start page links to (`www/legal.js`).
- **Updates**, for the build from GitHub only: if you agreed, the app reads
  `version.json` from the release
  [`android-latest`](https://github.com/FionaPreroll/rueder/releases/tag/android-latest)
  at every start (`Updates.java` in `src/github/`). A newer build is offered in a
  dialog that says its size and whether the network may cost money; _Download_
  hands it to Android's download manager. The app copies the download into its
  own cache, where no other app can change it, and hands it to Android's installer
  only if it is this app in the build `version.json` names; the installer asks
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
request: a debug APK of `github`, which installs but cannot update a release, an
unsigned release of `github` and an unsigned release of `play`. That job never
sees the signing key, because it runs the build scripts of every pull request
before anybody merged them.

On `main`, once the checks have passed and if the app changed, `app-release`
signs the unsigned release with `apksigner` from the Android SDK on the runner,
without running any of the repository's build scripts, and replaces the release
`android-latest` with `rueder.apk` and a `version.json` of the build number (CI's
run number, also the APK's `versionCode`), the commit and the size. It also
attests the APK, so that anybody can check where it comes from:

```sh
gh attestation verify rueder.apk --repo FionaPreroll/rueder
```

The app changed if anything in `app/` differs from the commit the tag
`android-latest` points at, apart from this README, the tests in `test/` and
`android/app/src/test/` and Prettier's settings. The job `app-changed` checks
that; a push that changes nothing else in the app, e.g. only the server or the
web page, makes no new release, so installed apps are not offered an update
that brings nothing new, and the environment below does not ask for an
approval. A change only to how CI builds or signs the app counts as none; it
reaches the release with the next change in `app/`.

The key is never in the repository. `app-release` takes it from four secrets:
`ANDROID_KEYSTORE_BASE64` (the keystore, base64-encoded),
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD`.
Without them it releases nothing. To make a key once:

```sh
keytool -genkeypair -v -keystore rueder.keystore -alias rueder \
  -keyalg RSA -keysize 4096 -validity 36500
base64 -w0 rueder.keystore   # the value of ANDROID_KEYSTORE_BASE64
```

Keep the keystore and its passwords safe and out of the repository: every update
must be signed with the same key, or Android refuses to install it over the app.
A release on the Play Store is signed by Google's Play App Signing and needs its
own upload key.

### The environment `android-release`

`app-release` runs in the GitHub environment `android-release`. GitHub makes it
the first time the job runs, without any protection; until the secrets move
there, the job reads them from the repository's secrets as before. To make the
key reachable from that one job only, and only after you agreed:

1. **Settings → Environments → `android-release`** (or **New environment** with
   that name, if CI has not run since this change).
2. **Deployment protection rules:** tick **Required reviewers** and add yourself.
   Leave **Prevent self-review** off if you are the only one, or you cannot
   approve your own merges. Each push to `main` that changes the app then waits
   with the app's release until you choose **Review deployments → Approve** in the run; the image and
   everything else go on without it.
3. **Deployment branches and tags:** choose **Selected branches and tags** and
   add `main`, so no other branch can run a job in this environment.
4. **Environment secrets:** add the four secrets above with the same names and
   values.
5. **Settings → Secrets and variables → Actions:** delete the four repository
   secrets of the same names. Only then is the key out of reach of every other
   job and workflow.

The secrets cannot be read back from GitHub: keep the keystore and its passwords
where you keep them now, to enter them again in step 4.

`RUEDER_UPDATES_URL` names another release (for a fork) at build time.
