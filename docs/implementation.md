# rueder: Umsetzung und Übergabe

Dieses Dokument sagt, was fertig ist, wie gearbeitet wird und welche Arbeitspakete offen sind, so dass jemand Neues die Arbeit übernehmen kann. Den Plan mit Anforderungen und Entscheidungen führt [plan.md](plan.md), den Aufbau des neuen Systems [architecture.md](architecture.md).

Wer ein Arbeitspaket abschließt, hakt es hier ab und trägt den Stand nach, im selben Pull Request.

- [Womit anfangen](#womit-anfangen)
- [Stand](#stand)
- [Repository](#repository)
- [Stack und Konventionen](#stack-und-konventionen)
- [Einrichten und Befehle](#einrichten-und-befehle)
- [CI](#ci)
- [Arbeitsweise](#arbeitsweise)
- [Arbeitspakete](#arbeitspakete)
- [Import der alten Daten](#import-der-alten-daten)
- [Messung](#messung)
- [Wissen über das Altsystem](#wissen-über-das-altsystem)

## Womit anfangen

1. [plan.md](plan.md) lesen: Anforderungen, Entscheidungen, die fünf Schritte.
2. [architecture.md](architecture.md) lesen: Datenmodell, Lesestand, Security.
3. Die Oberfläche mit erfundenen Daten starten und durchklicken: `cd web && pnpm install && pnpm dev:demo`. Mit einem Server dahinter ist es `pnpm dev`, siehe [Einrichten und Befehle](#einrichten-und-befehle).
4. Unter [Arbeitspakete](#arbeitspakete) das nächste offene Paket nehmen.

Fachliche Entscheidungen trifft Fiona. Die [Detailfragen](plan.md#detailfragen) sind nach den Vorschlägen entschieden.

## Stand

| Schritt | Stand |
|---|---|
| 1 Fundament und Daten | Fertig |
| 2 Backend | Fertig |
| 3 Frontend | Fertig |
| 4 Umstieg | Der neue rueder läuft in der Produktion, mit neuen Konten und Feeds; der alte Code ist entfernt. Offen: 4.5 (Import in der Produktion), 4.6 (Lesestand und Labels aus dem Browser) |
| 5 Öffentliche Instanz | Offen: 5.11 (Betrieb der öffentlichen Instanz) |

Was es gibt:

- `server/`: das Rust-Programm, ein Crate mit Unterbefehlen neben PostgreSQL. Es ruft die Feeds ab, bereinigt ihre Texte, liefert Bilder über den eigenen Proxy und bedient die API unter `/api`, die Live-Updates, die Google-Reader-API unter `/greader` und `/metrics`. Auf der Kommandozeile verwaltet es Konten, Registrierung, Aufbewahrung und den Import der alten Datenbank. Was es im Einzelnen kann, sagt [architecture.md](architecture.md); die Beschreibung der Schnittstelle steht in `server/openapi.json`. CI-Jobs `server` und `dependencies`.
- `web/`: die Oberfläche. Sie spricht mit dem Server und läuft mit erfundenen Daten für Tests, `pnpm dev:demo` und die Ein-Datei-Fassung. Sie lässt sich als App installieren (PWA). CI-Jobs `web` und `e2e` (Tests im Browser gegen den echten Server).
- `app/`: die Android-App mit Capacitor, in den Builds `github` und `play` ([app/README.md](../app/README.md)). CI-Jobs `app` und `app-release`.
- `pages/`: die Seite auf GitHub Pages unter `https://fionapreroll.github.io/rueder/`, auf Englisch, gebaut aus dem, was die anderen Teile bauen ([pages/README.md](../pages/README.md)). Workflow `pages.yml`.
- `Dockerfile` und `deploy/`: das Container-Image, die Compose-Datei mit PostgreSQL und die [Anleitung zur Installation](../deploy/README.md). CI-Job `image`.
- `docs/`: Plan, Architektur, dieses Dokument und das [Review des Designs](design_review.md).
- Das alte System (`backend/`, `frontend/`, `config/`) ist entfernt. Zum Lesen liegt es im Commit `0b36a96`, dem letzten, in dem es noch lief; wo darin was steht, sagt [Wissen über das Altsystem](#wissen-über-das-altsystem).

## Repository

| Pfad | Was |
|---|---|
| `server/` | Rust: HTTP-Server mit `axum`, `sqlx` und PostgreSQL. Dasselbe Programm hat die Kommandozeile (Konten, Admins, Registrierung, Umzug, Import, erneutes Bereinigen, Messdaten, `health`). Siehe [server/README.md](../server/README.md). |
| `server/src/` | `articles.rs` Artikelspeicher und Listen, `reading.rs` Lesestand, `http_client.rs` der gehärtete HTTP-Client, `feeds.rs` und `fetch.rs` Abruf, `cleaning.rs` Bereinigung, `reclean.rs` erneutes Bereinigen des Bestands, `images.rs` Bild-Proxy, `accounts.rs` Konten und Passwörter, `sessions.rs` Sitzungen und der Extraktor für eingeloggte Anfragen, `login.rs` Login, Logout und Rate-Limit auf falsche Passwörter, `hardening.rs` Header jeder Antwort und Prüfung der Herkunft ändernder Anfragen, `rate.rs` Rate-Limits je Adresse und je Konto, `events.rs` Live-Updates, `typescript.rs` die Typen der API für das Frontend, `api.rs` die API zum Lesen (Baum, Listen, Artikel, Versionen, Lesestand), `tree.rs` Änderungen am Baum und Einstellungen, `labels.rs` Labels, `filters.rs` Filterregeln, `subscribe.rs` Feed-Suche, Abonnieren und OPML, `profile.rs` eigene Sitzungen, Passwort und Konten für Admins, `oidc.rs` Login über OpenID Connect, `transfer.rs` Umzug eines Kontos als JSON-Datei (Export und Import), `health.rs` der Unterbefehl `health` für den Healthcheck des Containers, `import.rs` Import der alten Datenbank, `search.rs` die Suche, `registration.rs` Registrierung und Einladungen, `monitor.rs` was der Server über sich festhält, `retention.rs` wie lange alte Artikel bleiben, `status.rs` der Status für Admins, `metrics.rs` dieselben Zahlen für Prometheus, `reports.rs` Fehlerberichte aus den Browsern, `second_factor.rs` zweiter Faktor mit TOTP und Wiederherstellungscodes, `passkeys.rs` Passkeys, `asset_links.rs` `/.well-known/assetlinks.json` für Passkeys in der Android-App, `app_passwords.rs` App-Passwörter, `greader/` die Google-Reader-API für Apps anderer Anbieter, `email.rs` die E-Mail-Adresse eines Kontos, `mail.rs` Versand über SMTP, `recovery.rs` Passwort vergessen, `legal.rs` Rechtstexte und Zustimmung, `moderation.rs` Meldungen und gesperrte Feeds, `openapi.rs` die Beschreibung der Schnittstelle, `version.rs` was dieser Build ist, `bench.rs` Messdaten und Messung, `progress.rs` wie weit Migrationen, Import und erneutes Bereinigen sind, `lib.rs` Router und Fehlertyp, `main.rs` Kommandozeile. |
| `server/rules/` | Die Listen der Bereinigung als Daten: gesperrte Adressen und Tracking-Parameter, eine Regel je Zeile. |
| `server/migrations/` | SQL-Migrationen, laufen beim Start von selbst. |
| `server/deny.toml`, `server/.cargo/audit.toml` | Was `cargo deny` den Abhängigkeiten erlaubt (Lizenzen, Quellen, gesperrte Crates), und die Ausnahmen von `cargo audit` mit Begründung. |
| `server/openapi.json` | Die Beschreibung der Schnittstelle (OpenAPI 3.1), erzeugt und nicht von Hand zu ändern: `UPDATE_API_TYPES=1 cargo test --test api_types` schreibt sie mit `types.ts` neu. |
| `server/.sqlx/` | Cache der geprüften Abfragen, damit das Programm ohne Datenbank baut (`cargo sqlx prepare`). |
| `server/tests/` | Integrationstests gegen PostgreSQL. `tests/fixtures/legacy_schema.sql` ist das Schema des alten rueder, nachgebaut aus `backend/migrations/` im Commit `0b36a96`. `tests/data/` hält Feeds, wie die Tests des alten rueder sie aufgezeichnet haben. `fuzz_corpus.rs` spielt den Korpus der Fuzz-Targets ab, `soak.rs` ist der Soak-Test. |
| `server/fuzz/` | Fuzz-Targets für Bereinigung, Feed-Verarbeitung, Bild-Proxy und die Anfragen der Google-Reader-API (`fuzz_targets/`), was sie prüfen (`checks.rs`) und ihr Korpus (`corpus/`). Eine eigene Crate, weil `cargo-fuzz` Nightly-Rust braucht. |
| `web/` | SvelteKit, TypeScript und TailwindCSS als statische SPA. Siehe [web/README.md](../web/README.md). |
| `web/src/lib/` | Modelle (Lesestand, Regeln, Baum), Komponenten, Texte (`i18n/`). `reader.ts` ist die Naht: was die Seite von ihrer Datenquelle braucht. `api/` erfüllt sie mit dem Server (`types.ts` die Typen der API, erzeugt und nicht von Hand zu ändern, `client.ts` die Endpunkte, `reader.svelte.ts` der Reader, `session.svelte.ts` Login und Sitzung), `demo/` mit erfundenen Daten. |
| `web/e2e/` | End-to-End-Tests mit Playwright: die Specs je Weg, in `harness/` der Start des Servers mit eigener Datenbank (`server.js`), die erfundene Website mit ihren Feeds (`site.js`) und gemeinsame Helfer. |
| `app/` | Die Android-App mit Capacitor: Startseite in `www/`, das Android-Projekt in `android/` mit den Builds `github` und `play`, beschrieben in [app/README.md](../app/README.md). |
| `pages/` | Die Seite auf GitHub Pages: `build.js` setzt sie aus dem zusammen, was die anderen Teile bauen, und prüft ihre Links (`links.js`), `markdown.js` macht Seiten aus READMEs, `footer.js` schreibt Commit und Tag an jede Seite, `javadoc.gradle` gibt der App die Aufgabe `javadoc`, `src/` hält Übersicht, Vorlagen und Stylesheet. Beschrieben in [pages/README.md](../pages/README.md). |
| `Dockerfile`, `.dockerignore` | Das Container-Image mit Server und gebautem Frontend. |
| `deploy/` | Betrieb: `compose.yaml` startet rueder und PostgreSQL, `.env.example` nennt die Einstellungen, `README.md` ist die Anleitung zur Installation, `smoke-test.sh` baut und startet alles zur Probe. |
| `docs/` | Diese Dokumente. `design_review.md` ist das Review des Designs: Maßstab, wie man es wiederholt, Messwerte und Funde. `user_data.md` gehört zum alten rueder: Es beschreibt die Daten des alten Frontends im Browser, die Vorlage für Paket 4.6, nicht die des neuen. Was das neue im Browser hält, steht unter [Frontend](architecture.md#frontend). |
| `SECURITY.md` | Wie man eine Lücke meldet: privat über „Report a vulnerability“ auf GitHub, was dazugehört und dass nur der neueste Stand von `main` Korrekturen bekommt. Die Meldungen kommen nur an, wenn in den Einstellungen des Repositorys unter Security „Private vulnerability reporting“ eingeschaltet ist. |
| `.github/workflows/ci.yml` | CI für den neuen Code, veröffentlicht das Image und das APK. Daneben `pages.yml` für die Seite auf GitHub Pages, `codeql-analysis.yml`, `gitleaks.yml`, `fuzz.yml`, `soak.yml` und `screenshots.yml` (siehe [CI](#ci)). |
| `.github/dependabot.yml` | Updates der Abhängigkeiten von `server/`, `server/fuzz/`, `web/`, `app/` (npm und Gradle), `pages/` und der Actions einmal im Monat, der Basis-Images jede Woche, jeweils erst eine Woche nach dem Erscheinen einer Version. |


## Stack und Konventionen

### Server (Rust)

- Rust stable, Edition 2024. Ein Crate, ein Programm mit Unterbefehlen (`clap`), `serve` ist der Standard.
- `axum` auf `tokio`, `tower-http` für statische Dateien und Tracing, `tracing` für Logs.
- `sqlx` mit PostgreSQL, ohne ORM. Abfragen mit `sqlx::query!` und Verwandten, beim Kompilieren gegen das Schema geprüft. Der Cache in `server/.sqlx/` wird eingecheckt; die CI baut mit `SQLX_OFFLINE=true`.
- Migrationen als `server/migrations/000N_name.sql`, angewendet beim Start. Eine Migration, die auf `main` liegt, wird nicht mehr geändert, Änderungen kommen als neue Datei.
- Kein `unsafe`: Das Crate verbietet es (`[lints.rust]` in `Cargo.toml`). Was von außen kommt (Anfragen, Feeds, die alte Datenbank), darf den Server nicht zum Absturz bringen: kein `unwrap`, `expect` oder `panic!` darauf außerhalb von Tests.
- `reqwest` mit `rustls` für alle Abrufe nach außen, nur über den gehärteten Client (`rueder::http_client::HttpClient`). `server/clippy.toml` lässt den Bau eines anderen `reqwest`-Clients scheitern.
- `argon2` für Passwörter, `time` für Zeiten, `uuid` für die alten IDs, `anyhow` für Fehler im Inneren. Nach außen ein Fehlertyp mit festen Antworten (nicht gefunden, ungültige Anfrage, intern), der keine Interna verrät.
- Konfiguration über Umgebungsvariablen, lokal aus `server/.env` (Vorlage `server/.env.example`).
- Typen der API: Was als JSON hinausgeht oder hereinkommt, leitet neben `Serialize` oder `Deserialize` auch `TS` ab (`ts-rs`), und sein Modul nennt es in der Funktion `types`. Daraus schreibt `server/src/typescript.rs` die Datei `web/src/lib/api/types.ts`, die eingecheckt ist. Nach einer Änderung an einem solchen Typ: `UPDATE_API_TYPES=1 cargo test --test api_types`, die Datei mit committen. Ohne die Variable prüft derselbe Test, dass die Datei aktuell ist und kein JSON-Typ der API in ihr fehlt.
- Tests: Integrationstests in `server/tests/` mit `#[sqlx::test]`, das je Test eine frische Datenbank anlegt. Property-Tests mit `proptest`, wo ein einfaches Modell als Vergleich taugt (Lesestand, Baum). Der Test des Imports braucht keine zweite Datenbank: Die nachgebaute alte Datenbank liegt in einem eigenen Schema (`legacy`) der Testdatenbank, und der Import liest sie über eine Verbindung mit diesem `search_path`.
- Checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, Tests mit Abdeckung (`cargo llvm-cov`).

### Frontend (web/)

- SvelteKit 3 als SPA (`adapter-static`), Svelte 5 mit Runes, TypeScript im strikten Modus, TailwindCSS 4, Vite. Node 22, pnpm in der Version aus `package.json`.
- Tests mit Vitest und Testing Library, sie liegen als `*.test.ts` neben dem Code.
- Texte in `web/src/lib/i18n/` in allen Sprachen der Seite, keine Texte in Komponenten.
- Farben über benannte Variablen (`bg-paper`, `text-soft`, `border-line`), damit Themen und Akzentfarben nur Variablen umfärben.
- Checks: `pnpm lint` (Prettier), `pnpm check` (svelte-check), `pnpm test:coverage`, `pnpm build`.

### Sprache

- Code, Kommentare im Code, READMEs und Commit-Nachrichten auf Englisch.
- Kommentare in SQL-Migrationen sowie Plan, Architektur und dieses Dokument auf Deutsch.
- Die Oberfläche in den Sprachen aus `languages` in `web/src/lib/i18n/locale.svelte.ts`. Die Sprachen außer Englisch und Deutsch hat noch niemand durchgesehen, der die Sprache als Muttersprache spricht.

## Einrichten und Befehle

Voraussetzungen: Rust (stable), Node 22, pnpm, PostgreSQL 16 (lokal oder per Docker).

Das Frontend:

```sh
cd web
pnpm install
pnpm dev             # die Oberfläche auf http://localhost:5173, mit dem Server von http://127.0.0.1:3000 dahinter
pnpm dev:demo        # dasselbe ohne Server, mit erfundenen Daten
pnpm lint            # prettier --check
pnpm check           # svelte-check und TypeScript
pnpm test            # Vitest
pnpm test:coverage
pnpm e2e             # End-to-End-Tests im Browser gegen den echten Server, siehe unten
pnpm build           # die Anwendung für den Server, in build/
pnpm build:single    # die ganze Anwendung als eine build/index.html
```

`pnpm dev` reicht `/api` und `/img` an einen rueder-Server auf demselben Rechner weiter (`cargo run` in `server/`, ein Konto mit `cargo run -- account add <name>`); `RUEDER_SERVER` nennt einen anderen. `pnpm build:single` packt die Oberfläche mit erfundenen Daten in eine HTML-Datei, die ohne Server läuft. Die CI hängt sie an jeden Lauf als Artefakt `rueder-ui-draft`; so lässt sich ein Stand der Oberfläche herumreichen.

Der Server:

```sh
cp server/.env.example server/.env   # DATABASE_URL, dazu Einstellungen für den Abruf
cd server
cargo run                            # wendet Migrationen an, lauscht auf http://127.0.0.1:3000
cargo test                           # legt je Test eine Datenbank auf dem Server aus DATABASE_URL an
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo sqlx prepare -- --all-targets  # nach jeder geänderten Abfrage: Cache in .sqlx/ erneuern
cargo audit && cargo deny check      # Abhängigkeiten: bekannte Lücken, Lizenzen, Quellen (cargo install cargo-audit cargo-deny)
UPDATE_API_TYPES=1 cargo test --test api_types   # nach jedem geänderten Typ oder Handler der API: web/src/lib/api/types.ts und server/openapi.json neu schreiben

cargo run --release -- import-legacy --from postgres://user:passwort@host/rueder --dry-run   # zeigt, was zusammengeführt und angelegt würde
cargo run --release -- import-legacy --from postgres://user:passwort@host/rueder --account alt=neu   # alte Datenbank übernehmen, bei Bedarf User unter anderem Namen
cargo run -- account add <name>         # Konto anlegen, fragt nach dem Passwort; --admin für ein Admin-Konto
cargo run -- account password <name>    # neues Passwort setzen, beendet die Sitzungen des Kontos
cargo run -- account link <name> <subject>   # Konto mit einem Subject des Anbieters aus OIDC_ISSUER verknüpfen
cargo run -- account list               # Konten auflisten; in der Oberfläche legt ein Admin Konten über POST /api/accounts an
cargo run -- registration show          # ob sich jemand registrieren kann, die Grenzen und wie viele Konten es gibt
cargo run -- registration set --open true --max-accounts 50 --per-day 5   # offene Registrierung mit Grenzen; none hebt eine Grenze auf
cargo run -- registration set --invites 3 --invite-days 14 --admin-invite-accounts 20   # Einladungen je User, Gültigkeit der Links, Kontingent der Admins
cargo run -- registration set --captcha false   # die offene Registrierung ohne Captcha; true schaltet es wieder an
cargo run -- retention show             # wie lange die Zahlen der Statusseite bleiben
cargo run -- account invites <name> 10  # eigenes Einladungs-Kontingent für ein Konto; default nimmt wieder den Wert für alle
cargo run -- retention show             # wie lange was bleibt, und was die Aufbewahrung betreut
cargo run -- retention set --articles 24 --remove text --versions 12 --orphans 90   # Aufbewahrung; never hält wieder für immer
cargo run -- retention run              # alles Alte jetzt aufräumen, statt auf den stündlichen Lauf zu warten
cargo run -- account export <name> > konto.json      # alles eines Kontos als Datei für einen anderen rueder
cargo run -- account import <name> konto.json        # eine solche Datei in ein Konto nehmen, auch von einem anderen rueder
cargo run --release -- reclean          # alle Artikel aus ihren Versionen neu bereinigen, mit --feed <id> nur einen Feed
cargo run --release -- bench generate   # 10 Konten, 300 Feeds, 10 Millionen Artikel in eine leere Datenbank
cargo run --release -- bench measure    # misst die Abfragen und gibt die Tabellen aus

SOAK_SECONDS=3600 cargo test --release --test soak -- --nocapture   # Soak-Test eine Stunde, mit Tabelle
cd fuzz && cargo +nightly fuzz run -O feed -- -max_total_time=1800 -timeout=10 -max_len=131072   # ebenso cleaning, image und greader; auf GitHub läuft das in fuzz.yml
```

Fuzzen braucht Nightly-Rust und `cargo install cargo-fuzz`; wie der Korpus danach klein bleibt, steht in [server/README.md](../server/README.md#fuzzing).

Abfragen werden beim Kompilieren gegen die Datenbank aus `DATABASE_URL` geprüft; sie muss dafür das Schema haben (einmal `cargo run`), oder der Build nutzt mit `SQLX_OFFLINE=true` den Cache in `.sqlx/`, wie die CI. `cargo sqlx` kommt mit `cargo install sqlx-cli`. Das Konto der Datenbank muss Datenbanken anlegen dürfen, sonst laufen die Tests nicht.

Einstellungen des Logins: `CLIENT_IP_HEADER` nennt hinter einem Reverse-Proxy den Header mit der Adresse des Absenders, für die Rate-Limits; ohne ihn zählen alle Leser hinter dem Proxy als eine Adresse. `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` und `OIDC_REDIRECT_URL` schalten den Login über OpenID Connect ein; beim Anbieter wird `<Adresse von rueder>/api/oidc/callback` als Redirect-Adresse eingetragen. Der Sitzungs-Cookie verlangt HTTPS, zum Entwickeln geht `http://localhost:3000`. Das Passwort liest `account add` ohne Terminal als eine Zeile von der Eingabe, für Skripte.

Einstellungen der Bereinigung: `CLEANING_RULES_DIR` nennt ein Verzeichnis mit eigenen Listen (`blocked-links.txt`, `tracking-parameters.txt`) an Stelle der eingebauten aus `server/rules/`. Nach einer Änderung der Listen oder der Regeln bringt `reclean` den Bestand auf den neuen Stand; ohne das holt ihn der nächste Abruf nur für die Artikel nach, die der Feed noch führt.

Einstellungen des Bild-Proxy: `IMAGE_CACHE_DIR` ist das Verzeichnis seines Cache (Standard `image-cache` im Arbeitsverzeichnis), `IMAGE_CACHE_MB` dessen Größe (Standard 1024), `IMAGE_PROXY_KEY` der Schlüssel für die Adressen. Ohne Schlüssel erzeugt der Server einen und legt ihn ins Cache-Verzeichnis.

Die End-to-End-Tests (`pnpm e2e` in `web/`) brauchen den gebauten Server (`cargo build` in `server/`, oder `RUEDER_BIN` nennt das Programm), das gebaute Frontend (`pnpm build`), `psql` und ein PostgreSQL, dessen Konto aus `DATABASE_URL` Datenbanken anlegen darf. Jeder Lauf legt die Datenbank `rueder_e2e` neu an, startet den Server auf `localhost:3210` und eine erfundene Website auf `127.0.0.1:3211` und räumt die Datenbank beim nächsten Lauf weg. Den Browser bringt einmalig `pnpm exec playwright install chromium`.

Die Seite auf GitHub Pages ([pages/README.md](../pages/README.md)) baut der Workflow `pages.yml`; von Hand, wenn die Teile gebaut sind:

```sh
cd web && pnpm build:single && pnpm build:docs     # die Demo und die Doku des Frontends (TypeDoc)
cd web && pnpm showcase                            # die Bilder der README, von der Demo (nach build:single)
cd server && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib --document-private-items   # die Doku des Servers
cd app/android && ./gradlew --init-script ../../pages/javadoc.gradle javadoc   # die Doku der App, nach pnpm run sync in app/
cd pages && pnpm install && node build.js          # setzt alles in _site/ zusammen und prüft die Links
cd pages && pnpm lint && pnpm test
```

Im Container, wie eine Installation läuft ([deploy/README.md](../deploy/README.md)):

```sh
docker build -t rueder .                                      # das Image, aus dem Wurzelverzeichnis
cd deploy && cp .env.example .env                             # POSTGRES_PASSWORD setzen
docker compose up --detach --build                            # rueder und PostgreSQL, http://127.0.0.1:3000
docker compose exec rueder rueder account add <name> --admin  # die Befehle von oben laufen im Container
deploy/smoke-test.sh                                          # baut, startet und prüft alles unter eigenem Namen und Port
```

Das veröffentlichte Image statt eines selbst gebauten: `RUEDER_IMAGE=ghcr.io/fionapreroll/rueder:latest` in `deploy/.env`, dann `docker compose pull` und `docker compose up --detach`. Mit `RUEDER_IMAGE` startet auch `deploy/smoke-test.sh` ein fertiges Image, statt eines zu bauen.

Das alte System ist nicht mehr lauffähig: Seine Paket- und Compose-Dateien sind entfernt. Wer es starten will, nimmt den Commit `0b36a96` und das [README](../README.md) von dort.

## CI

| Workflow | Was er prüft |
|---|---|
| `ci.yml`, Job `web` | Neues Frontend: Lint, svelte-check, Tests mit Abdeckung, Build, dazu die Ein-Datei-Fassung als Artefakt. |
| `ci.yml`, Job `server` | Neuer Server: PostgreSQL 16 als Dienst, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, Tests mit Abdeckung (`cargo llvm-cov`). Gebaut wird mit `SQLX_OFFLINE=true`. Zu den Tests gehört, dass die eingecheckten TypeScript-Typen der API zu den Rust-Typen passen. |
| `ci.yml`, Job `dependencies` | Die Abhängigkeiten des Servers: `cargo audit` gegen die Datenbank von RustSec, `cargo deny check` nach `server/deny.toml` (bekannte Lücken, Lizenzen, Quellen, gesperrte Crates). Läuft auch jeden Montag, zusammen mit `workflows` und `image-scan`, damit eine neue Meldung ohne Änderung am Code auffällt. |
| `ci.yml`, Job `dependency-review` | Nur bei Pull Requests: ob der Pull Request eine Abhängigkeit mit bekannter Lücke hinzufügt oder auf eine solche Version hebt, über alles, was GitHub im Abhängigkeitsgraphen kennt (npm in `web/` und `app/`, die Actions, Cargo; Gradle nur, soweit GitHub es aus den Build-Dateien liest). Zählt ab Schwere „low“ und auch für Abhängigkeiten, die nur beim Bauen laufen. Ausgenommen sind `deepmerge` und `minimatch` 10 (siehe unten). |
| `ci.yml`, Job `workflows` | Die Workflows selbst, mit [zizmor](https://docs.zizmor.sh): Text von außen in Skripten, zu weite Rechte, Tokens, die für spätere Schritte liegen bleiben, nicht festgelegte Actions und Actions mit bekannter Lücke. Läuft auch jeden Montag. Ein Fund wird behoben, nicht mit einem Kommentar unterdrückt. |
| `ci.yml`, Job `e2e` | Die gebaute Oberfläche in Chromium gegen den gebauten Server, mit PostgreSQL 16 als Dienst und einer erfundenen Website für die Feeds (`web/e2e/`): Login, Abonnieren und Lesen, Ordner, Filterregeln, Live-Updates, getrennte Konten. Schlägt ein Test fehl, hängen Bildschirmfotos und Traces als Artefakt `e2e-report` am Lauf. |
| `ci.yml`, Job `app` | Die Android-App: Prettier und Tests der Startseite, die Unit-Tests der nativen Teile beider Builds, das APK des Builds `github` als Debug-Build und als unsigniertes Release und das des Builds `play` unsigniert, als Artefakt `rueder-android`. Den Schlüssel sieht der Job nie. |
| `ci.yml`, Job `app-changed` | Nur bei einem Push auf `main`: ob sich in `app/` seit dem Commit, auf den der Tag `android-latest` zeigt, etwas geändert hat, abgesehen von `app/README.md`, den Tests und den Einstellungen von Prettier. Ohne Tag gilt die App als geändert. |
| `ci.yml`, Job `app-release` | Nur bei einem Push auf `main`, nur wenn `app-changed` eine Änderung der App meldet, und erst, wenn `web`, `e2e` und `app` grün sind: signiert das unsignierte Release aus `app` und ersetzt mit diesem APK und seiner `version.json` das Release `android-latest`, solange `main` noch auf diesen Commit zeigt. Ohne den Schlüssel in den Secrets veröffentlicht er nichts. |
| `ci.yml`, Job `image` | Das Container-Image, je einmal für amd64 und arm64 auf einem Runner der Architektur, gebaut mit Buildx und dem Cache von GitHub Actions, in einem Build zugleich nach Docker und als OCI-Archiv geschrieben; dann startet `deploy/smoke-test.sh` es mit PostgreSQL nach `deploy/compose.yaml` und prüft Health, Frontend, Konto und Login, was dieser Build ist, Neustart mit erhaltenen Daten sowie Betrieb ohne Root und mit schreibgeschütztem Dateisystem. Danach prüft der Job, dass das Archiv genau das gestartete Image enthält, und probt auf amd64 das Veröffentlichen gegen eine Registry auf dem Runner. Bei einem Push auf `main` hängt er das Archiv an den Lauf. |
| `ci.yml`, Job `publish` | Nur bei einem Push auf `main` und erst, wenn `server`, `dependencies`, `web`, `e2e` und `image` grün sind: schiebt die beiden getesteten Archive unverändert mit `skopeo` nach `ghcr.io/fionapreroll/rueder`, je unter seinem Digest, fasst sie zu einer Manifest-Liste mit den Tags `:sha-<commit>` und `:latest` (solange `main` noch auf diesen Commit zeigt) zusammen und attestiert die Liste. Gebaut wird hier nichts. |
| `ci.yml`, Job `image-scan` | Jeden Montag: das veröffentlichte `:latest` für beide Architekturen mit Trivy gegen die bekannten Lücken seiner Systempakete (glibc, libgcc, Zertifikate aus distroless). Nur Lücken der Schwere „high“ und „critical“ mit Korrektur lassen den Lauf scheitern; die Korrektur kommt mit dem Update des Basis-Images von Dependabot und dem nächsten Push auf `main`. |
| `pages.yml` | Die Seite auf GitHub Pages: Jobs `rust` (rustdoc mit `-D warnings`), `web` (Demo, die Bilder der README von ihr mit `pnpm showcase` und TypeDoc), `app` (Javadoc), `site` (Prettier und Tests von `pages/`, dann `pages/build.js` mit der Prüfung der Links und der Bilder der README, die Seite als Artefakt `github-pages`) und nur auf `main` `deploy`, der sie veröffentlicht. Dafür muss in den Einstellungen des Repositorys unter Pages einmal „GitHub Actions“ als Quelle stehen. |
| `gitleaks.yml` | Geheimnisse in jedem Commit eines Pull Requests und in dem, was auf `main` landet. Kein Lauf nach Zeitplan: Die Geschichte ändert sich nicht, und Neues findet gitleaks in ihr nur mit einer neuen Version, die mit dem Update der Action kommt. Was vor dem Push aufhalten soll, ist die Push Protection von GitHub (siehe [Einstellungen des Repositorys](#einstellungen-des-repositorys)). |
| `codeql-analysis.yml` | Statische Analyse von Rust, JavaScript und TypeScript, dem Java der App und den Workflows. Das Java der App mit Build (`build-mode: manual`, Build `github`), damit CodeQL die Klassen von Capacitor, AndroidX und Gradle kennt, statt sie zu raten; der Rest ohne Build. |
| `fuzz.yml` | Die Fuzz-Targets in `server/fuzz/` mit Nightly-Rust: je drei Minuten bei einem Pull Request, der sie oder ihren Code berührt, je eine halbe Stunde jeden Montag, von Hand so lange wie angegeben (siehe [Fuzzing](architecture.md#fuzzing)). |
| `soak.yml` | Der Soak-Test (`server/tests/soak.rs`) eine halbe Stunde lang jeden Montag oder von Hand so lange wie angegeben. |
| `screenshots.yml` | Von Hand, für ein Review des Designs, und bei einem Pull Request, der ändert, wie die Bilder entstehen: Bildschirmfotos jeder Ansicht (Login, Listen und Ansichten, Menüs, Dialoge, Einstellungen, Konto, Status des Servers) auf dem Desktop und auf einem Handy, hell und dunkel, auf Englisch, Deutsch oder in beiden, als Artefakt `screenshots`. Wie `e2e` die gebaute Oberfläche in Chromium gegen den gebauten Server, hier mit den erfundenen Feeds des Entwurfs (`src/lib/demo/data.ts`), die eine erfundene Website als RSS ausliefert (`web/screenshots/`). Ein Konto je Sprache, Gerät und Thema; alle sind Admins, damit auch der Status drauf ist. Eine Ansicht, die scheitert, lässt den Lauf scheitern, die übrigen Bilder hängen trotzdem am Lauf. Neben den Bildern misst der Rundgang die Touch-Ziele und die Länge der Zeilen im Artikel; `measure.md` im Artefakt zählt es zusammen ([Review des Designs](design_review.md#vorgehen)). |


Die Abdeckung geht an Codecov, getrennt nach den Flags `web` und `server` (`codecov.yml`). Der Upload braucht das Repository-Secret `CODECOV_TOKEN`. Die Status von Codecov sind informativ und blockieren keinen Merge.

Der Korpus der Fuzz-Targets läuft im Job `server` als gewöhnlicher Test mit, der Soak-Test 20 Sekunden lang. Gefuzzt und länger gesoakt wird in den eigenen Workflows `fuzz.yml` und `soak.yml`.

Was die CI und das Image von außen holen, ist festgelegt (auf Fionas Wunsch):

- **Actions** stehen mit dem Hash ihres Commits und der Version als Kommentar (`@<hash> # v4.4.0`), `dtolnay/rust-toolchain` mit einem Commit seines Zweigs `master` (`# branch master`) und `toolchain`, weil sein Autor die Zweige `stable` und `nightly` bei jeder Änderung neu erzeugt: Ein Commit von dort liegt danach auf keinem Zweig mehr, und `workflows` lehnt ihn ab (`impostor-commit` von zizmor). Dependabot hebt Hash und Kommentar zusammen an, frühestens eine Woche nach dem Erscheinen (`cooldown`). Eine neue Action kommt genauso dazu, nie mit `@v1` oder einem Zweig. Der Hash ist der des Commits, bei einem annotierten Tag also nicht der des Tag-Objekts.
- **Kein Token bleibt liegen:** Jedes `actions/checkout` hat `persist-credentials: false`. `app-release` setzt den Tag `android-latest` deshalb über die API (`gh api`) statt mit `git push`. `pnpm` kommt über `pnpm/action-setup` in der Version aus `packageManager`.
- **Basis-Images** im `Dockerfile` stehen mit Tag und Digest; Dependabot hebt beide. `postgres:16` in den Diensten der CI und in `deploy/compose.yaml` bleibt beim Tag: In der CI sieht es kein Secret, und Betreiber sollen mit `docker compose pull` die Korrekturen von PostgreSQL bekommen.
- **Tools über `taiki-e/install-action`** (`cargo-audit`, `cargo-deny`, `cargo-llvm-cov`, `zizmor`, `trivy`) stehen ohne Version: Die Action nimmt dann die neueste aus ihrer eigenen Liste (`manifests/<tool>.json`) in genau dem Commit, auf den sie festgelegt ist, und prüft den Download gegen die Prüfsumme dort. Welche Version läuft, ist damit so fest wie der Hash der Action, und Dependabot hebt die Tools mit der Action an, ohne Handarbeit (Fionas Wunsch). Ausnahme ist `cargo-fuzz` in `fuzz.yml`, das nicht in der Liste steht: `cargo install cargo-fuzz@0.13.2 --locked`, gehoben nur, wenn ein neues Nightly es braucht.
- **gitleaks** kommt in der Version, die `gitleaks/gitleaks-action` in seinem festgelegten Commit nennt (8.24.3 in v2.3.9), und wird mit der Action gehoben. Die Action lädt es ohne Prüfsumme von den Releases von gitleaks; der Job darf nur lesen, ein falscher Download könnte also höchstens ein Geheimnis übersehen.
- **Gradle:** `gradle/actions/wrapper-validation` prüft im Job `app`, dass `gradle-wrapper.jar` eine von Gradle veröffentlichte ist, und der Wrapper prüft die Distribution, die er lädt, gegen `distributionSha256Sum` in `gradle-wrapper.properties`. Wer den Wrapper auf eine neue Version hebt, nimmt deren Prüfsumme aus `https://services.gradle.org/distributions/gradle-<version>-all.zip.sha256`.
- **Herkunft:** `publish` attestiert jedes Image mit `actions/attest`, `app-release` das APK. Veröffentlicht wird genau das Image, das der Smoke-Test gestartet hat, nie ein neu gebautes. Die unsignierte Provenance von Buildx entfällt dafür, die signierte Attestierung trägt die Herkunft. `gh attestation verify oci://ghcr.io/fionapreroll/rueder:latest --repo FionaPreroll/rueder` prüft vor einem `docker compose pull`, dass ein Image aus dieser CI und von diesem Repository stammt; `deploy/check.sh --server` tut das auch.
- **Signier-Schlüssel der App** nur im Job `app-release`, in der Environment `android-release`; wie man sie mit Freigabe einrichtet, steht in [app/README.md](../app/README.md#the-environment-android-release).
- **Bekannte Meldung ohne Korrektur:** Dependabot meldet `deepmerge` 4.3.1 ([GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4), niedrig), das `@sveltejs/vite-plugin-svelte` auch in seiner neuesten Version braucht. Eine korrigierte Version gibt es nicht. Es läuft nur beim Build und vereint dort die Einstellungen aus `web/svelte.config.js`, nichts von außen, und es steht nicht im gebauten Frontend. Die Meldung kann in GitHub mit „Vulnerable code is not actually used“ geschlossen werden; kommt eine Version des Plugins ohne `deepmerge` oder eine korrigierte, nimmt Dependabot sie mit.
- **Bekannte Meldung ohne nutzbare Korrektur** (von Fiona so entschieden): `minimatch` 10.2.6 ([GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249), mittel, Denial of Service über Glob-Muster), das TypeDoc für `pnpm build:docs` braucht, auch in seiner neuesten Version 0.28.20 (`^10.2.5`). Korrigiert ist erst `minimatch` 11.0.1; ein Override darauf geht nicht, weil TypeDoc `Minimatch.matchOne` aufruft, das es in 11 nicht mehr gibt. Es läuft nur beim Bauen der Doku des Frontends, mit den Mustern aus `web/typedoc.config.js`, nichts von außen, und steht weder in der Seite noch im Server. `dependency-review` lässt es durch (`allow-ghsas` in `ci.yml`); nimmt TypeDoc `minimatch` 11, fällt die Ausnahme weg.
- **Rust** bleibt das neueste `stable`; rustup prüft seine Downloads gegen die signierten Manifeste von rust-lang.org. Abhängigkeiten von npm, Cargo und Gradle sind über die Lockfiles festgelegt.

### Einstellungen des Repositorys

Was die Workflows nicht selbst festlegen können, steht in den Einstellungen des Repositorys auf GitHub. Ob es so eingestellt ist, kann die CI nicht sehen, Fiona prüft es dort.

- **Actions → General:**
  - „Workflow permissions“ auf „Read repository contents and packages permissions“ und „Allow GitHub Actions to create and approve pull requests“ aus. Die Workflows sagen ihre Rechte selbst, die Voreinstellung soll trotzdem nicht mehr geben.
  - „Approval for running fork pull request workflows from contributors“ auf „Require approval for all external contributors“. Ein Pull Request aus einem Fork führt die Build-Skripte seines Codes aus, auf den Runnern für amd64 und arm64, auch wenn er kein Secret sieht; Fiona startet sie erst nach einem Blick in den Pull Request.
  - „Allowed actions“ darf bei „Allow all actions“ bleiben: Jede Action steht mit dem Hash ihres Commits, und `workflows` prüft das. Wer es enger will, nimmt „Require actions to be pinned to a full-length commit SHA“ dazu.
- **Rules → Rulesets**, ein Ruleset für `main` (Standardzweig): Pull Request vor dem Merge, kein Force-Push, kein Löschen, und als Pflicht-Checks `server`, `dependencies`, `dependency-review`, `workflows`, `web`, `e2e`, `app`, `image (amd64, ubuntu-latest)`, `image (arm64, ubuntu-24.04-arm)`, `gitleaks` und `Analyze (actions)`, `Analyze (java-kotlin)`, `Analyze (javascript-typescript)`, `Analyze (rust)`. Nicht `fuzz`: Es läuft nur, wenn ein Pull Request seinen Code berührt. Ein zweites Ruleset für den Tag `android-latest`, das ihn nur für GitHub Actions ändern lässt, ist möglich, bei einer einzigen Person mit Schreibrecht aber nicht nötig.
- **Environments → `android-release`:** „Required reviewers“ mit Fiona, „Deployment branches and tags“ nur `main`, und dort die vier Secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` und `ANDROID_KEY_PASSWORD`. Unter „Secrets and variables → Actions“ auf Ebene des Repositorys darf dann nur noch `CODECOV_TOKEN` stehen; liegen die vier Secrets auch dort, kann jeder Workflow auf `main` sie lesen, ohne Freigabe.
- **Advanced Security:** „Dependency graph“, „Dependabot alerts“ und „Dependabot security updates“ an (der Job `dependency-review` braucht den Graphen), „Secret scanning“ mit „Push protection“ an, damit ein bekanntes Geheimnis schon beim Push abgewiesen wird, und „Private vulnerability reporting“ an, sonst kommen die Meldungen aus [SECURITY.md](../SECURITY.md) nicht an. „Code scanning“ läuft über den eigenen Workflow, nicht über „Default setup“; das Default setup bleibt aus, sonst laufen beide.
- **Packages → `rueder`:** öffentlich, mit dem Repository verknüpft und unter „Manage Actions access“ nur dieses Repository mit Schreibrecht.

## Arbeitsweise

- Jede Änderung kommt als Pull Request auf `main`. Die CI muss grün sein.
- Jedes Arbeitspaket und jeder Fix bekommt einen eigenen Branch von `origin/main`, benannt `feature/<name_in_snake_case>` oder `fix/<name_in_snake_case>`. Auf `main` wird nie direkt gepusht.
- Lieber ein Pull Request je Arbeitspaket als ein großer je Schritt.
- Vor dem Push laufen die Checks des betroffenen Teils lokal (siehe oben).
- Neuer Code kommt mit Tests. Ein Test wird nicht abgeschaltet, um die CI grün zu bekommen.
- Wer vom Plan oder von der Architektur abweicht, ändert das Dokument im selben Pull Request und nennt den Grund.
- Messwerte stehen mit Datum, Datenmenge und Maschine im Abschnitt [Messung](#messung).

## Arbeitspakete

Die Kürzel A1 bis A24 verweisen auf die [Anforderungen](plan.md#anforderungen). Fertige Pakete stehen hier mit einer Zeile; wie es gebaut ist, sagt [architecture.md](architecture.md), wie es dazu kam, der Pull Request.

### Schritt 1: Fundament und Daten

Ergebnis: ein Importprogramm, geprüft an einer nachgebauten alten Datenbank, und Messwerte, die die Speicherstrategie bestätigen.

- [x] **1.1 Grundgerüst.** Crate `server/` mit `serve`, Konfiguration, Logs, Fehlertyp und der Auslieferung von `web/build`.
- [x] **1.2 Schema.** Die Tabellen aus dem [Datenmodell](architecture.md#datenmodell), mit den Regeln als Constraints.
- [x] **1.3 Abfragen (A1, A7).** Listen, Artikel, Ungelesen-Zahlen, Lesestand. Siehe [Lesestand und Ungelesen-Zahlen](architecture.md#lesestand-und-ungelesen-zahlen).
- [x] **1.4 Import.** Unterbefehl, der die alte Datenbank liest. Siehe [Import der alten Daten](#import-der-alten-daten).
- [x] **1.5 Messung (A1, A2).** Messdaten und Messung gegen das [Messziel](plan.md#messziel). Siehe [Messung](#messung).
- [x] **1.6 CI.** Job `server`.

### Schritt 2: Backend

Ergebnis: Der neue Server läuft gegen echte Feeds.

- [x] **2.1 Gehärteter HTTP-Client (A3).** Siehe [Gehärteter HTTP-Client](architecture.md#gehärteter-http-client).
- [x] **2.2 Abruf mit Versionen (A2).** Siehe [Abruf](architecture.md#abruf).
- [x] **2.3 Bereinigung (A10).** Siehe [Bereinigung](architecture.md#bereinigung).
- [x] **2.4 Bild-Proxy (A11).** Siehe [Bild-Proxy](architecture.md#bild-proxy).
- [x] **2.5 Konten und Sitzungen (A3).** Siehe [Login und Sitzungen](architecture.md#login-und-sitzungen).
- [x] **2.6 API (A4 bis A9, A12).** Siehe [API](architecture.md#api).
- [x] **2.7 Live-Updates (A6).** Siehe [Live-Updates](architecture.md#live-updates).
- [x] **2.8 API-Typen für das Frontend.** Siehe [Typen für das Frontend](architecture.md#typen-für-das-frontend).
- [x] **2.9 Härtung (A3).** Siehe [Header, Herkunft und Limits](architecture.md#header-herkunft-und-limits).
- [x] **2.10 Betrieb.** Siehe [Betrieb](architecture.md#betrieb).
- [x] **2.11 Fuzzing (A3).** Siehe [Fuzzing](architecture.md#fuzzing).
- [x] **2.12 Security-Review.** Siehe [Restrisiken](architecture.md#restrisiken).
- [x] **2.13 Suche auf dem Server (A12).** Siehe [Suche](architecture.md#suche).
- [x] **2.14 Registrierung und Einladungen.** Siehe [Registrierung und Einladungen](architecture.md#registrierung-und-einladungen).
- [x] **2.15 Apps anderer Anbieter (A24).** Siehe [Apps anderer Anbieter](architecture.md#apps-anderer-anbieter).
  - In der Liste des Reiters stehen nur NetNewsWire und Read You. Die anderen Kandidaten (Reeder, ReadKit, Fiery Feeds, FeedMe, Fluent Reader, NewsFlash) ließen sich nicht prüfen; sie kommen dazu, sobald Fiona sie eingerichtet hat.
  - Die Grenze in der Zeit (60 Änderungen auf einmal, dann eine je Sekunde) ist nicht an echten Apps gemessen.

### Schritt 3: Frontend

Ergebnis: rueder ist täglich benutzbar. Alles hier ist unter [Frontend](architecture.md#frontend) beschrieben, wo nichts anderes steht.

- [x] **3.1 Anbindung.** Der `ServerReader`.
- [x] **3.2 Live-Updates (A6).**
- [x] **3.3 Einstellungen auf dem Server (A5)** und die änderbare Tastaturbelegung.
- [x] **3.4 Labels (A12).**
- [x] **3.5 Abonnieren und OPML (A12).**
- [x] **3.6 Tempo (A1, A4).** Virtualisierte Liste, Vorladen des nächsten Artikels.
- [x] **3.7 Prüfung im Browser.** Siehe [Prüfung im Browser](architecture.md#prüfung-im-browser).
- [x] **3.8 Statusseite für Admins (A13).** Siehe [Status für Admins](architecture.md#status-für-admins).
- [x] **3.9 Fehlerberichte aus dem Browser (A13).** Siehe [Fehlerberichte aus dem Browser](architecture.md#fehlerberichte-aus-dem-browser).
- [x] **3.10 PWA mit Update-Check (A14).**
- [x] **3.11 Android-App (A14).** Siehe [Android-App](architecture.md#android-app).
- [x] **3.12 Umzug eines Kontos (A15).** Siehe [Umzug eines Kontos](architecture.md#umzug-eines-kontos).
- [x] **3.13 Drei Bereiche (A4).**
- [x] **3.14 Magazin-Ansicht (A4).**
- [x] **3.15 Stand nach dem Neuladen.**

### Schritt 4: Umstieg

Ergebnis: Die Produktion läuft auf dem neuen rueder mit den alten Daten, der alte Code ist entfernt.

Es gibt keinen Parallelbetrieb: Die alte Produktion ist abgeschaltet, ihre Daten liegen in einem Backup. Der neue rueder läuft ohne alte Daten, die alten kommen einmalig dazu ([Entscheidungen](plan.md#entscheidungen)).

- [x] **4.1 Zweites Security-Review.** Siehe [Restrisiken](architecture.md#restrisiken).
- [x] **4.2 Inbetriebnahme.** Der neue rueder läuft in der Produktion, mit neu angelegten Konten und neu abonnierten Feeds.
- [x] **4.3 Import in den laufenden Bestand.** Siehe [Einmaliger Import in den laufenden Bestand](#einmaliger-import-in-den-laufenden-bestand).
- [x] **4.4 Probelauf.** Siehe [Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15).
- [ ] **4.5 Einmaliger Import in der Produktion.** Server anhalten, Backup, Import, Start. Weg zurück festhalten (das Backup von davor).
- [ ] **4.6 Lesestand und Labels aus dem Browser.** Das neue Frontend liest nach dem Import die Schlüssel `feedstate` und `labels` aus dem `localStorage` des alten Frontends ([user_data.md](user_data.md)) und schickt sie an einen einmaligen Endpunkt. Das geht nur, wenn das neue Frontend unter derselben Adresse läuft wie das alte und der Browser den Speicher noch hat. Weil der Import nach der Inbetriebnahme kommt, bietet das Frontend die Übernahme an, sobald das Konto importierte Daten hat, nicht beim ersten Login. Wer die Aufbewahrung (5.3) einschaltet, tut es erst danach.
- [x] **4.7 Abbau.** `backend/`, `frontend/` und `config/` sind entfernt; was 4.4 bis 4.6 vom alten Code brauchen, steht im Commit `0b36a96`.
- [x] **4.8 Feeds gesperrter Konten ruhen (A18).** Für alte, importierte Konten, die niemand mehr nutzt. Siehe [Konto löschen und sperren](architecture.md#konto-löschen-und-sperren).

### Schritt 5: Öffentliche Instanz

Ergebnis: Eine öffentliche Instanz lässt sich mit rueder betreiben, ohne dass sich für eine selbst gehostete etwas ändert (A16). Die Nummern stammen aus Fionas eigener Planung; die Pakete, die hier fehlen, gehören nicht zu diesem Repository.

- [x] **5.2 Abruf einstellbar (A6).** Siehe [Abruf](architecture.md#abruf).
- [x] **5.3 Aufbewahrung (A21, E4).** Siehe [Aufbewahrung](architecture.md#aufbewahrung).
- [x] **5.4 E-Mail am Konto (A16, A17, E6).** Siehe [E-Mail am Konto](architecture.md#e-mail-am-konto).
- [x] **5.5 Passwort vergessen (A17).** Siehe [Passwort vergessen](architecture.md#passwort-vergessen).
- [x] **5.6 Konto löschen und sperren (A18).** Siehe [Konto löschen und sperren](architecture.md#konto-löschen-und-sperren).
- [x] **5.7 Zweiter Faktor (A22).** Siehe [Zweiter Faktor](architecture.md#zweiter-faktor).
- [x] **5.8 Rechtliches in der Oberfläche (A20).** Siehe [Rechtliches](architecture.md#rechtliches).
- [x] **5.9 Meldeweg und Sperre von Feeds (A20, E12).** Siehe [Meldungen und gesperrte Feeds](architecture.md#meldungen-und-gesperrte-feeds).
- [x] **5.10 Seitenleiste, Breiten und der Artikelbereich.** Siehe [Frontend](architecture.md#frontend).
- [ ] **5.11 Betrieb der öffentlichen Instanz (A23).** Noch nicht ausgearbeitet; dazu gehört die Kontaktstelle nach dem DSA, die Paket 5.9 angekündigt hat.
- [x] **5.12 Was dieser Build ist, Images für arm64, und was sich an einem Artikel änderte.** Siehe [Status für Admins](architecture.md#status-für-admins) und [Artikelspeicher und Versionen](architecture.md#artikelspeicher-und-versionen).
- [x] **5.13 Einstellungen und Kontoeinstellungen getrennt.** Siehe [Frontend](architecture.md#frontend).
- [x] **5.14 Status über den Neustart.** Am 2026-11-14 wieder entfernt; für längere Zeit gibt es die Metriken (5.25).
- [x] **5.15 Passkeys (A22).** Siehe [Passkeys](architecture.md#passkeys).
- [x] **5.16 Ungelesene Artikel in der Seitenleiste.** Siehe [Frontend](architecture.md#frontend).
- [x] **5.17 Passkeys in der Android-App (A22).** Siehe [Passkeys](architecture.md#passkeys). Der Fingerabdruck der App von GitHub (`asset_links::OFFICIAL`) stammt aus dem `rueder.apk` des Releases `android-latest`; ändert sich ihr Schlüssel, muss er dort mit.
- [x] **5.19 Verbindungsanzeige.** Siehe [Frontend](architecture.md#frontend).
- [x] **5.20 Änderungen ohne Verbindung.** Siehe [Frontend](architecture.md#frontend).
- [x] **5.21 PWA am Desktop**, statt einer eigenen Desktop-App. Siehe [Frontend](architecture.md#frontend).
- [x] **5.22 Seite auf GitHub Pages.** Siehe [Seite auf GitHub Pages](architecture.md#seite-auf-github-pages).
- [x] **5.23 Die REST-API auf der Seite.** Siehe [Beschreibung der Schnittstelle](architecture.md#beschreibung-der-schnittstelle-openapi).
- [x] **5.24 Review des Designs.** Siehe [design_review.md](design_review.md).
- [x] **5.25 Metriken für Prometheus (A13).** Siehe [Metriken für Prometheus](architecture.md#metriken-für-prometheus).
- [x] **5.26 Bilder in der README.** Siehe [Seite auf GitHub Pages](architecture.md#seite-auf-github-pages).
- [x] **5.27 Ein Knopf schließt den offenen Artikel.** Siehe [Frontend](architecture.md#frontend).
- [x] **5.28 Captcha bei der Registrierung.** Siehe [Captcha](architecture.md#captcha).
- [x] **5.29 Drossel für Registrierungen.** Siehe [Registrierung und Einladungen](architecture.md#registrierung-und-einladungen).
- [x] **5.30 Lastmessung für viele Konten.** `rueder bench readers` und die Lasttests in `web/load/`, siehe [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15) und `server/README.md` unter „Measuring“.
- [x] **5.31 Live-Updates je Konto (A6).** Siehe [Live-Updates](architecture.md#live-updates).
- [x] **5.32 Abruf je Host über Runden hinweg (A6).** Siehe [Abruf](architecture.md#abruf) und [Gehärteter HTTP-Client](architecture.md#gehärteter-http-client).
- [x] **5.33 Größen einstellbar.** `DATABASE_CONNECTIONS`, `HEAVY_AT_ONCE` und `MAILS_PER_DAY`, siehe [Header, Herkunft und Limits](architecture.md#header-herkunft-und-limits), [Verbindungen zur Datenbank](#verbindungen-zur-datenbank-vom-2026-11-19) und die Tabelle für 10.000 und 100.000 Konten in [deploy/README.md](../deploy/README.md#a-large-instance).
- [x] **5.34 Grenzen im Speicher für viele Schlüssel.** Siehe [Header, Herkunft und Limits](architecture.md#header-herkunft-und-limits) und [Grenzen im Speicher vom 2026-11-19](#grenzen-im-speicher-vom-2026-11-19).
- [x] **5.35 Platz und Datenbank für große Instanzen.** Die Seiten der Abos lassen Platz für die nächste Lesemarke (Migration 0030), siehe [Lesestand und Ungelesen-Zahlen](architecture.md#lesestand-und-ungelesen-zahlen), [Platz, Arbeitsspeicher und Abos](#platz-arbeitsspeicher-und-abos-vom-2026-11-19) und „A large instance“ in [deploy/README.md](../deploy/README.md#a-large-instance).
- [x] **5.36 Bilder ohne geteilten Cache.** Siehe [Bild-Proxy](architecture.md#bild-proxy), [Bild-Proxy unter Last](#bild-proxy-unter-last-vom-2026-11-19) und `web/load/images.js`.
- [x] **5.37 Messen in der Form des alten Bestands.** Siehe [Suche vom 2026-11-15](#suche-vom-2026-11-15).
- [x] **5.38 Gespeicherte Wörter für die Suche.** Siehe [Gespeicherte Wörter vom 2026-11-16](#gespeicherte-wörter-vom-2026-11-16).
- [x] **5.39 Fortschritt langer Läufe.** Migrationen, `import-legacy` und `reclean` sagen im Log regelmäßig, wie weit sie sind (`server/src/progress.rs`).
- [x] **5.40 Feed-Infos.** Siehe [Ordner und Abos](architecture.md#ordner-und-abos).
- [x] **5.41 User-Agent je Feed nur für Admins.** Siehe [Detailfragen](plan.md#detailfragen).
- [x] **5.42 Duplikate unter neuer Kennung.** Siehe [Abruf](architecture.md#abruf) und [Duplikate vom 2026-11-16](#duplikate-vom-2026-11-16).
- [x] **5.43 Abruf achtet auf 429 und `Retry-After`.** Siehe [Abruf](architecture.md#abruf).
- [x] **5.44 Bilder über ein CDN (optional).** Siehe [Bild-Proxy](architecture.md#bild-proxy) und „Images through a CDN“ in [deploy/README.md](../deploy/README.md#images-through-a-cdn).
- [x] **5.45 Artikel-Infos.** Siehe [Artikelspeicher und Versionen](architecture.md#artikelspeicher-und-versionen).
- [x] **5.46 Bilder und Links im Vergleich der Versionen.** Siehe [Artikelspeicher und Versionen](architecture.md#artikelspeicher-und-versionen).
- [x] **5.47 Artikelseite: wohin die Links führen.** Neben dem Titel steht leise die Domain, auf die er verlinkt; Titel und Name des Feeds sagen beim Überfahren, ob sie zum Original führen oder in rueder bleiben. Das Menü „…“ und „Schließen“ stehen zusammen am Ende der ersten Zeile unter dem Text (`ArticleBody.svelte`).
- [x] **5.48 Quelltext des Artikels.** Siehe [Artikelspeicher und Versionen](architecture.md#artikelspeicher-und-versionen).
- [x] **E13 Inaktive Konten (A18, E13).** Siehe [Aufbewahrung](architecture.md#aufbewahrung).

### Nicht mit Fiona abgesprochen

Was beim Bauen selbst entschieden wurde und noch ihr Wort braucht:

- **Grenzen im Speicher (5.34):** Ein neuer Schlüssel sieht 16 Einträge an (`table::LOOK`). Ein gedrosselter Schlüssel kann damit gehen, wenn alle 16 neben ihm gedrosselt sind, nicht erst, wenn es alle der Tabelle sind. Abgelaufene Einträge gehen einzeln, wenn ein neuer Platz braucht, statt alle auf einmal; eine Tabelle bleibt also voll, wenn sie einmal voll war. `indexmap` ist neu als direkte Abhängigkeit, es war schon im Baum. Die benutzten Aufgaben des Captchas verdrängen nichts, denn eine verdrängte gälte wieder; voll nimmt das Captcha wie bisher keine Lösung an.
- **Aufbewahrung (5.3):** Bei „nur der Text“ bleiben Schlagwörter, Anhänge und Bild, damit eine Podcast-Folge abspielbar bleibt. Was beim Aufräumen markiert war, bleibt, auch wenn die Markierung später geht. Ein gesperrter Feed ohne Leser behält seine Zeile und damit die Sperre. Feeds ohne Leser zählen ihre Tage ab dem ersten Lauf, der sie so sieht. Der Server räumt stündlich höchstens fünf Minuten lang in Portionen von 500 Artikeln.
- **E-Mail am Konto (5.4):** Mit `optional` fragt die Seite nach dem Login nicht. Mit `required` fragt sie bestehende Konten in einem Fenster, das nur der Link oder das Abmelden schließt, sperrt aber nicht die API oder Apps. Ein Konto, das mit `required` registriert und nie bestätigt wird, geht nach 7 Tagen. Ob eine Adresse schon einem anderen Konto gehört, sagt erst der Link. Mails sind nur Text und gehen sofort, ohne Warteschlange: Nimmt das Relais eine nicht, antwortet der Server 503.
- **Passwort vergessen (5.5):** Gesucht und verschickt wird nach der Antwort, so dass auch die Zeit nichts verrät. Ein Link gilt nur, solange das Konto die Adresse hat, an die er ging. Wer Passwort und zweiten Faktor zugleich verliert, braucht den Betreiber (`rueder account second-factor-off`).
- **Ein Knopf schließt den offenen Artikel (5.27):** Er steht am Ende der Zeile unter dem Text, wo der Leser nach dem Lesen ist, nicht oben am Titel, und zeigt das Kreuz und „Schließen“, auf schmalen Bildschirmen nur das Kreuz, damit er mit „…“ in der ersten Zeile bleibt (5.47). Im Magazin steht er zusätzlich zum Kreuz oben am Blatt, damit jeder offene Artikel gleich endet.
- **Drossel für Registrierungen (5.29):** Je Netz 5 Versuche auf einmal und dann einer alle 12 Minuten statt der vorgeschlagenen 3 und einer alle 20 Minuten, weil ein vergebener Name ebenso zählt und sich viele Leser im Mobilfunk eine IPv4-Adresse teilen. Der Topf des ganzen Servers zählt Versuche mit einer gültigen Lösung auch dann, wenn sie danach aus anderem Grund abgewiesen werden.
- **Abruf bei 429 (5.43):** Nach einem 429 bleibt das längere Intervall einen Tag, statt sich langsam zu verkürzen oder dauerhaft zu bleiben; ein weiteres 429 verdoppelt es wieder und beginnt den Tag neu. `Retry-After` zählt auch bei 503, nicht bei anderen Fehlern. Andere Feeds desselben Hosts warten nicht mit; jeder richtet sich nach seinem eigenen 429.
- **Captcha (5.28):** Die Höchstzahl ist 100.000 und eine Aufgabe gilt fünf Minuten; im Speicher stehen höchstens 100.000 benutzte. Die Seite rechnet in einem Strang, ohne Worker, und der Knopf sagt derweil „Dieser Browser wird geprüft …“. Zählt eine Lösung nicht, etwa nach einem Neustart des Servers mitten im Rechnen, bittet die Seite, es noch einmal zu versuchen, statt selbst eine neue Aufgabe zu holen. Das Wort der Ablehnung ist `captcha`, wie der Schalter.
- **Lastmessung (5.30):** Die Lasttests sind Node ohne Pakete in `web/load/`, neben den Werkzeugen der Tests im Browser, statt ein zweites Programm in Rust. `rueder bench readers` schreibt die Tokens der Sitzungen in eine Datei, damit ein Lasttest nicht für jeden Leser ein Passwort hashen lässt, und lehnt jede Datenbank mit einem Konto ab, das keine erfundenen Daten gemacht haben.
- **Duplikate (5.42):** Ein Eintrag ohne Titel wird nur an seiner Kennung erkannt, auch mit Datum.
- **Artikel-Infos (5.45):** Der Feed steht mit dem Namen, den er sich gibt, nicht mit dem, den ihm der Leser gab, damit der Betreiber ihn wiederfindet. „Zuerst abgerufen“ ist bei importierten Artikeln, wann der alte rueder sie zuerst sah. Das Menü unter dem Artikel ist immer da, „Artikel melden …“ darin nur, wo der Server Meldungen nimmt.
- **Quelltext (5.48):** Er zeigt die neueste aufbewahrte Version, wie der Feed sie schickte, nicht jede; ältere zeigt der Vergleich der Versionen. Wo ein Tag auf einen anderen folgt, beginnt eine neue Zeile, damit sich der Text lesen lässt; kopiert wird er unverändert.
- **Passkeys in der Android-App (5.17):** `assetlinks.json` nennt neben `get_login_creds` auch `handle_all_urls`, wie Googles Beispiel; Android darf es eine Stunde zwischenspeichern; höchstens 10 Apps.

### Für später

Von Fiona notiert, noch keinem Schritt zugeordnet:

- **Mehrere Prozesse (horizontal).** Von Fiona am 2026-11-15 zurückgestellt: Erst trägt eine Maschine 100.000 Konten (5.30 bis 5.36). Laufen später mehrere Prozesse desselben Programms hinter einem Load Balancer, braucht es:
  1. Ereignisse zwischen den Prozessen über `LISTEN/NOTIFY` von PostgreSQL, aufbauend auf 5.31; hinüber geht nur die Nummer von Konto oder Feed, weil eine Nachricht höchstens 8 KB hat.
  2. Die Login-Grenzen, die Zeremonien der Passkeys, die Aufgaben des Captchas und die Drossel der Registrierung gemeinsam in PostgreSQL; im Speicher jedes Prozesses vervielfachten sie die erlaubten Versuche.
  3. Die stündlichen Aufgaben (Aufbewahrung, Aufräumen) nur in einem Prozess, über eine Sperre in PostgreSQL (Advisory Lock).
  4. Einen Prozess nur für den Abruf und `FETCH=off` auf den übrigen; die Feeds teilt `FOR UPDATE SKIP LOCKED` schon heute.
  5. Einen gemeinsamen Cache der Bilder oder ein CDN, mit demselben `IMAGE_PROXY_KEY` auf allen.
  6. PgBouncer für die Verbindungen zur Datenbank, mit einer eigenen direkten Verbindung für `LISTEN`.
  7. Metriken und Statusseite je Prozess, zusammengeführt in Prometheus.
  8. Für die Ausfallsicherheit eine PostgreSQL in Bereitschaft und Backups mit Wiederherstellung auf einen Zeitpunkt.

- **Ein Fuzz-Target für die Datei eines Kontos** (`transfer.rs`), die jedes Konto über `POST /api/import` hochladen kann. Das Parsen macht serde; das Target prüfte vor allem die eigenen Grenzen und Abgleiche.
- **Offline lesen.** Pro Gerät die neuesten, etwa 200 ungelesenen Artikel mit Texten und Bildern im Browser halten. Zurückgestellt, weil es mit der Regel bricht, dass im Browser nichts liegt, was einem Konto gehört (siehe [Frontend](architecture.md#frontend)), und weil der Reader dann die Zahlen ohne Server schätzen müsste. Offline lesen geht heute mit den Apps über die Google-Reader-API.
- **Eine Desktop-App** mit Tauri, als Hülle wie die Android-App. Zurückgestellt, weil sie wenig über die installierte PWA (5.21) hinaus brächte. Dagegen stehen: unter Linux keine Passkeys (WebKitGTK kann kein WebAuthn), Kosten für das Signieren unter macOS und Windows, und drei Betriebssysteme in der CI.

## Import der alten Daten

Die alte Datenbank ist PostgreSQL, das Schema stand in `backend/migrations/` (Fizz), die Modelle in `backend/pkg/repository/pop/models/` (beides im Commit `0b36a96`, nachgebaut in `server/tests/fixtures/legacy_schema.sql`). Der Import (`rueder import-legacy --from <Adresse der alten Datenbank>`, Code in `server/src/import.rs`) liest sie über eine zweite Verbindung in einer einzigen Nur-Lese-Transaktion und ändert sie nicht. Seit Paket 4.3 führt er die alten Daten mit dem zusammen, was der neue rueder schon hat, siehe [unten](#einmaliger-import-in-den-laufenden-bestand); die Tabelle sagt, wie ein Feed, Artikel oder User aussieht, der neu dazukommt.

| Alt | Neu | Hinweise |
|---|---|---|
| `feeds` (`id`, `feed_url`, `site_url`, `title`, `icon`, `fetched_at`, `fetch_delay_s`, `fetcher_state` als JSON mit `etag`, `last_modified`, `working`, `last_success`, `last_error`, `message`) | `feeds` | Die UUID wandert nach `legacy_id`. ETag und Abrufstand werden übernommen, damit der erste Abruf bedingt bleibt; er ist fällig, wenn der alte rueder wieder abgerufen hätte. Das Intervall wird auf 15 Minuten bis 12 Stunden begrenzt. Die Fehlermeldung kommt nur mit, wenn der Feed zuletzt nicht ging. `last_seq` ist die Zahl der Artikel des Feeds. Ein Feed ohne Adresse bleibt samt Artikeln weg und wird gemeldet. |
| `articles` (`id`, `seq` als globaler Zähler, `feed_id`, `site_guid`, `posted_at`, `link`, `thumbnail`, `image`, `image_title`, `title`, `teaser`, `content` als JSON mit `authors`, `tags`, `enclosures`, `text`) | `articles`, `article_content`, `article_versions` | Die neue Nummer im Feed ist `row_number() over (partition by feed_id order by seq)`, genau wie das alte System `feed_seq` berechnet. Das alte `seq` wird die neue ID, die Reihenfolge des Eintreffens bleibt also. Der heutige Text wird Version 1: im Original nach `article_versions` (Abrufzeit ist das alte `created_at`), als aktueller Text nach `article_content`. Die UUID steht in der Tabelle `legacy_articles`, nicht in einer Spalte, damit die heiße Tabelle schmal bleibt und die Zuordnung nach dem Umstieg wegkann. Der Text zum Lesen kommt aus der neuen [Bereinigung](architecture.md#bereinigung), wie bei einem Artikel, der heute abgerufen wird; auch Link, Vorschaubild und Bild verlieren ihre Tracking-Parameter. |
| `users` (`id`, `auth_origin`, `auth_subject`, `folders` als JSON) | `accounts`, `account_identities`, `folders`, `subscriptions` | Gibt es kein Konto, mit dem der User [zusammengeführt](#einmaliger-import-in-den-laufenden-bestand) wird, wird `auth_subject` der Login-Name des neuen Kontos; ist er vergeben (Namen unterscheiden keine Groß- und Kleinschreibung) oder leer, bekommt das Konto einen anderen, und der Import meldet es. `auth_origin` und `auth_subject` stehen in `account_identities`. Die Ordner haben eine Ebene; ihre Reihenfolge und die der Feeds darin kommt aus der Reihenfolge im JSON. Hat ein User einem Feed einen eigenen Namen gegeben, wird er der Titel des Abos. |
| `user_feeds` (`user_id`, `feed_id`) | `subscriptions` | Ein Abo, das in keinem Ordner steht, landet auf oberster Ebene hinter den Ordnern. |
| `user_states` | nichts | Die Tabelle wird vom alten System nicht benutzt. Der Import meldet, wenn sie Zeilen enthält. |
| `labels` | nichts | Im alten Go-Code gibt es kein Modell dazu, die Labels liegen im Browser. Der Import meldet, wenn die Tabelle Zeilen enthält. |
| `localStorage` im Browser: `feedstate`, `labels` | Lesemarke, Ausnahmen, `labels`, `article_labels` | Nicht Teil des Imports, sondern von Paket 4.6. `readAllUntil` wird die Lesemarke, `readArticles` (Nummern im Feed) werden Ausnahmen, Artikel von Labels werden über ihre UUID (`legacy_id`) gefunden. |

Nach dem Import und vor der Übernahme aus dem Browser hat ein Konto für die alten Artikel keinen Lesestand. Gebaut ist der Vorschlag: Alte Artikel gelten als gelesen, und die Übernahme aus dem Browser (Paket 4.6) setzt den genauen Stand einmalig. Wer keinen alten Browser-Speicher mehr hat, beginnt dann mit null ungelesenen alten Artikeln statt mit zehntausenden.

Gemessen am 2026-11-10, noch mit der Wiederholung aus Schritt 1 und in eine leere Datenbank, an einer nachgebauten alten Datenbank mit 1 Million Artikeln (1,4 GB): 3 Minuten 50 Sekunden, also rund 4.300 Artikel je Sekunde und hochgerechnet etwa 40 Minuten für 10 Millionen. Ein zweiter Lauf ohne Neues dauert 35 Sekunden. Das Ergebnis stimmte in Nummern, Reihenfolge, Titeln und Texten aller Artikel mit der Quelle überein (Prüfsumme über beide Datenbanken). Seit Paket 2.3 bereinigt der Import jeden Text; das ist in dieser Messung nicht enthalten und kostet nach den Zeiten der Bereinigung grob 0,2 Millisekunden je Artikel zusätzlich, bei 10 Millionen also etwa eine halbe Stunde. An echten Daten siehe [Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15). Eine Messung des Zusammenführens steht unter [Import vom 2026-11-12](#import-vom-2026-11-12).

### Einmaliger Import in den laufenden Bestand

Gebaut mit Paket 4.3. Der neue rueder läuft schon, mit eigenen Konten und Feeds, wenn die alten Daten aus dem Backup dazukommen. Der Import führt sie mit dem Bestand zusammen:

- **Ablauf.** Die alte Datenbank muss für den Import erreichbar sein: als Backup in ein PostgreSQL eingespielt oder mit ihrem alten Volume in einem eigenen Container mit PostgreSQL 13 ([deploy/README.md](../deploy/README.md#bringing-over-the-old-rueder)). Der Import liest sie nur. Der neue Server steht während des Imports, davor gibt es ein Backup der neuen Datenbank. `rueder import-legacy --from <alte Datenbank> --dry-run` zeigt vorab als Trockenlauf, welche Feeds und Konten zusammengeführt und welche neu angelegt würden, und schreibt nichts. Danach derselbe Aufruf ohne `--dry-run`. Am Ende sagt der Import, was er zusammengeführt, angelegt und ausgelassen hat.
- **Feeds.** Ein alter Feed wird mit einem vorhandenen zusammengeführt, wenn die Adresse gleich ist. Gleich heißt zuerst wörtlich; sonst zählen `http` und `https`, Groß- und Kleinschreibung des Hosts, der Standard-Port und ein Schrägstrich am Ende nicht (`import::address_key`), weil der neue rueder beim Abonnieren oft die Adresse mit `https` gespeichert hat. Wo nur so ein Paar gefunden wurde, nennt der Bericht beide Adressen, damit der Trockenlauf zeigt, ob es stimmt. Ein Feed des neuen rueder nimmt höchstens einen alten auf. Der vorhandene behält Titel, Adresse, Intervall und Abrufstand und bekommt die alte UUID in `legacy_id`. Alle anderen alten Feeds kommen neu dazu, wie in der Tabelle oben.
- **Artikel eines zusammengeführten Feeds.** Die alten Artikel sind älter als alles, was der neue rueder abgerufen hat, und stehen deshalb davor: Sie bekommen die Nummern 1 bis n in ihrer alten Reihenfolge, die vorhandenen Artikel rücken um n nach hinten, mit den Lesemarken, Ausnahmen und Labels ihrer Abonnenten. Die Fremdschlüssel auf `articles` folgen dafür der Änderung (`ON UPDATE CASCADE`, Migration `0006_merging_import.sql`) und nehmen Text, Fassungen, Labels und alte UUIDs mit; die Kopie für die Suche in `article_content` zieht so ebenfalls mit. Die Ausnahmen in `article_states` hängen am Abo, die verschiebt der Import selbst (`import::move_up`), in zwei Schritten über die höchste Nummer hinaus, damit zwei Zeilen nie dieselbe Nummer haben. Das ist die einzige Stelle, an der vergebene Nummern sich ändern; sie ist vertretbar, weil der Server steht und es einmal geschieht. Offene Tabs laden nach dem Neustart des Servers ohnehin alles neu (`hello`). Ein Artikel, den beide Seiten haben (gleiche Kennung im Feed), bleibt, wie der neue rueder ihn hat, und wird nicht doppelt angelegt; seine alte UUID zeigt auf ihn.
- **Reihenfolge des Eintreffens.** Die alten Artikel behalten ihr altes `seq` als ID. Damit sie vor allem stehen, was der neue rueder abgerufen hat, bekommen dessen Artikel vor dem ersten alten einmal neue IDs hinter dem höchsten alten `seq` (`import::make_room`), und neue Artikel kommen danach.
- **Konten.** Ein alter User wird mit dem Konto gleichen Namens zusammengeführt (Groß- und Kleinschreibung zählt nicht) oder mit dem Konto, das der Aufruf nennt: `--account <alter Name>=<neuer Name>`, mehrfach möglich. Ein Konto, das der Aufruf nennt, nimmt keinen anderen alten User gleichen Namens auf. Nennt der Aufruf einen User oder ein Konto, das es nicht gibt, oder eins doppelt, bricht der Import ab, bevor er etwas schreibt. Das Konto bekommt die alte UUID und die alte Anmeldung (`auth_origin`, `auth_subject`) in `account_identities`, sein Passwort bleibt. Alte User ohne Gegenstück bekommen ein neues Konto ohne Passwort; `rueder account password` setzt eins.
- **Ordner und Abos beim Zusammenführen.** Was das Konto im neuen rueder eingerichtet hat, bleibt: Ein Abo, das es schon hat, behält Ordner, Titel und Platz. Die übrigen alten Abos kommen mit ihren alten Ordnern dazu, hinter das Vorhandene; ein alter Ordner, den es auf oberster Ebene mit gleichem Titel schon gibt (ohne Groß- und Kleinschreibung), geht in diesem auf, und ein alter Ordner, aus dem nichts mehr dazukommt, bleibt weg. Feeds, die in keinem Ordner standen, kommen ans Ende der obersten Ebene. Die Filterregeln des Kontos sehen sich jedes neue Abo an wie beim Abonnieren (`filters::subscribed`). Die Grenzen je Konto (2.000 Feeds) gelten für den Import nicht.
- **Lesestand.** Alte Artikel gelten als gelesen, bis der Stand aus dem Browser kommt (Paket 4.6). Für ein vorhandenes Abo rückt die Lesemarke um die n alten Artikel mit, ungelesen bleibt, was ungelesen war. Bei einem Abo, das der Import neu anlegt, steht die Lesemarke am Ende der alten Artikel, und alte Artikel, die der neue rueder schon hatte, sind als Ausnahme gelesen: Was der neue rueder sonst seither abgerufen hat, ist ungelesen.
- **Zuordnung für den Browser.** Der alte Browser-Speicher nennt Artikel mit ihrer alten Nummer im Feed. Wo ein Artikel doppelt war, stimmt sie nicht mehr mit der neuen überein. `legacy_articles` hat deshalb seit Migration `0006_merging_import.sql` die alte Nummer als Spalte `legacy_seq` (eindeutig je Feed) und in `matched`, ob der Artikel einer war, den der neue rueder schon hatte.
- **Abbruch.** Geschrieben wird je Feed mit allen seinen Artikeln in einer Transaktion, danach je Konto mit seinem Baum in einer. Ein Feed oder Konto mit `legacy_id` ist fertig. Bricht der Import ab, wird er einfach erneut gestartet und macht mit dem weiter, was fehlt; der Trockenlauf mit `--dry-run` zeigt auch dann, was noch kommt. Was die alte Datenbank seit dem ersten Lauf dazubekommen hätte, bringt ein zweiter Lauf nicht mehr: Das alte System läuft nicht mehr, ihr Stand ist das Backup. Die Wiederholung aus Schritt 1, die neue Artikel bekannter Feeds hinten anhängte, gibt es seither nicht mehr.

Geprüft in `server/tests/import.rs`: eine nachgebaute alte Datenbank in einen Bestand mit Konten (eins mit anderer Schreibung des Namens, eins unter anderem Namen per `--account`, eins ohne alten User), Feeds (einer mit gleicher Adresse und einem Artikel, den beide haben, einer mit anders geschriebener Adresse, einer ohne Gegenstück), Lesestand, Stern, Label und einer Filterregel; Reihenfolge, Nummern, IDs, alte Nummern, Text, Fassungen und Suchkopie, Ordner, Abos, Lesestand und Ungelesen-Zahlen werden geprüft. Dazu ein Abbruch mitten in einem zusammengeführten Feed und einer nach dem ersten Feed, beide fortgesetzt, ein Trockenlauf und falsche Aufrufe, die nichts schreiben, und ein Vergleich mit dem einfachen Modell (zuerst die alten Artikel, die nur der alte rueder hat, dann alles des neuen; Lesestand und Sterne bleiben am Artikel) mit zufälligen Feeds in 30 Runden. Der hat einen Fehler gefunden: Hatte ein zusammengeführter Feed noch keine eigenen Artikel, rückte die Lesemarke seiner Abonnenten nicht mit.

## Messung

Methode für Paket 1.5:

- Daten: 10 User, 300 Feeds, 10 Millionen Artikel. Die Feedgrößen sind schief verteilt (wenige sehr große Feeds, viele kleine), jeder User abonniert einen großen Teil der Feeds und hat einen realistischen Lesestand mit Ausnahmen.
- Gemessen wird an der Datenbank, mit warmem Cache und nach einem Neustart: Liste eines Feeds (erste Seite und eine Seite tief im Feed, im größten Feed), gemischte Liste über alle Abos, Artikel öffnen, Ungelesen-Zahlen aller Feeds eines Kontos, „alle als gelesen“.
- Je Abfrage Median, 95. und 99. Perzentil über viele Läufe mit wechselnden Parametern, dazu der Plan aus `EXPLAIN (ANALYZE, BUFFERS)`.
- Ziel: jeweils unter 50 ms. Dazu die Größe der Tabellen und Indizes auf der Platte.
- Wiederholung mit zehnfacher Artikelzahl, um zu sehen, ob die Zeiten von der Gesamtgröße unabhängig bleiben; passt das nicht auf die Maschine, mit einem Zehntel.

### Ergebnisse vom 2026-11-10

Alle Abfragen bleiben unter 50 ms, auch im 99. Perzentil und auch direkt nach einem Neustart. Die Zeiten hängen nicht von der Gesamtzahl der Artikel ab. Die Speicherstrategie mit drei Tabellen ist damit bestätigt, das Schema blieb, wie es war; korrigiert wurde die Listenabfrage (siehe [Auswertung](#auswertung)).

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher, virtuelle Platte. PostgreSQL 16.15 mit `shared_buffers = 2GB`, sonst Standardwerte; `jit = off` setzt der Server selbst. Server und Datenbank auf derselben Maschine.
- **Daten:** `rueder bench generate`: 10 Konten, 300 Feeds, 10.000.000 Artikel über zehn Jahre. Die Feedgrößen folgen einer Zipf-Verteilung, der größte Feed hat 1.592.179 Artikel, der kleinste rund 5.000. Das erste Konto abonniert alle Feeds, die anderen sechs von zehn, zusammen 1.893 Abos in vier Ordnerebenen. Lesestand von „alles gelesen“ bis „nie geöffnet“ mit 101.674 Ausnahmen. Jeder zwanzigste Artikel hat eine zweite Version, jeder hundertste eine dritte. Das Erzeugen dauert 17 Minuten.
- **Messung:** `rueder bench measure`: jede Abfrage 500-mal mit wechselnden Konten, Feeds und Stellen in der Liste, Seiten zu 50 Artikeln. Gemessen wird im Rust-Programm, also mit dem Weg zur Datenbank und zurück; die Zeit an der Datenbank allein ist kürzer. „Tief in der Liste“ heißt: vor einer zufälligen Stelle in zehn Jahren. Die Messung setzt den Lesestand danach zurück, ein zweiter Lauf misst also dasselbe.

| Abfrage | Median | p95 | p99 | Nach Neustart: Median | Nach Neustart: p99 | 1 Mio. Artikel: Median | 1 Mio. Artikel: p99 |
|---|---:|---:|---:|---:|---:|---:|---:|
| Ein Feed, erste Seite | 1,8 ms | 2,9 ms | 4,5 ms | 2,2 ms | 6,1 ms | 1,7 ms | 5,2 ms |
| Ein Feed, nur ungelesen | 1,3 ms | 2,2 ms | 2,5 ms | 1,8 ms | 4,3 ms | 1,1 ms | 3,0 ms |
| Größter Feed, tief in der Liste | 1,9 ms | 2,4 ms | 3,1 ms | 3,0 ms | 6,2 ms | 1,8 ms | 3,0 ms |
| Ein Ordner, nur ungelesen | 2,6 ms | 3,2 ms | 3,7 ms | 2,2 ms | 4,5 ms | 2,7 ms | 4,7 ms |
| Alle Feeds, erste Seite | 7,1 ms | 10,5 ms | 12,9 ms | 7,1 ms | 13,5 ms | 7,2 ms | 13,8 ms |
| Alle Feeds, nur ungelesen | 8,6 ms | 15,5 ms | 17,9 ms | 8,8 ms | 29,7 ms | 9,5 ms | 17,4 ms |
| Alle Feeds, tief in der Liste | 10,8 ms | 14,1 ms | 17,5 ms | 19,7 ms | 34,0 ms | 9,9 ms | 15,5 ms |
| Alle Feeds, nur ungelesen, tief in der Liste | 11,4 ms | 14,1 ms | 15,9 ms | 16,2 ms | 26,2 ms | 11,9 ms | 24,9 ms |
| Ungelesen-Zahlen aller Feeds eines Kontos | 7,1 ms | 11,7 ms | 14,7 ms | 7,2 ms | 13,3 ms | 7,4 ms | 18,7 ms |
| Neuen Artikel öffnen | 0,3 ms | 0,6 ms | 0,7 ms | 0,4 ms | 2,1 ms | 0,2 ms | 0,5 ms |
| Artikel beliebigen Alters öffnen | 0,3 ms | 0,5 ms | 0,7 ms | 0,5 ms | 1,5 ms | 0,2 ms | 0,4 ms |
| Versionen eines Artikels beliebigen Alters | 1,7 ms | 3,3 ms | 7,0 ms | 3,4 ms | 15,4 ms | 1,5 ms | 6,2 ms |
| Einen Artikel als gelesen markieren | 0,7 ms | 1,2 ms | 1,5 ms | 0,7 ms | 1,8 ms | 0,7 ms | 2,0 ms |
| Einen Feed als gelesen markieren | 1,0 ms | 1,9 ms | 2,2 ms | 1,0 ms | 2,6 ms | 1,1 ms | 4,3 ms |

Die ersten drei Spalten sind 10 Millionen Artikel mit warmem Cache. „Nach Neustart“ ist dieselbe Datenbank nach einem Neustart von PostgreSQL mit geleertem Cache des Betriebssystems. Einzelne Läufe lagen über dem Ziel, wenn eine Seite erst von der Platte kam: Der langsamste von 21.000 Läufen brauchte 139 ms.

**Zehnfache Artikelzahl:** 100 Millionen Artikel brauchen rund 180 GB, die Messmaschine hat 30 GB Platte. Statt nach oben wurde deshalb nach unten verglichen: Dieselben Abfragen brauchen bei 1 Million und bei 10 Millionen Artikeln gleich lang (die beiden rechten Spalten gegen die linken). Die Abfragen lesen je Feed und je Zeile der Seite eine feste Zahl von Indexseiten, die Tiefe des Index wächst mit dem Zehnfachen um höchstens eine Ebene. Mehr Daten hat die alte Produktion nicht: 707.040 Artikel, siehe [Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15).

Größe auf der Platte bei 10 Millionen Artikeln, zusammen 18 GB:

| Tabelle | Daten | Indizes | Anmerkung |
|---|---:|---:|---|
| `articles` (heiß) | 4,2 GB | 0,6 GB | Rund 420 Byte je Artikel, davon Titel, Anriss und zwei Adressen. Listen lesen daraus nur die neuesten Seiten je Feed. |
| `article_content` (warm) | 7,7 GB | 1,3 GB | Der Index auf die Kennung im Feed, den der Abruf braucht, macht 1,1 GB aus. |
| `article_versions` (kalt) | 4,5 GB | (enthalten) | 10,6 Millionen Versionen in 124 Monatspartitionen. |
| `article_states` | 9 MB | 9 MB | 101.674 Ausnahmen für 1.893 Abos. |

Die erzeugten Texte sind Zufallszeichen und lassen sich kaum komprimieren. Echtes HTML wird in `article_content` (lz4) und `article_versions` (zstd) deutlich kleiner; die Werte für diese beiden Tabellen sind also eine Obergrenze für diese Textmenge.

### Auswertung

- **Die Listenabfrage wurde umgebaut, weil sie das Ziel verfehlte.** Die erste Fassung holte je Feed die neuesten Artikel als Stichprobe, leitete daraus eine Untergrenze ab und las alle Feeds bis dorthin. Hatten nur wenige Feeds etwas zu zeigen, gab es keine Untergrenze, und sie las aus jedem eine ganze Seite. „Alle Feeds, nur ungelesen, tief in der Liste“ brauchte so 31 ms im Median, 66 ms im 95. und 89 ms im 99. Perzentil. Die jetzige Fassung führt die Feeds zusammen wie sortierte Listen: ein Schritt in den Index je Feed für dessen neuesten passenden Artikel, dann je Zeile der Seite ein weiterer Schritt für den Nachfolger im selben Feed. Der Aufwand hängt nur noch von der Zahl der Feeds und der Länge der Seite ab.
- **Der Plan** (`EXPLAIN (ANALYZE, BUFFERS)`, alle 300 Feeds, nur ungelesen): Die Abos des Kontos kommen über ihren Primärschlüssel, die Lesemarke wird je Feed über `articles_pkey` in eine Stelle der Ankunftsreihenfolge übersetzt. Darauf folgt eine rekursive Vereinigung: je Feed und je Zeile ein rückwärts laufender Index-Scan auf `articles_arrival`, der nach dem ersten Treffer endet, und je gelesenem Artikel ein Schritt in `article_states_pkey`. Die 50 Zeilen der Seite kommen am Ende über `articles_arrival` aus der Tabelle. Es gibt keinen Scan einer ganzen Tabelle und keine Sortierung über mehr als die Köpfe der Feeds. Eine Seite über alle 300 Feeds fasst rund 3.700 Puffer an, eine Seite eines Feeds rund 700, alle aus dem Cache.
- **Planen kostete mehr als Ausführen.** PostgreSQL plante die Listenabfrage bei jedem Aufruf neu, rund 3 ms. Die Abfrage läuft jetzt mit einem festen Plan je Verbindung (`plan_cache_mode = force_generic_plan`, nur für diese Abfrage gesetzt). Bitmap-Scans sind für sie abgeschaltet, weil der Planer nicht weiß, wie wenige Zeilen ein Schritt liest.
- **Der Lesestand wird je Zeile nachgeschlagen**, nicht mit allen Zuständen des Kontos verbunden. Für ein Konto mit zehntausenden Sternen und ausgeblendeten Artikeln kostet eine Seite deshalb nicht mehr als für jedes andere.
- **Ungelesen-Zahlen** brauchen 7 ms für alle Feeds eines Kontos und werden aus Lesemarke und Ausnahmen berechnet, wie geplant. Verdichten hält die Ausnahmen klein: Je Abo sind es im Schnitt 54.
- **Alte Versionen stören nicht.** Das Öffnen eines Artikels liest nur `articles` und `article_content` über ihre Primärschlüssel und ist bei jedem Alter gleich schnell.
- **Nicht gemessen:** der Index für die Volltextsuche; das kam mit Paket 2.13, siehe [Suche vom 2026-11-11](#suche-vom-2026-11-11). An echten Daten siehe [Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15).
- **Reserve, falls es später eng wird:** Die Übersetzung der Lesemarke und die wieder ungelesenen Artikel unter der Marke kosten je Feed einen Zugriff auf die Tabelle. Ein Index auf `(feed_id, seq)`, der die ID mitführt, würde ihn sparen. Bei den gemessenen Zeiten lohnt das den größeren Index nicht.

### Soak-Test vom 2026-11-11

Der [Soak-Test](architecture.md#soak-test) eine Stunde lang: `SOAK_SECONDS=3600 cargo test --release --test soak -- --nocapture`, auf dem Stand von main nach Pull Request 25.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher, PostgreSQL 16.15 mit Standardwerten (`shared_buffers = 128MB`). Server, Datenbank, Leser und die Quelle der Feeds auf derselben Maschine, sonst lief nichts.
- **Daten:** 6 Konten, 24 Feeds, je Konto 16 Abos in zwei Ordnern. Die Feeds wuchsen von 240 auf 2.898 Artikel; jeder fünfte Artikel bekam alle anderthalb Minuten einen neuen Text, bis an die Grenze von 20 Versionen.
- **Ergebnis:** bestanden. 389.684 Anfragen ohne eine unerwartete Antwort, im Mittel 108 je Sekunde, dazu 10.624 Bilder über den Proxy und 55.995 Ereignisse an die offenen Tabs. Die Ungelesen-Zahlen stimmten am Ende mit den Listen überein, im Log stand kein Fehler, und der Server endete auf `SIGTERM` sauber.

| Nach | Speicher | Threads | Offene Dateien | Verbindungen | Anfragen in 30 s | Median | p99 | Artikel |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 30 s | 147,2 MB | 11 | 29 | 9 | 3.410 | 2,0 ms | 19,4 ms | 250 |
| 10 min | 149,4 MB | 11 | 29 | 9 | 3.342 | 2,1 ms | 27,6 ms | 678 |
| 20 min | 150,0 MB | 11 | 30 | 9 | 3.245 | 2,4 ms | 49,5 ms | 1.127 |
| 30 min | 150,3 MB | 11 | 28 | 7 | 3.230 | 2,3 ms | 72,4 ms | 1.574 |
| 40 min | 150,4 MB | 11 | 30 | 9 | 3.236 | 2,0 ms | 76,4 ms | 2.024 |
| 50 min | 150,5 MB | 11 | 30 | 10 | 3.210 | 2,2 ms | 78,9 ms | 2.472 |
| 60 min | 150,6 MB | 11 | 31 | 10 | 3.160 | 2,2 ms | 101,1 ms | 2.894 |

- **Was der Server hält, bleibt stehen:** Der Speicher wuchs nach den ersten Minuten in einer Stunde um 1,2 MB, Threads und offene Dateien blieben gleich, die Verbindungen zur Datenbank blieben im Pool von zehn.
- **Der Median bleibt, das 99. Perzentil steigt** mit dem Bestand, von 19 ms auf rund 100 ms. Die langsamsten Anfragen waren alle `POST /api/rules` mit 270 bis 380 ms: Eine neue Regel prüft die neuesten 200 Artikel jedes Feeds und alles Ausgeblendete (siehe [Filterregeln](architecture.md#filterregeln)), und das wird mit jedem Artikel mehr. Welche Anfragen sonst das 99. Perzentil heben, hält der Test nicht fest. Für die Regeln ist das bei dieser Menge noch kein Problem, aber die Stelle, an der es sich zuerst zeigen würde.
- Ein erster Lauf auf dem Stand vor dem Merge von Pull Request 24 zeigte in 16 Minuten dasselbe Bild, bevor er für den neuen Stand abgebrochen wurde.

### Fuzzing vom 2026-11-12

Die drei Fuzz-Targets aus Paket 2.11 zum ersten Security-Review, auf dem Stand von Paket 2.12 (main nach Pull Request #39 mit den Änderungen des Reviews): `cargo +nightly fuzz run <target> -- -max_total_time=1800 -rss_limit_mb=2048`, ausgehend vom gespeicherten Korpus.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,3 GHz), 15 GB Arbeitsspeicher; die drei Targets liefen zugleich, je eines auf einem Kern. Nightly-Rust vom 2026-11-06.
- **Erster Lauf:** Bereinigung 102.858 Eingaben in einer halben Stunde ohne Fund, höchstens 621 MB; Bild-Proxy 446.585 Eingaben ohne Fund, höchstens 501 MB. Feeds brach nach etwa 16.000 Eingaben mit einer Panic in `quick-xml` ab, die `feed-rs` auslöste: Befund S7, ein Byte, das kein UTF-8 ist, mitten in der Deklaration. Behoben und als `found-broken-declaration` im Korpus.
- **Zweiter Lauf, nur Feeds, mit der Behebung:** 199.270 Eingaben in einer halben Stunde ohne Fund, höchstens 712 MB. In derselben Zeit lief einmal `cargo test`, die Zahl der Eingaben ist deshalb etwas kleiner als ohne.
- Was der Fuzzer an neuer Abdeckung fand, ist nicht in den gespeicherten Korpus gekommen; der bleibt beim Stand von Paket 2.11 und dem neuen Fund.

### Fuzzing vom 2026-11-14

Die vier Fuzz-Targets (`greader`, `cleaning`, `feed`, `image`) zum zweiten Security-Review, auf dem Stand von `main` nach Pull Request #100 (`10bb547`): der Workflow Fuzz, von Fiona von Hand mit 1800 Sekunden gestartet (Lauf 43633311236), mit den Grenzen dort (`-timeout=10 -rss_limit_mb=2048 -max_len=131072`), ausgehend vom gespeicherten Korpus.

- **Maschine:** je Target ein Runner `ubuntu-latest` von GitHub, alle vier zugleich, Nightly-Rust vom Tag des Laufs.
- **Ergebnis:** alle vier je eine halbe Stunde ohne Fund; mit Bauen dauerte jeder Job rund 41 bis 44 Minuten. Der Workflow sagt nicht, wie viele Eingaben es waren.

### Suche vom 2026-11-11

Gemessen für Paket 2.13, bevor und während die [Suche](architecture.md#suche) gebaut wurde: was der Index kostet und wie schnell sie ist. Ergebnis: Der Index ist tragbar, die Suche ist gebaut.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,3 GHz), 15 GB Arbeitsspeicher, virtuelle Platte. PostgreSQL 16.15 mit `shared_buffers = 2GB`, sonst Standardwerte. Server und Datenbank auf derselben Maschine.
- **Daten:** `rueder bench generate` wie unter [Ergebnisse vom 2026-11-10](#ergebnisse-vom-2026-11-10), 10.000.000 Artikel, aber mit Wörtern statt Zufallszeichen, weil Größe und Tempo eines Index der Wörter an ihnen hängen. Die Wörter sind aus 32 Silben gebaut, 1 Million verschiedene, ihre Häufigkeit folgt einer Zipf-Verteilung wie in echter Sprache; Titel und Texte sind Sätze aus je 9 Wörtern. Ein Text hat im Schnitt 544 Zeichen, ein Titel 62. Echte Artikel sind oft länger, das macht das Ansehen eines Artikels teurer, nicht aber das Fragen des Index. Das Erzeugen dauert 48 Minuten: 16 für die Artikel, 30 für den Index der Suche, der wie bei einer Migration auf den fertigen Bestand gebaut wird.
- **Messung:** `rueder bench measure` mit sieben Arten von Suchen, je 500-mal mit wechselnden Konten und Wörtern, in allen Feeds eines Kontos, Seiten zu 50 Treffern. Ein Wort „sehr häufig“ steht in rund 40 % der Artikel, „häufig“ in rund 10 %, „seltener“ in 1.000 bis 10.000 Artikeln, „selten“ in einigen hundert; „zwei Wörter“ ist ein häufiges und ein selteneres, zusammen in rund 200 Artikeln. „Tief“ beginnt an einer zufälligen Stelle der zehn Jahre. Die Zeit ist die einer Anfrage mit allen Schritten bis zur vollen Seite, längstens bis zur halben Sekunde, nach der die Suche mit `next` antwortet.

**Größe.** Der Index braucht 2,9 GB, rund 300 Byte je Artikel; die ganze Datenbank hat 21 GB, der Index ist also ein Siebtel davon. Dazu kommen die Kopie von Titel, Autor und ID in `article_content`, die der Index liest (im Schnitt rund 80 Byte je Artikel), und ein Index auf diese ID mit 216 MB. Eine eigene Spalte mit den Wörtern wäre größer als der Text gewesen und wurde deshalb nicht gebaut (siehe [Suche](architecture.md#suche)).

**Schreiben.** Je 20 neue Artikel eines Feeds in einer Transaktion, wie der Abruf sie schreibt. `bench measure` misst das 500-mal über den Weg des Programms, mit dem Index und ohne ihn (den es dafür in einer Transaktion fallen lässt); `pgbench` misst dieselbe Menge 400-mal direkt an der Datenbank, um die beiden Arten des Index zu vergleichen:

| Messung | Median | p95 | p99 | Max |
|---|---:|---:|---:|---:|
| `bench`, ohne Index der Suche, warm (2 Läufe) | 8 bis 11 ms | 52 bis 56 ms | 59 bis 65 ms | 72 bis 113 ms |
| `bench`, Index mit Warteliste, warm (2 Läufe) | 11 bis 15 ms | 56 bis 63 ms | 76 bis 111 ms | 2,3 bis 3,9 s |
| `pgbench`, Index mit Warteliste, warm (2 Läufe) | 5 bis 6 ms | 15 bis 25 ms | 76 bis 135 ms | 2,0 bis 13,4 s |
| `pgbench`, Index ohne Warteliste, warm | 30 ms | 57 ms | 82 ms | 257 ms |
| `bench`, ohne Index der Suche, nach Neustart | 4 ms | 49 ms | 51 ms | 74 ms |
| `bench`, Index ohne Warteliste, nach Neustart | 21 ms | 179 ms | 485 ms | 1,2 s |

Mit der Warteliste, dem Standard von PostgreSQL, sammelt der Index neue Wörter erst und arbeitet sie in einem Rutsch ein, sobald es 4 MB sind. Das macht das Schreiben meist billig, aber das Einarbeiten ließ einzelne Abrufe Sekunden warten, in einem Lauf direkt nach einem Neustart sogar 42 Sekunden, und jede Suche über den Index muss die ganze Liste lesen. Der Index wird deshalb ohne Warteliste angelegt: Warm kostet ein Artikel beim Schreiben rund 1,5 ms mehr, gleichmäßig; nach einem Neustart, solange die Seiten des Index von der Platte kommen, im 99. Perzentil eine halbe Sekunde je 20 Artikel. Für den Abruf, der im Hintergrund schreibt und keinen Leser warten lässt, ist das tragbar.

**Suchen.** Die erste Fassung ließ den Planer von PostgreSQL wählen, ob er den Index fragt oder die Artikel einzeln ansieht. Die zweite wählt selbst, wie unter [Suche](architecture.md#suche) beschrieben, und fragt bei mehreren Wörtern den Index nur nach dem seltensten. Zeiten mit warmem Cache:

| Suche | Planer: Median | Planer: p99 | Planer: Max | Gebaut: Median | Gebaut: p95 | Gebaut: p99 | Gebaut: Max |
|---|---:|---:|---:|---:|---:|---:|---:|
| Ein sehr häufiges Wort | 6 ms | 11 ms | 101 ms | 29 ms | 40 ms | 46 ms | 54 ms |
| Ein häufiges Wort | 492 ms | 798 ms | 852 ms | 67 ms | 124 ms | 156 ms | 187 ms |
| Ein selteneres Wort | 32 ms | 91 ms | 3,06 s | 67 ms | 108 ms | 128 ms | 148 ms |
| Ein seltenes Wort | 70 ms | 3,07 s | 3,09 s | 97 ms | 119 ms | 143 ms | 209 ms |
| Zwei Wörter | 563 ms | 1,34 s | 1,84 s | 165 ms | 243 ms | 423 ms | 502 ms |
| Ein häufiges Wort, tief | 496 ms | 769 ms | 824 ms | 89 ms | 239 ms | 321 ms | 3,42 s |
| Ein selteneres Wort, nur ungelesen | 81 ms | 175 ms | 197 ms | 136 ms | 304 ms | 452 ms | 2,31 s |

Nach einem Neustart von PostgreSQL mit geleertem Cache des Betriebssystems, mit der gebauten Fassung:

| Suche | Median | p95 | p99 | Max |
|---|---:|---:|---:|---:|
| Ein sehr häufiges Wort | 29 ms | 42 ms | 46 ms | 320 ms |
| Ein häufiges Wort | 67 ms | 124 ms | 166 ms | 257 ms |
| Ein selteneres Wort | 76 ms | 112 ms | 127 ms | 200 ms |
| Ein seltenes Wort | 106 ms | 131 ms | 162 ms | 223 ms |
| Zwei Wörter | 184 ms | 260 ms | 332 ms | 413 ms |
| Ein häufiges Wort, tief | 77 ms | 159 ms | 198 ms | 224 ms |
| Ein selteneres Wort, nur ungelesen | 130 ms | 220 ms | 263 ms | 7,4 s |

Die Listen blieben in beiden Läufen bei den Zeiten unter [Ergebnisse vom 2026-11-10](#ergebnisse-vom-2026-11-10), im 99. Perzentil unter 80 ms. Nach dem Neustart waren aber die Abfragen langsamer, die alte Artikel von der Platte lesen: Versionen eines Artikels beliebigen Alters im Median 72 ms statt 3 ms, das Öffnen im 99. Perzentil 63 ms statt 2 ms. Vermutlich, weil die Datenbank mit Index und Wörtern jetzt 21 GB hat und weniger davon in die 15 GB Arbeitsspeicher passt; mit warmem Cache war davon nichts zu sehen. Die echten Daten passen mit 1,9 GB ganz in den Arbeitsspeicher ([Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15)).

- **Warum der Planer daneben lag.** Ein häufiges Wort ist am schnellsten gefunden, wenn man rückwärts durch die neuesten Artikel geht: Jeder zehnte ist ein Treffer, nach 500 Artikeln ist die Seite voll. Der Planer nahm stattdessen oft den Index, der für das Wort eine Million Artikel lesen musste, auch wenn nur die im Fenster der neuesten 20.000 gebraucht wurden: eine halbe Sekunde. Bei seltenen Wörtern lag er manchmal andersherum und ging rückwärts durch ein Fenster von Millionen Artikeln, bis die Grenze von drei Sekunden die Suche abbrach (14 von 3.500 Suchen). Beides kommt von seiner Schätzung, wie viele Artikel ein Wort und seine Verlängerungen haben; mit höheren Kosten für den Index (`cpu_index_tuple_cost`) verschob sich der Fehler nur von der einen Seite auf die andere.
- **Was die eigene Wahl kostet.** Ein sehr häufiges Wort ist langsamer als beim Planer (29 statt 6 ms), weil die Suche immer erst 500 Artikel ansieht, bevor sie entscheidet. Dafür bleibt jede Art von Suche im 99. Perzentil unter einer halben Sekunde. Die zwei Ausreißer über zwei Sekunden mit warmem Cache kamen in einem Lauf, kurz nachdem PostgreSQL nach einem Neustart des Containers den Absturz aufgeräumt hatte, und im selben Lauf hatte auch das Schreiben Ausreißer. Im Lauf davor, der bei mehreren Wörtern noch alle im Index fragte, blieb jede Suche mit einem Wort unter 350 ms, und nach dem Neustart gab es einen Ausreißer von 7,4 s, als die Seiten noch von der Platte kamen. Die Kosten je Artikel misst die Suche selbst, die je Artikel im Index (1,3 µs) sind in `server/src/search.rs` eingetragen und stammen aus dieser Messung.
- **Nicht gemessen:** Suchen in einem einzelnen Feed oder Ordner und mit langen Texten. Die Wahl rechnet mit beidem (wie weit die Artikel der Suche auseinanderliegen und was einer kostet), geprüft ist das aber nur mit dem Modell-Test in `server/tests/search.rs` und kleinen Versuchen von Hand. Mit echten, längeren Texten siehe [Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15).

### Abruf vom 2026-11-12

Für Paket 5.2: wie viele Abrufe je Stunde eine Maschine bei einem gegebenen Takt schafft. `rueder bench fetch` auf einer eigenen Datenbank, im Release-Build, je Lauf fünf Minuten (der Standardtakt zwei).

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher, PostgreSQL 16.15 mit `shared_buffers = 2GB`. Server, Datenbank und Quelle auf derselben Maschine.
- **Daten:** Erfundene RSS-Feeds mit je 20 Artikeln zu Beginn und einem neuen je Minute, jeder mit einem Absatz, einem Bild und einem Link mit Tracking-Parameter. Ein Feed ohne Neues antwortet mit „nicht geändert“ (ETag). Vor jeder Runde werden alle Feeds wieder fällig, so dass jede Runde nimmt, so viel sie darf.

| Takt | Feeds, Namen | Runde im Mittel, längste | Abrufe (mit Inhalt, unverändert) | Neue Artikel | Abruf Median, 95. Perzentil | Je Stunde |
|---|---|---|---|---|---|---|
| 100 alle 30 s, 8 gleichzeitig (Standard) | 2.000, 250 | 0,27 s, 0,53 s | 500 (407, 93) | 5.405 | 20 ms, 51 ms | 12.000 |
| 1.000 alle 10 s, 64 gleichzeitig | 5.000, 250 | 1,1 s, 3,4 s | 31.000 (16.536, 14.464) | 117.097 | 41 ms, 163 ms | 360.000 |
| 10.000 alle 5 s, 256 gleichzeitig | 20.000, 1.000 | 4,7 s, 28 s | 490.000 (94.478, 395.522) | 476.275 | 81 ms, 258 ms | 5,9 Millionen |

- **Der Takt begrenzt, nicht die Maschine,** bis weit über das, was eine Instanz braucht: Beim Standard ist eine Runde nach einer Viertelsekunde von 30 fertig, bei 1.000 alle 10 Sekunden nach gut einer Sekunde.
- **Teuer ist der erste Abruf.** Die längste Runde des größten Takts war die erste: 10.000 Feeds, die zum ersten Mal je 20 Artikel bringen, in 28 Sekunden, rund 350 Feeds mit Inhalt je Sekunde. Danach antworten die meisten Feeds „nicht geändert“, und eine Runde dauert 3 bis 5 Sekunden.
- **Echte Websites antworten langsamer** als eine Quelle auf derselben Maschine. Dann begrenzt `FETCH_AT_ONCE`: 64 gleichzeitig bei einer halben Sekunde je Abruf sind rund 130 Abrufe je Sekunde. Die Werte für eine große Instanz in [deploy/README.md](../deploy/README.md#a-large-instance) rechnen damit.
- Ein erster Lauf scheiterte an einem Deadlock: Die Messung hatte, anders als der Server beim Start, die Partition des Monats für `article_versions` nicht angelegt, und mehrere Abrufe legten sie zugleich an. Sie legt sie jetzt vorher an wie der Server.

### Import vom 2026-11-12

Das Zusammenführen aus Paket 4.3, gemessen am 2026-11-12 in der Entwicklungsumgebung (4 Kerne, 15 GB Arbeitsspeicher, PostgreSQL 16 auf derselben Maschine, Release-Build), mit erfundenen Daten:

- **Alt:** 300 Feeds mit 300.000 Artikeln, schief verteilt (der größte Feed hat 44.906), Texte von rund 1 KB mit Links, 10 User mit je 300 Abos und einem Ordner. Die Datenbank hat 346 MB.
- **Neu:** 5 Konten mit den Namen von 5 alten Usern, 150 Feeds mit den Adressen von 150 alten, je 50 eigene Artikel, alle 5 Konten haben alle 150 Feeds abonniert und je 25 Artikel gelesen.

| Lauf | Dauer |
|---|---|
| `--dry-run` | unter 1 s |
| Import: 150 Feeds zusammengeführt, 150 neu, 300.000 Artikel, 5 Konten zusammengeführt, 5 neu, 2.250 Abos | 171 s, rund 1.750 Artikel je Sekunde |
| Zweiter Lauf ohne Neues | 1 s |

Danach hat jeder Feed die Nummern 1 bis `last_seq` ohne Lücke, in jedem Feed steigt die ID mit der Nummer, jedes alte Konto hat in den zusammengeführten Feeds seine 25 ungelesenen eigenen Artikel, und die neuen Konten haben die 50 eigenen Artikel je Feed ungelesen, die alten gelesen. Die neue Datenbank hat danach 660 MB.

Langsamer als 2026-11-10 (4.300 je Sekunde), weil seither jeder Text bereinigt wird und der Index der Suche mitwächst. Hochgerechnet bräuchten 10 Millionen Artikel rund anderthalb Stunden. Gelesen werden die alten Artikel jetzt Feed für Feed statt in der Reihenfolge des Eintreffens; bei einer Datenbank, die nicht in den Arbeitsspeicher passt, kann das mehr von der Platte lesen. An echten Daten waren es rund 1.470 je Sekunde ([Probelauf vom 2026-11-15](#probelauf-vom-2026-11-15)).

### Design-Review vom 2026-11-14

Touch-Ziele und Zeilenlänge, gemessen vom Rundgang der Screenshots (`pnpm screenshots`, `measure.md`) mit den erfundenen Feeds der Screenshots (14 Feeds, rund 470 Artikel) in Chromium von Playwright als Pixel 7 (`pointer: coarse`) und am Desktop mit 1440×900, im Container der Sitzung mit Claude; die Farben aus `layout.css`. Die Tabellen stehen in [design_review.md](design_review.md#messwerte).

| Messung | Vorher | Nachher |
|---|---|---|
| Ziele unter 44×44, en-phone-light (67 Bilder) | 176 | 0 |
| Ziele unter 24×24 ohne den Abstand nach 2.5.8, en-desktop-light (56 Bilder) | nicht gemessen | 0 (73 kleinere mit Abstand) |
| Längste Zeile Artikeltext, Desktop, Schrift „Mittel“ | 84 Zeichen (`44rem`), mit `66ch` 86 | 80 Zeichen (`60ch`) |
| Längste Zeile Artikeltext, Handy | – | 46 Zeichen |
| Kontrast aller Themen und Akzente | F8, F10–F12 unter dem Maß | alle 37 Fälle von `colors.test.ts` erfüllt |

| Thema | Wo |
|---|---|
| Schema der alten Datenbank | `backend/migrations/*.up.fizz`, Modelle in `backend/pkg/repository/pop/models/` |
| Artikelliste mit Fensterfunktion (die langsame Abfrage) | `backend/pkg/repository/pop/api/api.go` |
| Ordner speichern, Grenzen je User | `backend/pkg/repository/pop/api/api_user.go` |
| Abruf, Intervall nach Aktivität, Sortierung der Artikel beim Einfügen | `backend/pkg/worker/feed.go` |
| Planer und Warteschlange des Abrufs | `backend/pkg/worker/scheduler/` |
| Bereinigung im Backend (Tags) | `backend/pkg/worker/feed.go` (`htmlsanitizer`) |
| Werbe- und Tracker-Filter mit Tests | `frontend/src/rueder/helpers/ArticleCleaner.ts`, `ArticleCleaner.test.ts` |
| Adressen für den Bild-Proxy | `frontend/src/rueder/helpers/ImageProxy.ts` |
| Feed-Suche auf Webseiten | `backend/pkg/feedfinder/` |
| OPML | `frontend/src/rueder/helpers/opml.ts`, `frontend/src/rueder/components/Sidebar/OPMLExport.svelte` |
| Lesestand im Browser | `frontend/src/rueder/stores/articlestate.ts`, beschrieben in [user_data.md](user_data.md) |
| Labels im Browser | `frontend/src/rueder/stores/labels.ts` |
| Login mit loginsrv und JWT | `backend/cmd/authbackend`, `backend/pkg/fibertools/`, `config/rueder.example.env` |
| Betrieb des alten rueder | `docker-compose.yaml`, `config/` |

### Captcha vom 2026-11-15

Wie lange die Seite für das [Captcha](architecture.md#captcha) rechnet, gemessen mit der Schleife aus `web/src/lib/captcha.ts` in Chromium (Playwright, ohne Fenster) auf dem Cloud-Container mit 4 Kernen, über alle 100.001 Zahlen bis `captcha::MAX_NUMBER`, also die längste Wartezeit; im Mittel ist es die Hälfte.

| Browser | Längste Wartezeit |
|---|---|
| Ungebremst | 0,5 s |
| CPU viermal gebremst, als Ersatz für ein Telefon | 2,1 bis 2,3 s |

Auf einem echten Telefon ist es nicht gemessen. Bündel von 1.000 Hashes auf einmal waren nicht schneller als einer nach dem anderen.

Eigener Code dagegen, gemessen am 2026-11-15 auf demselben Container mit `openssl speed -bytes 64 sha256` auf einem Kern: rund 4,4 Millionen Hashes je Sekunde, also im Mittel etwa 11 ms je Aufgabe und 23 ms für alle 100.001 Zahlen; Python mit `hashlib` schafft rund 2 Millionen je Sekunde. Eigener Code ist damit etwa zwanzigmal so schnell wie die Seite, je Kern.

### Viele Konten vom 2026-11-14 und 2026-11-15

Für die Frage, was rueder für 10.000 und 100.000 registrierte Konten braucht, und als Ausgangslage für die Pakete 5.31 bis 5.36. Am 2026-11-14 mit Skripten im Scratchpad gemessen, am 2026-11-15 mit `rueder bench readers` und `web/load/` aus Paket 5.30 wiederholt; die Tabellen sind die vom 2026-11-15, die Werte vom Vortag lagen gleich.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher, virtuelle Platte. PostgreSQL 16.15 mit `shared_buffers = 2GB`, sonst Standardwerte, Release-Build. Server, Datenbank und Lastgenerator (Node 22) auf derselben Maschine; der Lastgenerator braucht davon rund einen Kern.
- **Lesen:** `rueder bench generate --accounts 10 --feeds 20000 --articles 3000000`, dazu `rueder bench readers --count 100000`: 100.000 Leser mit 4.915.319 Abos, im Schnitt 49, höchstens 470, der beliebteste Feed mit 27.279 Lesern. Die Datenbank hat 8,8 GB. Ein Besuch ist, was die Seite tut: Baum, erste Seite der ungelesenen Artikel aller Feeds, drei davon öffnen und als gelesen markieren, eine Seite eines Feeds, zusammen 9 Anfragen. Gemessen nach `VACUUM ANALYZE` und 90 Sekunden zum Aufwärmen, je 30 Sekunden.
- **Live-Updates:** eine leere Datenbank mit `rueder bench readers --count N --shared …`, alle lesen denselben Feed aus `load/feed.js`. Jeder Leser hält eine Verbindung zu `/api/events`. Dann ein neuer Artikel in diesem Feed, dann je 20 Sekunden lang Änderungen von 50 Lesern (ein Artikel gelesen und wieder ungelesen), jede ein Ereignis nur für ihren Leser.

Lesen mit 100.000 Lesern:

| Zugleich | Anfragen je s | Besuche je s | Server | PostgreSQL | Baum p99 | Ungelesen p99 | Artikel p99 | Markieren p99 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 8 | 1.452 | 161 | 0,9 Kerne | 1,8 Kerne | 13 ms | 19 ms | 9 ms | 13 ms |
| 32 | 1.635 | 182 | 1,0 | 1,9 | 39 ms | 41 ms | 30 ms | 35 ms |
| 64 | 1.645 | 183 | 1,0 | 1,9 | 81 ms | 75 ms | 57 ms | 58 ms |

Live-Updates:

| Verbindungen | Speicher je Verbindung | Neuer Artikel bei allen | 50 Änderungen je s | 200 je s | 800 je s |
|---:|---:|---:|---|---|---|
| 2.000 | 24,5 KB | in 44 ms | 0,8 Kerne | 2,3 Kerne | 763 je s geschafft, p50 1,0 s |
| 5.000 | 23,3 KB | in 71 ms | 1,8 Kerne | 2,6 Kerne | 512 je s, p50 6,0 s, 2.083 `resync` |
| 10.000 | 22,8 KB | in 141 ms | 2,7 Kerne | 2,7 Kerne | 313 je s, p50 15,6 s, 4.659 `resync` |

- **Lesen ist nicht die Grenze.** Die Anfragen enden bei rund 1.650 je Sekunde, weil die Kerne dann zwischen Server, PostgreSQL und Lastgenerator aufgeteilt sind; mehr Verbindungen zur Datenbank bringen auf dieser Maschine nichts ([Verbindungen zur Datenbank](#verbindungen-zur-datenbank-vom-2026-11-19)). Bei 100.000 Konten mit 20 bis 30 % täglich aktiven und einer Spitze von 15 bis 35 Besuchen je Sekunde ist das ein Zehntel davon. Am Vortag mit im Schnitt 43 Abos ohne lange Ausreißer waren es 2.050 je Sekunde.
- **Live-Updates sind die Grenze.** Jedes Ereignis geht an jede offene Verbindung, und jede sieht selbst in der Liste der Konten nach, ob es ihr gilt (`events.rs`). Mit 10 Verbindungen kosteten 800 Änderungen je Sekunde am Vortag 0,3 Kerne; mit 10.000 kosten schon 50 je Sekunde 2,7. Kommt eine Verbindung nicht nach, fällt sie aus dem Puffer und bekommt `resync`, und die Seite lädt alles neu. Bei 100.000 Konten wären 3.000 bis 5.000 Verbindungen und um 150 Ereignisse je Sekunde zu erwarten, mehr als diese Maschine schafft. Paket 5.31.
- **Ein großes Konto.** Eines der Konten des Generators mit 12.012 Abos brauchte für den Baum 0,34 s (3,6 MB JSON) und für eine Liste über alle Feeds rund 0,3 s; ein Konto darf aber höchstens 2.000 Abos haben. Die Leser des beliebtesten Feeds zu finden, wie es jeder Abruf mit Neuem tut, kostete bei 26.246 Lesern 67 ms.
- **Kalter Cache.** Nach dem Löschen und Neuanlegen von fünf Millionen Abos, mit `shared_buffers = 128MB`, brauchte eine Seite der ungelesenen Artikel im Median 37 s und die Maschine rechnete fast nichts: Alles kam von der Platte. Was gelesen wird, muss in den Arbeitsspeicher passen; wie viel das ist, steht unter [Platz, Arbeitsspeicher und Abos](#platz-arbeitsspeicher-und-abos-vom-2026-11-19).
- **Nicht gemessen:** Apps über die Google-Reader-API, der Bild-Proxy unter Last, Abrufe gegen echte, langsame Websites und die Suche mit Hunderten Millionen Artikeln.

### Probelauf vom 2026-11-15

Paket 4.4: die alte Datenbank, wie die alte Produktion sie hinterlassen hat, in eine Testinstanz des neuen rueder importiert und danach mit `rueder bench measure` an den echten Daten gemessen. Ergebnis: Der Import dauert acht Minuten, Listen, Artikel und Ungelesen-Zahlen bleiben weit unter dem Ziel von 50 ms. Die Suche ist langsamer als an erfundenen Daten, bleibt aber in ihrer halben Sekunde.

- **Maschine:** virtuelle Maschine (libvirt/KVM) mit 4 Kernen (Intel Xeon E3-1225 v3, 3,2 GHz), 10 GB Arbeitsspeicher und NVMe-SSD. rueder und PostgreSQL 16 aus `deploy/compose.yaml`, die alte Datenbank daneben in einem eigenen Container mit PostgreSQL 13 auf ihrem alten Volume ([deploy/README.md](../deploy/README.md#bringing-over-the-old-rueder)). Während `bench measure` lief der Server und rief Feeds ab.
- **Alt:** 143 Feeds mit 707.040 Artikeln und 4 User.
- **Neu davor:** 2 Konten mit eigenen Feeds, zwei davon mit der Adresse eines alten Feeds.
- **Texte**, an jedem zehnten Artikel: Ein Zehntel hat keinen Text, der Median hat 130 Zeichen HTML (107 ohne Tags), 90 % haben bis 1.223, 99 % bis 8.099, im Mittel sind es 654, der längste hat 135.707. Ein Titel hat im Median 67 Zeichen.
- **Konten danach:** Ihre Feeds haben 72 %, 23 %, 1,7 % und 0,08 % aller Artikel.

| Lauf | Dauer |
|---|---|
| Import: 2 Feeds zusammengeführt, 141 neu, 707.040 Artikel, 2 Konten zusammengeführt (mit `--account`), 2 neu, 17 Ordner, 169 Abos | 480 s, rund 1.470 Artikel je Sekunde |

Die Zeit geht an wenige große Feeds: Fünf Feeds brauchten zusammen 324 der 480 Sekunden. Je Artikel ist das etwas langsamer als an erfundenen Daten am 2026-11-12 (1.750 je Sekunde), vermutlich weil die echten Texte länger sind. In den beiden zusammengeführten Feeds hatte der neue rueder keinen der alten Artikel schon abgerufen.

Danach hat die Datenbank 710.030 Artikel in 167 Feeds und 1,9 GB:

| Tabelle | Daten | Indizes | Anmerkung |
|---|---:|---:|---|
| `articles` | 354 MB | 39 MB | Rund 500 Byte je Artikel. |
| `article_content` | 587 MB | 292 MB | Rund 840 Byte je Artikel, mit lz4 gepackt. Der Index der Suche hat 178 MB, rund 250 Byte je Artikel. |
| `article_versions` | 447 MB | (enthalten) | |
| `legacy_articles` | 89 MB | (enthalten) | Die alte UUID und Nummer jedes alten Artikels, für Paket 4.6. |

`bench measure`, je Abfrage 500 Läufe, mit warmem Cache, verglichen mit den 10 Millionen erfundenen Artikeln unter [Ergebnisse vom 2026-11-10](#ergebnisse-vom-2026-11-10) und [Suche vom 2026-11-11](#suche-vom-2026-11-11):

| Abfrage | Median | p95 | p99 | Max | Erfunden: Median | Erfunden: p99 |
|---|---:|---:|---:|---:|---:|---:|
| Ein Feed, erste Seite | 1,4 ms | 2,2 ms | 3,8 ms | 11,7 ms | 1,8 ms | 4,5 ms |
| Ein Feed, nur ungelesen | 0,7 ms | 1,5 ms | 1,8 ms | 2,7 ms | 1,3 ms | 2,5 ms |
| Größter Feed, tief in der Liste | 1,9 ms | 2,7 ms | 3,9 ms | 33,5 ms | 1,9 ms | 3,1 ms |
| Ein Ordner, nur ungelesen | 1,7 ms | 1,9 ms | 3,1 ms | 7,3 ms | 2,6 ms | 3,7 ms |
| Alle Feeds, erste Seite | 2,9 ms | 3,5 ms | 4,4 ms | 7,1 ms | 7,1 ms | 12,9 ms |
| Alle Feeds, nur ungelesen | 2,9 ms | 3,9 ms | 5,0 ms | 14,8 ms | 8,6 ms | 17,9 ms |
| Alle Feeds, tief in der Liste | 4,0 ms | 5,8 ms | 7,5 ms | 44,8 ms | 10,8 ms | 17,5 ms |
| Alle Feeds, nur ungelesen, tief in der Liste | 1,1 ms | 1,4 ms | 2,0 ms | 4,5 ms | 11,4 ms | 15,9 ms |
| Ungelesen-Zahlen aller Feeds eines Kontos | 0,7 ms | 0,9 ms | 1,4 ms | 2,6 ms | 7,1 ms | 14,7 ms |
| Neuen Artikel öffnen | 0,3 ms | 0,6 ms | 1,0 ms | 71,6 ms | 0,3 ms | 0,7 ms |
| Artikel beliebigen Alters öffnen | 0,3 ms | 0,8 ms | 1,2 ms | 1,7 ms | 0,3 ms | 0,7 ms |
| Versionen eines Artikels beliebigen Alters | 0,4 ms | 1,8 ms | 4,9 ms | 14,2 ms | 1,7 ms | 7,0 ms |
| Einen Artikel als gelesen markieren | 1,7 ms | 2,2 ms | 3,6 ms | 5,6 ms | 0,7 ms | 1,5 ms |
| Einen Feed als gelesen markieren | 2,0 ms | 2,3 ms | 3,5 ms | 5,7 ms | 1,0 ms | 2,2 ms |
| 20 neue Artikel eines Feeds speichern | 17,5 ms | 23,4 ms | 29,9 ms | 43,8 ms | | |
| Dasselbe ohne den Index der Suche | 4,1 ms | 6,6 ms | 9,4 ms | 11,4 ms | 8 bis 11 ms | 59 bis 65 ms |
| Suche, ein sehr häufiges Wort | 152 ms | 257 ms | 273 ms | 289 ms | 29 ms | 46 ms |
| Suche, ein häufiges Wort | 219 ms | 282 ms | 316 ms | 499 ms | 67 ms | 156 ms |
| Suche, ein selteneres Wort | 219 ms | 247 ms | 253 ms | 271 ms | 67 ms | 128 ms |
| Suche, ein seltenes Wort | 138 ms | 249 ms | 254 ms | 273 ms | 97 ms | 143 ms |
| Suche, zwei Wörter | 147 ms | 252 ms | 285 ms | 472 ms | 165 ms | 423 ms |
| Suche, ein häufiges Wort, tief | 65 ms | 341 ms | 362 ms | 458 ms | 89 ms | 321 ms |
| Suche, ein selteneres Wort, nur ungelesen | 220 ms | 248 ms | 255 ms | 289 ms | 136 ms | 452 ms |

- **Listen, Artikel und Ungelesen-Zahlen** bleiben im 99. Perzentil unter 8 ms, schneller als an den erfundenen Daten, deren Konten mehr Abos haben. Der langsamste von 6.000 Läufen brauchte 72 ms. Als gelesen markieren ist langsamer als an den erfundenen Daten, aber weit unter dem Ziel; warum, ist nicht untersucht.
- **Speichern:** Der Index der Suche kostet beim Abruf rund 0,7 ms je Artikel, halb so viel wie an den erfundenen Daten.
- **Die Wörter der Suchen waren erfunden.** `bench measure` setzte sie hier noch aus den Silben der erfundenen Daten zusammen (seit Paket 5.37 nimmt es sie aus dem Bestand); in den echten Texten finden drei der sieben Arten keinen Artikel, die anderen zwischen 71 und 9.061. Keine der Zeilen ist deshalb eine Suche nach einem häufigen Wort.
- **Die Suche nach einem Wort braucht bis zu fünfmal so lange wie an den erfundenen Daten**, im Median 140 bis 220 ms; zwei Wörter und die Suche tief in der Liste sind etwa gleich schnell. Das 99. Perzentil bleibt unter 0,4 s, die längste Suche brauchte 0,5 s. Auch ein Wort, das kein Artikel hat, kostet 140 bis 220 ms. Das liegt an den neuesten Artikeln: Sie sind viel länger als die alten, und jede Suche bestimmte zuerst ihre Wörter ([Gespeicherte Wörter vom 2026-11-16](#gespeicherte-wörter-vom-2026-11-16)).
- **Nicht gemessen:** nach einem Neustart mit kaltem Cache. Die Datenbank passt mit 1,9 GB ganz in den Arbeitsspeicher; die langsameren Zugriffe auf alte Artikel nach einem Neustart, die bei 21 GB erfundenen Daten auffielen ([Suche vom 2026-11-11](#suche-vom-2026-11-11)), sind bei dieser Größe nicht zu erwarten.

### Suche vom 2026-11-15

Paket 5.37: erfundene Daten in der Form des alten Bestands aus dem [Probelauf](#probelauf-vom-2026-11-15), um die langsame Suche dort nachzustellen. Ergebnis: Hier braucht die Suche im Median 33 bis 87 ms, seltene Wörter ein Drittel bis ein Viertel der Zeit an den echten Daten. Nachgestellt ist die Langsamkeit damit nicht.

- **Maschine:** Cloud-Container mit 4 virtuellen Kernen (Intel Xeon, 2,3 GHz), 15 GB Arbeitsspeicher, PostgreSQL 16 mit Standardwerten (`shared_buffers = 128MB`), Release-Build. Server und Datenbank auf derselben Maschine.
- **Daten:** `rueder bench generate --accounts 4 --feeds 170 --articles 710000`, 1,6 GB. Die Texte haben die Längen des alten Bestands (unten), mit einem Absatz alle drei Sätze und einem Link alle vier. Ein Konto hat alle Feeds, zwei haben sechs von zehn, das vierte nur die sieben kleinsten mit 0,7 % der Artikel, als Leser mit wenigen Feeds neben Lesern mit vielen.
- **Messung:** `rueder bench measure`, je 500 Läufe, mit warmem Cache. Die Wörter nimmt es aus einer Stichprobe des Bestands selbst, nach dem Anteil der Artikel, in denen sie stehen (`bench::Texts`), damit eine Art von Suche an erfundenen und echten Daten dasselbe heißt. Dazu kommt eine Suche nach einem häufigen Wort für das Konto mit den wenigsten Artikeln.

Länge der Texte in Zeichen HTML, an jedem zehnten Artikel:

| | Ohne Text | Median | 90 % | 95 % | 99 % | Mittel | Längster |
|---|---:|---:|---:|---:|---:|---:|---:|
| Alter Bestand | ein Zehntel | 130 | 1.223 | 2.128 | 8.099 | 654 | 135.707 |
| Erfunden | ein Zehntel | 137 | 1.228 | 2.176 | 8.188 | 654 | 14.013 |

| Suche | Median | p95 | p99 | Max | Artikel mit dem ersten Wort |
|---|---:|---:|---:|---:|---:|
| Ein sehr häufiges Wort | 33 ms | 53 ms | 64 ms | 72 ms | 175.692 |
| Ein häufiges Wort | 66 ms | 109 ms | 119 ms | 148 ms | 44.637 |
| Ein selteneres Wort | 53 ms | 114 ms | 125 ms | 131 ms | 87 |
| Ein seltenes Wort | 46 ms | 103 ms | 120 ms | 138 ms | 52 |
| Zwei Wörter | 87 ms | 134 ms | 159 ms | 186 ms | 34 |
| Ein häufiges Wort, tief | 60 ms | 91 ms | 120 ms | 148 ms | 64.284 |
| Ein selteneres Wort, nur ungelesen | 56 ms | 82 ms | 99 ms | 127 ms | 98 |
| Ein häufiges Wort, Konto mit den wenigsten Artikeln | 79 ms | 111 ms | 130 ms | 147 ms | 58.342 |

- **Listen, Artikel und Ungelesen-Zahlen** bleiben im 99. Perzentil unter 10 ms, wie im Probelauf.
- **Die Suche ist hier schneller als an den echten Daten.** Im Probelauf brauchten Suchen nach seltenen oder fehlenden Wörtern im Median 140 bis 220 ms, hier brauchen seltene Wörter 46 bis 56 ms. Die Länge der Texte ist dieselbe, und auch das Konto mit wenigen Artikeln erklärt den Abstand nicht. Für ein solches Konto die neuesten Artikel Feed für Feed zu holen, statt rückwärts durch alle zu gehen, änderte die Zeiten nicht und wurde deshalb nicht gebaut.
- **Woran es lag:** an den neuesten Artikeln der echten Daten, die viel länger sind als die übrigen; die Stichprobe über den ganzen Bestand sieht sie nicht, weil es wenige sind. Siehe [Gespeicherte Wörter vom 2026-11-16](#gespeicherte-wörter-vom-2026-11-16).

### Live-Updates je Konto vom 2026-11-16

Nach Paket 5.31, wie unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15) gemessen: dieselbe Maschine, `load/live.js` mit `rueder bench readers --shared`, dazu 2.000 Änderungen je Sekunde.

| Verbindungen | Speicher je Verbindung | Neuer Artikel bei allen | 50 Änderungen je s | 200 je s | 800 je s | 2.000 je s |
|---:|---:|---:|---|---|---|---|
| 2.000 | 33,2 KB | in 39 ms | 0,1 Kerne | 0,2 Kerne | 0,7 Kerne | 1,0 Kerne, p99 12 ms |
| 5.000 | 31,8 KB | in 73 ms | 0,1 | 0,2 | 0,6 | 0,9, p99 65 ms |
| 10.000 | 31,2 KB | in 121 ms | 0,1 | 0,2 | 0,6 | 1,0, p99 145 ms |

- **Die Kosten hängen nicht mehr an den Verbindungen.** Bei 10.000 Verbindungen kosten 800 Änderungen je Sekunde 0,6 statt 2,7 Kerne, und kein Lauf sah ein `resync`; vorher war die Maschine ab 200 je Sekunde voll. Was bei 2.000 je Sekunde bleibt, sind die Anfragen selbst und das Schreiben in die Datenbank.
- **Mehr Speicher je Verbindung**, rund 8 KB, weil jedes Konto mit offener Verbindung einen eigenen Puffer hat. Bei 5.000 Verbindungen sind das 40 MB.

### Abruf je Host vom 2026-11-16

Nach Paket 5.32: `rueder bench fetch --feeds 5000 --hosts 250 --seconds 120` mit `FETCH_ROUND=1000`, `FETCH_AT_ONCE=64`, `FETCH_EVERY=10`, dazu mit `--slow 2000` ein weiterer Name mit 2.000 Feeds, der jede Antwort eine halbe Sekunde später gibt. Dieselbe Maschine wie unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15), Release-Build, eine Datenbank für sich. „Vorher“ ist derselbe Lauf mit Runden, die aufeinander warten, wie bis Paket 5.32.

| Lauf | Abrufe je Stunde | davon nicht vom langsamen Namen | vom langsamen Namen in 120 s |
|---|---:|---:|---:|
| ohne langsamen Namen | 359.815 | 359.815 | – |
| mit, vorher | 23.717 | 16.792 | 292 |
| mit, nachher | 357.772 | 350.997 | 226 |
| mit, nachher, `FETCH_PER_HOST=4` | 357.892 | 333.544 | 812 |

- **Vorher hielt der langsame Name alles auf.** Die erste Runde nahm neben den übrigen Feeds 292 langsame und holte sie nacheinander, 151 Sekunden lang; so lange wartete die nächste Runde.
- **Nachher hält er nur sich selbst auf.** Die übrigen Feeds kommen fast so oft wie ohne ihn, der langsame Name einmal je halbe Sekunde. Mit vier zugleich kommt er gut dreieinhalbmal so oft dran.

### Gespeicherte Wörter vom 2026-11-16

Paket 5.38: Die Suche speichert die Wörter jedes Artikels, statt sie bei jeder Suche für die neuesten Artikel neu zu bestimmen (`migrations/0024_search_words.sql`). Ergebnis: An den echten Daten braucht die Suche im Median 9 bis 23 ms statt 30 bis 264 ms, an erfundenen Daten, deren neueste Artikel so lang sind wie die echten, 7 bis 21 ms statt 130 bis 160 ms. Die Wörter brauchen dafür etwa so viel Platz wie die Texte selbst. Entschieden von Fiona am 2026-11-16.

- **Vorher an den echten Daten** (Fionas Testinstanz wie im [Probelauf](#probelauf-vom-2026-11-15), `bench measure` mit Wörtern aus dem Bestand): Die Suche brauchte im Median 116 bis 264 ms, für das Konto mit den wenigsten Artikeln aber 30 ms und tief in der Liste 84 ms. Langsam war also, was bei den neuesten Artikeln der großen Konten anfängt. Die 3.429 Artikel, die seit dem Import abgerufen wurden, haben im Mittel 4.006 Zeichen HTML (90 % bis 10.072), die 707.040 importierten 632, die neuesten 500 im Mittel 1.804. Die Wörter dieser 500 zu bestimmen dauerte 103 ms, und jede Suche tat das zuerst (`search::PROBE`).
- **Maschine für die erfundenen Daten:** wie unter [Suche vom 2026-11-15](#suche-vom-2026-11-15).
- **Daten:** die erfundenen von dort, die neuesten 4.000 Artikel mit den Texten von je fünf anderen: im Mittel 3.465 Zeichen, die neuesten 500 3.888. Die Wörter dieser 500 zu bestimmen dauert hier 119 ms.
- **Nachher an den echten Daten:** dieselbe Testinstanz nach dem Update, mit 710.590 Artikeln, wieder während der Server Feeds abrief. `bench measure` zieht die Wörter bei jedem Lauf neu aus dem Bestand; verglichen wird also je Art von Suche, nicht dasselbe Wort.

| Suche, Median | Echt, vorher | Echt, nachher | p99 nachher | Erfunden, vorher | Erfunden, nachher | p99 nachher |
|---|---:|---:|---:|---:|---:|---:|
| Ein sehr häufiges Wort | 116 ms | 9 ms | 105 ms | 131 ms | 7 ms | 18 ms |
| Ein häufiges Wort | 234 ms | 13 ms | 70 ms | 132 ms | 7 ms | 27 ms |
| Ein selteneres Wort | 248 ms | 20 ms | 42 ms | 138 ms | 14 ms | 35 ms |
| Ein seltenes Wort | 246 ms | 18 ms | 61 ms | 140 ms | 15 ms | 32 ms |
| Zwei Wörter | 264 ms | 23 ms | 149 ms | 162 ms | 20 ms | 35 ms |
| Ein häufiges Wort, tief | 84 ms | 12 ms | 111 ms | 59 ms | 7 ms | 24 ms |
| Ein selteneres Wort, nur ungelesen | 248 ms | 20 ms | 43 ms | 153 ms | 21 ms | 37 ms |
| Ein häufiges Wort, Konto mit den wenigsten Artikeln | 30 ms | 11 ms | 44 ms | 82 ms | 15 ms | 25 ms |

- **Die langsamsten Suchen** brauchen an den echten Daten länger als an den erfundenen: Das 99. Perzentil liegt bei bis zu 149 ms (vorher bis zu 582 ms), die längste Suche brauchte 443 ms (vorher 664 ms). Ob das am Abruf liegt, der nebenher lief, ist nicht untersucht.
- **Platz:** An den echten Daten wächst `article_content` von 617 auf 992 MB, um etwa so viel, wie die Texte brauchen; ihre übrigen Indexe werden beim Umschreiben etwas kleiner. An den erfundenen wächst sie von 643 auf 1.200 MB, die ganze Datenbank von 1,7 auf 2,25 GB, weil deren Wörter anderthalbmal so groß sind wie die Texte, vermutlich weil die Texte kaum HTML haben. Der Index der Suche bleibt so groß wie vorher. Gespeichert werden die Wörter ohne Positionen, die die Suche nicht braucht; mit ihnen wären sie rund die Hälfte größer.
- **Migration:** 235 s für 710.000 erfundene Artikel, rund 200 s an den echten Daten. So lange startet der Server beim Update nicht. Die Datenbank wuchs an den echten Daten zwischendurch um 1,2 GB, weil PostgreSQL die Tabelle neu schreibt, bevor die alte Kopie wegfällt.
- **Listen, Artikel und Speichern** werden nicht langsamer. An den echten Daten brauchen 20 neue Artikel im Median 16,8 statt 18,0 ms; davon fällt mehr auf den Teil ohne den Index (6,9 statt 4,2 ms), weil die Wörter jetzt in der Tabelle bestimmt werden und nicht mehr im Index.

### Duplikate vom 2026-11-16

Paket 5.42: wie rueder einen Artikel erkennt, dem der Feed bei jedem Abruf eine neue Kennung gibt, und was das kostet. Ergebnis: Mit einem Index auf Feed und Datum kostet das Nachschlagen 0,3 ms je Abruf, auch mit kaltem Cache rund 1 ms; ohne Index las jeder Abruf rund 1.000 Seiten der Tabelle. Entschieden von Fiona am 2026-11-16.

- **Echte Daten:** ein Feed eines Themas von Google News in der Produktion, mit lesenden Abfragen von Fiona: 27.143 Artikel, 3.921 mit einem Titel, den es im Feed schon gab, 2.892 davon mit gleichem Datum. Der Feed wird alle 30 Minuten abgerufen. Seit November kamen drei Artikel je 18-mal, einer je Abruf, mit jedes Mal neuer GUID. Diese Kennungen enthalten seit 2024 nicht mehr die Adresse des Artikels. Die von 2021 und 2022 enthielten sie noch, und an ihnen ist zu sehen, dass derselbe Titel an verschiedenen Tagen verschiedene Artikel sind, etwa die Termine einer Veranstaltung. Kopien mit gleichem Titel und Datum lagen höchstens 448 Artikel auseinander, bis auf einen Ausreißer mit 15.093. Auch der alte rueder hatte solche Kopien, 100 bis 230 im Monat, obwohl er sich als Chrome ausgab.
- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher, virtuelle Platte, PostgreSQL 16.15 mit `shared_buffers = 2GB`, Release-Build.
- **Daten:** `rueder bench generate --articles 2000000`: 10 Konten, 300 Feeds, 2.000.000 Artikel, der größte Feed mit 318.563.
- **Messung:** `rueder bench measure`, je 500-mal ein Abruf mit 100 Einträgen unter unbekannten Kennungen, die Hälfte mit Titel und Datum eines der neuesten Artikel des Feeds, einmal für einen zufälligen Feed und einmal für den größten. Mit kaltem Cache `EXPLAIN (ANALYZE, BUFFERS)` derselben Abfrage für vier Feeds, nach einem Neustart von PostgreSQL mit geleertem Cache des Betriebssystems.

| Weg | Warm, ein Feed: Median, p99 | Warm, größter Feed | Kalt | Seiten von der Platte, kalt |
|---|---:|---:|---:|---:|
| Index auf Feed und Datum (gebaut) | 0,35 ms, 0,65 ms | 0,31 ms | rund 1 ms | 2 bis 3 |
| Ohne Index, die neuesten 1.000 über die Nummern | 1,7 ms, 4,0 ms | 0,9 ms | 25 ms bis 1 s | 470 bis 950 |
| Ohne Index, der ganze Feed | 4,9 ms, 440 ms | 414 ms | – | – |

- **Ohne Index liegt jeder Artikel auf einer eigenen Seite.** Die Artikel aller Feeds treffen gemischt ein. Von den 1.000 neuesten Artikeln eines kleinen Feeds liegt deshalb fast jeder auf einer eigenen Seite, zusammen rund 8 MB, beim größten Feed sind es rund 340 Seiten. Bei vielen Feeds passen diese Seiten nicht mehr in den Cache. Der Index fragt nur nach den Daten der Einträge, die neuesten liegen beieinander.
- **Schreiben.** 20 neue Artikel zu speichern brauchte mit dem Index 20,7 ms im Median und 84,7 ms im 95. Perzentil, ohne ihn 25,5 und 83,5 ms; der Unterschied geht in der Streuung unter, die der Index der Suche macht. Ohne den Index der Suche waren es 5,7 und 6,0 ms.
- **Platz und Migration.** Der Index braucht 61 MB für 2 Millionen Artikel, rund 30 Byte je Artikel, und entstand in 1,1 Sekunden.

### Verbindungen zur Datenbank vom 2026-11-19

Paket 5.33: ob mehr Verbindungen zur Datenbank mehr Lesen tragen, mit `load/reading.js` wie unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15). Ergebnis: Auf vier Kernen nicht. Mit 20 Verbindungen schafft der Server so viel wie mit 10, rund 2.300 Anfragen je Sekunde. Die Grenze sind die Kerne, die sich Server, PostgreSQL und Lastgenerator teilen, nicht der Pool.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,3 GHz), 15 GB Arbeitsspeicher, virtuelle Platte. PostgreSQL 16.15 mit `shared_buffers = 2GB`, Release-Build. Server, Datenbank und Lastgenerator (Node 22) auf derselben Maschine.
- **Daten:** `rueder bench generate --accounts 10 --feeds 20000 --articles 1000000`, dazu `rueder bench readers --count 100000`: 100.000 Leser mit 4.894.260 Abos, im Schnitt 48,9, höchstens 481, der beliebteste Feed mit 27.129 Lesern. Die Datenbank hat 4,6 GB. Ein Drittel der Artikel vom 2026-11-15, weil die Daten von damals für die Migrationen seither zu wenig Platz auf der Platte ließen.
- **Messung:** `DATABASE_CONNECTIONS=10`, dann 20, dann noch einmal 10, jeweils nach `VACUUM ANALYZE` und 90 Sekunden zum Aufwärmen, je 30 Sekunden mit 8, 32 und 64 Lesern zugleich; ein Besuch sind 9 Anfragen wie dort.

| Verbindungen | Zugleich | Anfragen je s | Besuche je s | Server | PostgreSQL | Baum p99 | Ungelesen p99 | Artikel p99 | Markieren p99 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 10 | 8 | 1.690 | 188 | 0,9 Kerne | 1,8 Kerne | 11 ms | 15 ms | 8 ms | 12 ms |
| 10 | 32 | 2.297 | 255 | 1,0 | 1,9 | 28 ms | 30 ms | 21 ms | 25 ms |
| 10 | 64 | 2.263 | 251 | 1,0 | 1,9 | 57 ms | 52 ms | 43 ms | 47 ms |
| 20 | 8 | 1.684 | 187 | 0,9 | 1,7 | 11 ms | 16 ms | 8 ms | 12 ms |
| 20 | 32 | 2.231 | 248 | 1,0 | 2,1 | 30 ms | 33 ms | 23 ms | 30 ms |
| 20 | 64 | 2.323 | 258 | 1,0 | 2,1 | 60 ms | 56 ms | 43 ms | 51 ms |
| 10, noch einmal | 8 | 1.870 | 208 | 0,9 | 1,7 | 10 ms | 14 ms | 7 ms | 11 ms |
| 10, noch einmal | 32 | 2.297 | 255 | 1,0 | 1,9 | 27 ms | 29 ms | 21 ms | 24 ms |
| 10, noch einmal | 64 | 1.858 | 206 | 1,0 | 1,9 | 71 ms | 63 ms | 53 ms | 56 ms |

- **Die Kerne sind die Grenze, nicht der Pool.** Ab 32 Lesern zugleich sind alle vier Kerne belegt: rund einer für den Server, zwei für PostgreSQL und einer für den Lastgenerator. Mehr Verbindungen lassen dann nur mehr Anfragen in PostgreSQL warten, und die Zeiten wachsen mit den Lesern wie mit 10. Die Läufe streuen um rund 10 %, wie die Wiederholung mit 10 Verbindungen zeigt.
- **Mehr als am 2026-11-15**, 2.300 statt 1.650 Anfragen je Sekunde: Mit einem Drittel der Artikel ist die erste Seite der ungelesenen Artikel billiger, und die Maschine hat 2,3 statt 2,1 GHz. Für den Vergleich der Verbindungen zählt nur, dass alle Läufe auf denselben Daten liefen.
- **Was daraus folgt.** Mehr Verbindungen lohnen erst mit mehr Kernen für PostgreSQL. Die Tabelle in [deploy/README.md](../deploy/README.md#a-large-instance) nennt 10 für vier Kerne und 20 für acht, das zweite nach der Faustregel aus dem Wiki von PostgreSQL (zwei Verbindungen je Kern, eine je Platte), nicht gemessen. Mit 250 Besuchen je Sekunde trägt diese Maschine die Spitze von 100.000 Konten (15 bis 35 Besuche je Sekunde, siehe dort) gut siebenfach.

### Grenzen im Speicher vom 2026-11-19

Paket 5.34: was eine Grenze im Speicher einen Aufruf kostet, mit `rueder bench limits` (Release-Build, ohne Datenbank). Gemessen sind je 5.000 Aufrufe der Grenze aller Anfragen (`rate`) und der falschen Passwörter (`login`), für einen Schlüssel, den die Tabelle kennt, und für einen neuen. Dazu kommt der Beginn eines Logins mit Passkey, den jeder ohne Anmeldung auslösen kann, jedes Mal von einem anderen Absender: mit 100 Zeremonien, von denen jede neue gleich wieder endet, und mit allen Plätzen belegt (`passkeys::MAX_CEREMONIES`), so dass jede neue die älteste verdrängt. Das geschah einmal mit 100 Schlüsseln und einmal mit einer vollen Tabelle, in der jeder Schlüssel eben gefragt hat, so dass keiner einfach gehen kann. „Vorher“ ist der Stand vor dem Paket, mit seinen 10.000 Schlüsseln und ein zweites Mal mit 100.000.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz). Aus drei Läufen ist der mittlere genommen; die Läufe streuen um bis zu einem Drittel.

| Stand | Grenze | Schlüssel | bekannt: Mittel / 99 % | neu: Mittel / 99 % |
|---|---|---:|---:|---:|
| vorher | Anfragen | 100 | 102 ns / 145 ns | 199 ns / 168 ns |
| vorher | Anfragen | 10.000 | 128 ns / 311 ns | 130 µs / 200 µs |
| vorher | Anfragen | 100.000 | 239 ns / 504 ns | 1,4 ms / 1,6 ms |
| vorher | Passwörter | 100 | 173 ns / 207 ns | 284 ns / 315 ns |
| vorher | Passwörter | 10.000 | 196 ns / 398 ns | 165 µs / 258 µs |
| vorher | Passwörter | 100.000 | 400 ns / 684 ns | 1,9 ms / 3,5 ms |
| nachher | Anfragen | 100 | 139 ns / 163 ns | 233 ns / 1,5 µs |
| nachher | Anfragen | 100.000 | 206 ns / 367 ns | 413 ns / 808 ns |
| nachher | Passwörter | 100 | 309 ns / 469 ns | 462 ns / 2,1 µs |
| nachher | Passwörter | 100.000 | 370 ns / 652 ns | 917 ns / 1,7 µs |
| vorher | Passkey-Logins | 100 | – | 1,5 µs / 2,3 µs |
| vorher | Passkey-Logins | 10.000 | – | 211 µs / 291 µs |
| nachher | Passkey-Logins | 100 | – | 938 ns / 1,4 µs |
| nachher | Passkey-Logins | 10.000 | – | 1,6 µs / 2,6 µs |

- **Ein neuer Schlüssel in einer vollen Tabelle sucht nicht mehr.** Bei 100.000 Schlüsseln kostet er 0,4 bis 0,9 µs statt 1,4 bis 1,9 ms, weniger als vorher schon bei 10.000. Vorher durchsuchte jeder neue die ganze Tabelle, unter der Sperre, die jede Anfrage nimmt.
- **Ein Login mit Passkey sieht die anderen nicht mehr an.** Vorher lief jeder neue durch alle offenen Zeremonien, um die abgelaufenen zu finden, die seines Absenders zu zählen und die älteste zu verdrängen. Bei 10.000 kostete das 0,2 ms unter der Sperre aller Zeremonien, ohne dass es eine Anmeldung brauchte. Jetzt sind es 1,6 µs.
- **Ganz so billig wie bei 100 ist er nicht.** Eine Tabelle mit 100.000 Schlüsseln passt nicht mehr in den Cache des Prozessors. Jeder Zugriff auf sie, auch auf einen bekannten Schlüssel, kostet deshalb einige Zehntel Mikrosekunden mehr, beim Platzmachen mehrmals. Mit der Zahl der Schlüssel wächst das nicht weiter. Dass das 99. Perzentil mit 100 Schlüsseln höher ist, liegt daran, dass die Tabelle dort noch wächst. Gegen die Millisekunden einer Anfrage (siehe [Verbindungen zur Datenbank](#verbindungen-zur-datenbank-vom-2026-11-19)) fällt beides nicht ins Gewicht.

### Platz, Arbeitsspeicher und Abos vom 2026-11-19

Paket 5.35. Ergebnis:

- Ein neu abgerufener Artikel braucht rund 7,5 KB, nach der Aufbewahrung aus E4 noch 1,2 KB.
- Den Platz bestimmen die Feeds, nicht die Konten. Ein Feed bringt im Median 57 Artikel im Jahr, im Mittel 4.530, die größten über 100.000. Ein Konto mit 50 Abos braucht rund 7 KB und wächst nur um seine Sterne, Labels und ausgeblendeten Artikel.
- In den Arbeitsspeicher gehören die Abos, die Zeilen der gelesenen Artikel und die Ausnahmen, von den Texten nur die geöffneten: 1,3 GB für 100.000 Leser mit je 49 Abos.
- Die Abos lassen ein Zehntel jeder Seite frei (Füllgrad 90, Migration 0030). Damit kommen alle Updates der Lesemarke ohne neuen Indexeintrag aus statt eines Drittels.

**Kennzahlen aus der Produktion.** Fionas Instanz am 2026-11-19, mit lesenden Abfragen, die nur Summen und Verteilungen ausgeben: 2 aktive Konten, 148 abonnierte Feeds, 715.805 Artikel, davon 707.040 aus dem alten rueder (angekommen vom 2021-05-30 bis zum 2022-10-17). Die Datenbank hat 2,2 GB, `shared_buffers = 512MB`, 99,3 % der Lesezugriffe kamen aus dem Speicher; der Cache der Bilder hat 59 MB.

Artikel je Feed und Jahr, 2022 im alten rueder: 119 Feeds in 290 Tagen, aufs Jahr hochgerechnet.

| 10. | 25. | 50. | 75. | 90. | 99. Perzentil | größter | Mittel |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 4 | 14 | 57 | 325 | 3.490 | 136.000 | 195.000 | 4.530 |

Die zwei größten Feeds brachten rund 60 % aller Artikel. Seit dem Umstieg kommen rund 900 Artikel am Tag aus 148 Feeds, ohne den ersten Abruf jedes Feeds, der auf einmal bringt, was der Feed gerade zeigt.

**Ein Artikel.** In der Produktion hat ein neu abgerufener Text im Mittel 2.760 Zeichen HTML (Median 865, 99. Perzentil 31.482). Er speichert nach Kompression 1,7 KB HTML, 1,4 KB Wörter der Suche und 0,3 KB Titel und Anriss und hat im Mittel 1,1 Versionen zu je 1,7 KB. Die Gegenprobe:

- **Maschine:** wie unter [Verbindungen zur Datenbank](#verbindungen-zur-datenbank-vom-2026-11-19), Release-Build.
- **Daten:** `rueder bench generate --accounts 10 --feeds 300 --articles 300000`, die Ankunft über die zehn Jahre verteilt, in denen die Artikel erschienen. Jeder Text besteht aus den Texten von vier Artikeln, im Mittel 2.693 Zeichen, die Versionen tragen den Text als Rohdaten, 1,06 je Artikel. Alles mit `VACUUM FULL` gepackt, die Indizes mitgezählt.

| | Zeile (`articles`) | Text, Wörter, Rest (`article_content`) | Versionen | zusammen |
|---|---:|---:|---:|---:|
| erfundene Daten | 0,5 KB | 6,0 KB | 3,4 KB | 9,9 KB |
| mit den Größen der Produktion | 0,6 KB | 4,5 KB | 2,4 KB | 7,5 KB |
| nach E4, ohne Text und Versionen | 0,5 KB | 0,7 KB | – | 1,2 KB |

Die erfundenen Texte haben mehr verschiedene Wörter als echte (2,5 statt 1,4 KB) und größere Rohdaten; die zweite Zeile setzt die Größen der Produktion in die gemessenen ein.

**Die Aufbewahrung.** `rueder retention run` mit `--articles 24 --remove text --versions 12 --orphans 90` brauchte für die 300.000 Artikel 57 s: 217.400 Texte gingen, dazu die Versionen von 108 Monaten.

- Die Versionen gehen als ganze Monate, ihr Platz ist sofort wieder auf der Platte: Die Datenbank hatte danach 2,2 statt 3,0 GB.
- Die Texte nicht. Weil jedes Update eine neue Fassung der Zeile schreibt, wuchs `article_content` erst von 1,8 auf 1,9 GB. `VACUUM` gibt den Platz frei, damit neue Zeilen ihn nutzen, macht die Datei aber nicht kleiner. Erst `VACUUM FULL` gibt ihn der Platte zurück: `article_content` danach 0,65 GB, die Datenbank 0,9 GB.
- Eine Instanz, die die Aufbewahrung einschaltet, wächst danach nicht mehr um die alten Texte, schrumpft ohne `VACUUM FULL` aber auch nicht.

**Je Feed und Jahr**, mit 7,5 KB je Artikel, 5,1 KB ohne Versionen und 1,2 KB ohne Text:

| Feed | Artikel je Jahr | ohne Aufbewahrung, je Jahr | mit E4, nach zwei Jahren | danach je Jahr dazu |
|---|---:|---:|---:|---:|
| Median | 57 | 0,4 MB | 0,7 MB | 0,1 MB |
| 90. Perzentil | 3.490 | 26 MB | 44 MB | 4 MB |
| Mittel | 4.530 | 34 MB | 57 MB | 5 MB |
| 99. Perzentil | 136.000 | 1,0 GB | 1,7 GB | 160 MB |

**Je Konto.** Ein Abo braucht gepackt 137 Byte samt Indizes (`subscriptions` mit 5 Millionen Abos 687 MB), ein Konto mit 50 Abos also rund 7 KB. Eine Ausnahme vom Lesestand, ein Stern oder ein Label kostet rund 110 Byte. Mit den Jahren wächst ein Konto um seine Sterne und Labels und um die Artikel, die seine Filterregeln ausblenden, eine Zeile je Artikel. In der Produktion waren es 389 ausgeblendete in wenigen Tagen.

**Arbeitsspeicher.** Die Daten von [Verbindungen zur Datenbank](#verbindungen-zur-datenbank-vom-2026-11-19): 100.000 Leser mit 5 Millionen Abos, 4,6 GB, `shared_buffers = 2GB`. PostgreSQL neu gestartet und den Cache des Betriebssystems geleert, dann 13 Minuten `load/reading.js` mit 32 Lesern zugleich. Danach lagen 1,3 GB im Cache von PostgreSQL:

| Tabelle | im Cache | ganz |
|---|---:|---:|
| `subscriptions` mit Primärschlüssel | 412 MB | 429 MB |
| `articles` | 436 MB | 465 MB |
| `article_states` mit Primärschlüssel | 184 MB | 258 MB |
| `article_content` mit TOAST | 167 MB | 1.545 MB |
| Konten, Sitzungen und Feeds | 37 MB | 37 MB |

- **Gebraucht werden die Abos und die gelesenen Artikel fast ganz,** von den Texten nur die geöffneten.
- **Der kalte Start war nach einer halben Minute vorbei:** Im ersten Fenster von 30 s kamen 1.631 Anfragen je Sekunde, im 99. Perzentil 68 ms für die ungelesenen Artikel, danach rund 1.800 und 37 ms wie warm.
- **Mit weniger als diesem Bestand** kam alles von der Platte, wie mit 128 MB unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15).

**Abos und Füllgrad.** Dieselben Daten, je 20 Minuten `load/reading.js` mit 32 Lesern, vorher `subscriptions` mit `VACUUM FULL` gepackt:

| Füllgrad | Updates der Lesemarke | ohne neuen Indexeintrag | tote Zeilen danach | Tabelle | Indizes |
|---|---:|---:|---:|---|---|
| 100, wie bisher | 31.368 | 35 % | 20.326 | 303 MB, 1,3 MB mehr | 351 MB, 0,2 MB mehr |
| 90 | 35.782 | 100 % | 48 | 337 MB, nicht mehr | 351 MB, nicht mehr |

- **Das Autovacuum lief für `subscriptions` in keinem Lauf.** Mit dem Standard von PostgreSQL wartet es auf 20 % tote Zeilen, bei 5 Millionen Abos auf eine Million; mit dem Füllgrad 90 entstehen kaum welche.
- **In der Produktion** waren 98 % der Updates ohne neuen Indexeintrag, weil 182 Abos auf wenigen halbleeren Seiten liegen.
- **`article_states`** wird fast nur eingefügt und gelöscht. Dort lief das Autovacuum zweimal in 20 Minuten; ein Füllgrad hilft nicht.
- **Am meisten geändert werden in der Produktion die Feeds**: 37.327 Updates, nur 5 % ohne neuen Indexeintrag, 431 Läufe des Autovacuum. Jeder Abruf schreibt die Zeit des nächsten, und die hat einen Index (`feeds_next_fetch`). Dagegen hilft kein Füllgrad; das Autovacuum kam hinterher.

**Nicht gemessen:** wie viele und wie schwere Feeds eine öffentliche Instanz hat. Die Raten stammen aus Fionas 148 Feeds, und schon dort tragen zwei Feeds den größten Teil. Ebenso wenig, wie viel ein Konto mit Filterregeln über ein Jahr ausblendet.

### Bild-Proxy unter Last vom 2026-11-19

Paket 5.36: ob rueder die Bilder für 100.000 Konten allein ausliefern kann oder einen geteilten Cache davor braucht. Ergebnis: Er kann es. Aus seinem Cache liefert er knapp 9.000 Bilder je Sekunde, gebraucht werden in der Spitze geschätzt 350 bis 2.100. Fiona hat daraufhin entschieden, dass Bilder nur für eingeloggte User bleiben und kein Cache vor rueder sie hält.

- **Maschine** wie unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15), Release-Build, Server, PostgreSQL und Lastgenerator auf denselben vier Kernen.
- **Daten:** `rueder bench readers --count 100000` in einer leeren Datenbank, 100.000 Leser mit je einer Sitzung, 45 MB. `load/images.js` mit 3.000 Bildern einer erfundenen Seite unter 50 Namen aus 127.0.0.0/8: je ein Vorschaubild, für jedes zweite ein Icon, für jedes vierte das Bild selbst, 5.250 Adressen. Die Quellen sind PNG, Fotos mit 1.600 × 1.000 Pixeln und rund 360 KB, Icons mit 64 × 64. Im Cache lagen danach 357 MB, im Mittel 69 KB, im Median 25 KB je Datei.

| Lauf | Bilder je Sekunde | p50 | p95 | Kerne Server | Kerne PostgreSQL |
|---|---:|---:|---:|---:|---:|
| erstes Mal, 32 zugleich | 110 | 298 ms | 337 ms | 2,0 | 0 |
| aus dem Cache, 8 zugleich | 7.761 | 0,9 ms | 1,8 ms | 1,5 | 0,9 |
| aus dem Cache, 32 zugleich | 8.928 | 3,2 ms | 6,5 ms | 1,6 | 0,9 |
| aus dem Cache, 64 zugleich | 8.763 | 6,7 ms | 12,6 ms | 1,7 | 0,9 |

- **Das erste Mal** holt, verkleinert und speichert der Proxy das Bild. Es hängt an den wenigen Bildern, die er zugleich dekodiert (`images::ImageProxy`), daher die zwei Kerne; die anderen warten, auch Icons, die nicht verkleinert werden.
- **Aus dem Cache** kostet ein Bild rund 0,2 ms im Server und 0,1 ms in PostgreSQL, das ist die Sitzung. Die Grenze sind die Kerne, die sich Server, PostgreSQL und Lastgenerator teilen. Der Server hielt 180 MB, es gab keinen Fehler.
- **Was 100.000 Konten brauchen**, geschätzt aus der Spitze von 15 bis 35 Besuchen je Sekunde unter [Viele Konten](#viele-konten-vom-2026-11-14-und-2026-11-15): In der Liste, wie die Seite beginnt, lädt ein Besuch die Icons seiner Feeds und die Bilder der geöffneten Artikel, rund 10; im Magazin bis zu 60 mit den Vorschaubildern. Jedes Bild lädt erst, wenn es zu sehen ist, und der Browser behält es ein Jahr. Das sind 350 bis 2.100 Bilder je Sekunde, ein Viertel bis ein Fünfundzwanzigstel dessen, was rueder hier schafft. Neue Bilder kommen mit neuen Artikeln: 20.000 Feeds mit dem Mittel aus der [Produktion](#platz-arbeitsspeicher-und-abos-vom-2026-11-19) bringen rund drei Artikel je Sekunde.
- **Spürbar wird eher die Leitung.** Zehn Bilder von 69 KB je Besuch sind bei 35 Besuchen je Sekunde rund 24 MB/s, also 200 Mbit/s. Dagegen hülfe nur ein CDN, kein Reverse-Proxy auf derselben Maschine, und das müsste die Bilder auch ohne Sitzung ausliefern.

**Nicht gemessen:** echte Fotos als JPEG und wie viele Bilder echte Artikel haben; die Schätzung nimmt drei geöffnete Artikel mit je drei Bildern an.

### Speicher beim Lesen eines Feeds vom 2026-11-20

Was `feeds::parse` höchstens an Speicher belegt, gemessen mit einem zählenden Allokator um den Aufruf (Release-Build, ohne Datenbank). Die Feeds sind ausgedacht: ein Kopf, immer dasselbe Stück und ein Schluss, 5 MiB und 20 MiB groß, so viel, wie ein Abruf annimmt (`http_client::Config::max_body`). „Vorher“ ist der Stand ohne den Vorfilter `feeds::limited`, gemessen mit 5 MiB, weil 20 MiB die Maschine an ihre Grenze gebracht hätten; der Verbrauch wächst dort mit der Größe.

- **Maschine:** 4 virtuelle Kerne (Intel Xeon, 2,1 GHz), 15 GB Arbeitsspeicher.

| Feed | vorher, 5 MiB | nachher, 5 MiB | nachher, 20 MiB |
|---|---:|---:|---:|
| RSS aus leeren `<item/>` | 736 MiB, 0,7 s | 0,4 MiB | 0,4 MiB, 0,3 s |
| Atom aus leeren `<entry/>` | 736 MiB | 0,4 MiB | 0,4 MiB |
| JSON Feed aus leeren Items `{}` | 2.128 MiB, 1,5 s | 0,5 MiB | 0,5 MiB |
| JSON Feed, ein Item mit lauter leeren Tags | 240 MiB | 0,2 MiB | 0,2 MiB |
| RSS, ein Item mit lauter `<enclosure url="a"/>` | 238 MiB | 0 MiB | 0 MiB |
| RSS, ein Item mit lauter `<media:content>` in `<media:group>` | 209 MiB | 0 MiB | 0 MiB |
| RSS, ein Item mit lauter verschiedenen Attributen | 84 MiB | 5,5 MiB | 23 MiB |
| RSS aus kurzen Items mit Titel, Text und ID | 101 MiB | 0,6 MiB | 0,6 MiB |
| JSON Feed aus kurzen Items mit Text und Tags | 163 MiB | 0,8 MiB | 0,8 MiB |

- **Der Verbrauch hängt nicht mehr an der Länge des Feeds.** Vorher hielt `feed-rs` jeden Eintrag, bevor `MAX_ARTICLES` griff, und jedes Element, aus dem es etwas macht: bis zu 425-mal die Größe des Feeds, 20 MiB leere Items in JSON über 8 GiB. Ein Konto konnte das mit einer Adresse auslösen, die es selbst bedient, beim Suchen, beim Abonnieren und bei jedem Abruf.
- **Das Teuerste, was bleibt,** ist ein Eintrag knapp unter `MAX_ELEMENTS` aus Enclosures oder `media:content`: rund 22 MiB. Was über die Grenze geht, fällt als ganzer Eintrag weg, deshalb 0 MiB in der Tabelle. Die Attribute eines Elements liest `quick-xml` auf einmal in seinen Puffer, das kostet einmal die Größe des Elements.
- **Gemessen wie in der CI:** `server/tests/feed_memory.rs` liest elf solcher Feeds zu 20 MiB mit einer Grenze von 64 MiB für den Allokator. Ohne den Vorfilter bricht der Test ab.
- **Nichts anderes ändert sich:** Alle 1.484 Eingaben des Fuzz-Korpus für Feeds ergeben vorher und nachher dieselbe Zahl von Artikeln und ausgelassenen Einträgen.

## Wissen über das Altsystem

Wo im alten Code steht, was der neue nachbauen oder übernehmen musste. Der alte Code ist nicht mehr im Repository; die Pfade gelten im Commit `0b36a96`, etwa mit `git show 0b36a96:backend/pkg/worker/feed.go`.
