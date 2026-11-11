# rueder: Umsetzung und Übergabe

Stand 2026-11-11. Dieses Dokument sagt, was fertig ist, wie gearbeitet wird und welche Arbeitspakete offen sind. Es ist so geschrieben, dass jemand Neues die Arbeit übernehmen kann. Den Plan mit Anforderungen und Entscheidungen führt [plan.md](plan.md), den Aufbau des neuen Systems [architecture.md](architecture.md).

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

1. [plan.md](plan.md) lesen: Anforderungen, Entscheidungen, die vier Schritte.
2. [architecture.md](architecture.md) lesen: Datenmodell, Lesestand, Security.
3. Den Entwurf der Oberfläche starten und durchklicken: `cd web && pnpm install && pnpm dev`. Er zeigt, was der Server am Ende können muss.
4. In der Tabelle [Stand](#stand) den laufenden Schritt suchen und unter [Arbeitspakete](#arbeitspakete) das nächste offene Paket nehmen. Jedes Paket nennt, woran man erkennt, dass es fertig ist, und wo im alten Code die Vorlage liegt.

Fachliche Entscheidungen trifft Fiona. Die [Detailfragen](plan.md#detailfragen) sind nach den Vorschlägen entschieden.

## Stand

| Schritt | Stand | Wo |
|---|---|---|
| Vorarbeit: klickbarer Entwurf der Oberfläche | Fertig und abgenommen am 2026-11-10 | `web/`, Pull Request #1 |
| 1 Fundament und Daten | Fertig am 2026-11-10: Schema, Artikelspeicher, Lesestand, Import und Messung | `server/` |
| 2 Backend | Begonnen am 2026-11-10: Pakete 2.1 (gehärteter HTTP-Client), 2.2 (Abruf mit Versionen), 2.3 (Bereinigung), 2.4 (Bild-Proxy), 2.5 (Konten und Sitzungen) und 2.10 (Betrieb) sind fertig | `server/src/http_client.rs`, `server/src/feeds.rs`, `server/src/fetch.rs`, `server/src/cleaning.rs`, `server/src/images.rs`, `server/src/accounts.rs`, `server/src/sessions.rs`, `server/src/login.rs`, `server/src/oidc.rs`, `Dockerfile`, `deploy/` |
| 3 Frontend | Entwurf steht, Anbindung an den Server offen | `web/` |
| 4 Umstieg | Offen | |

Was es heute gibt:

- `web/`: die neue Oberfläche als Entwurf mit erfundenen Daten, 154 Tests, CI-Job `web`.
- `server/`: das Rust-Programm mit dem neuen Schema, dem Artikelspeicher (einfügen, neue Version, Listen, öffnen, Versionen), dem Lesestand (lesen, wieder ungelesen, alle als gelesen, Stern, ausblenden, Ungelesen-Zahlen), dem Import der alten Datenbank, dem Mess-Werkzeug, dem gehärteten HTTP-Client, dem Abruf der Feeds, der mit `serve` läuft, der Bereinigung der Artikeltexte mit eigenen Regeln, dem Bild-Proxy unter `/img/`, Konten mit Passwort, Sitzungen und dem Login, auch über OpenID Connect. 134 Tests, die meisten gegen PostgreSQL, CI-Job `server`. Von der API gibt es `GET /api/health`, Login, Logout und `GET /api/me`; ausgeliefert wird das gebaute Frontend. `rueder health` fragt den laufenden Server, ob es ihm gut geht.
- `Dockerfile` und `deploy/`: das Container-Image mit Server und Frontend, die Compose-Datei mit PostgreSQL und die [Anleitung zur Installation](../deploy/README.md), CI-Job `image`.
- `docs/`: Plan, Architektur und dieses Dokument, mit den [Messwerten](#messung) aus Schritt 1.
- `CLAUDE.md`: die Regeln des Projekts in Kürze, für Sitzungen mit Claude.
- Das alte System in `backend/` und `frontend/`, unverändert und lauffähig.

Was es noch nicht gibt: die API für Feeds, Artikel und Lesestand. Das ist der Rest von Schritt 2, als Nächstes Paket 2.6. Abonnieren lässt sich ein Feed bis dahin nur direkt in der Datenbank.

## Repository

| Pfad | Was |
|---|---|
| `server/` | Neu. Rust: HTTP-Server mit `axum`, `sqlx` und PostgreSQL. Dasselbe Programm hat die Kommandozeile (Import, Messdaten, später Konten). Siehe [server/README.md](../server/README.md). |
| `server/src/` | `articles.rs` Artikelspeicher und Listen, `reading.rs` Lesestand, `http_client.rs` der gehärtete HTTP-Client, `feeds.rs` und `fetch.rs` Abruf, `cleaning.rs` Bereinigung, `reclean.rs` erneutes Bereinigen des Bestands, `images.rs` Bild-Proxy, `accounts.rs` Konten und Passwörter, `sessions.rs` Sitzungen und der Extraktor für eingeloggte Anfragen, `login.rs` Login, Logout und Rate-Limit, `oidc.rs` Login über OpenID Connect, `health.rs` der Unterbefehl `health` für den Healthcheck des Containers, `import.rs` Import der alten Datenbank, `bench.rs` Messdaten und Messung, `lib.rs` Router und Fehlertyp, `main.rs` Kommandozeile. |
| `server/rules/` | Die Listen der Bereinigung als Daten: gesperrte Adressen und Tracking-Parameter, eine Regel je Zeile. |
| `server/migrations/` | SQL-Migrationen, laufen beim Start von selbst. |
| `server/.sqlx/` | Cache der geprüften Abfragen, damit das Programm ohne Datenbank baut (`cargo sqlx prepare`). |
| `server/tests/` | Integrationstests gegen PostgreSQL. `tests/fixtures/legacy_schema.sql` ist das Schema des alten rueder, nachgebaut aus `backend/migrations/`. |
| `server/fuzz/` | Kommt mit Paket 2.11: Fuzz-Targets für Bereinigung, Feed-Verarbeitung und Bild-Proxy samt Korpus. |
| `web/` | Neu. SvelteKit, TypeScript und TailwindCSS als statische SPA. Siehe [web/README.md](../web/README.md). |
| `web/src/lib/` | Modelle (Lesestand, Regeln, Baum), Komponenten, Texte (`i18n/`), erfundene Daten (`demo/`). |
| `Dockerfile`, `.dockerignore` | Neu. Das Container-Image mit Server und gebautem Frontend. |
| `deploy/` | Neu. Betrieb: `compose.yaml` startet rueder und PostgreSQL, `.env.example` nennt die Einstellungen, `README.md` ist die Anleitung zur Installation, `smoke-test.sh` baut und startet alles zur Probe. |
| `docs/` | Diese Dokumente. `user_data.md` beschreibt die Daten des alten Frontends im Browser. |
| `.github/workflows/ci.yml` | CI für den neuen Code. |
| `backend/` | Alt. Fünf Go-Dienste (`cmd/api`, `authbackend`, `events`, `feedfinder`, `worker`), Migrationen in `backend/migrations/`. Bleibt bis zum Umstieg. |
| `frontend/` | Alt. Svelte-Frontend. Bleibt bis zum Umstieg. |
| `config/`, `docker-compose*.yaml` | Alt. Betrieb des alten Systems mit loginsrv, imgproxy und nginx. Die Compose-Datei des neuen liegt deshalb bis zum Abbau in `deploy/`. |

Der alte Code wird nicht mehr weiterentwickelt, nur noch gelesen: als Vorlage für den Import, die Bereinigung und das Verhalten des Abrufs.

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
- Tests: Integrationstests in `server/tests/` mit `#[sqlx::test]`, das je Test eine frische Datenbank anlegt. Property-Tests mit `proptest`, wo ein einfaches Modell als Vergleich taugt (Lesestand, Baum). Der Test des Imports braucht keine zweite Datenbank: Die nachgebaute alte Datenbank liegt in einem eigenen Schema (`legacy`) der Testdatenbank, und der Import liest sie über eine Verbindung mit diesem `search_path`.
- Checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, Tests mit Abdeckung (`cargo llvm-cov`).

### Frontend (web/)

- SvelteKit 3 als SPA (`adapter-static`), Svelte 5 mit Runes, TypeScript im strikten Modus, TailwindCSS 4, Vite. Node 22, pnpm in der Version aus `package.json`.
- Tests mit Vitest und Testing Library, sie liegen als `*.test.ts` neben dem Code.
- Texte in `web/src/lib/i18n/` auf Englisch und Deutsch, keine Texte in Komponenten.
- Farben über benannte Variablen (`bg-paper`, `text-soft`, `border-line`), damit Themen und Akzentfarben nur Variablen umfärben.
- Checks: `pnpm lint` (Prettier), `pnpm check` (svelte-check), `pnpm test:coverage`, `pnpm build`.

### Sprache

- Code, Kommentare im Code, READMEs und Commit-Nachrichten auf Englisch.
- Kommentare in SQL-Migrationen sowie Plan, Architektur und dieses Dokument auf Deutsch.
- Die Oberfläche auf Englisch und Deutsch.

## Einrichten und Befehle

Voraussetzungen: Rust (stable), Node 22, pnpm, PostgreSQL 16 (lokal oder per Docker).

Das Frontend, heute schon:

```sh
cd web
pnpm install
pnpm dev             # der Entwurf auf http://localhost:5173
pnpm lint            # prettier --check
pnpm check           # svelte-check und TypeScript
pnpm test            # Vitest
pnpm test:coverage
pnpm build           # die Anwendung für den Server, in build/
pnpm build:single    # die ganze Anwendung als eine build/index.html
```

`pnpm build:single` packt alles in eine HTML-Datei, die ohne Server läuft. Die CI hängt sie an jeden Lauf als Artefakt `rueder-ui-draft`; so lässt sich ein Stand der Oberfläche herumreichen.

Der Server:

```sh
cp server/.env.example server/.env   # DATABASE_URL, dazu Einstellungen für den Abruf
cd server
cargo run                            # wendet Migrationen an, lauscht auf http://127.0.0.1:3000
cargo test                           # legt je Test eine Datenbank auf dem Server aus DATABASE_URL an
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo sqlx prepare -- --all-targets  # nach jeder geänderten Abfrage: Cache in .sqlx/ erneuern

cargo run --release -- import-legacy --from postgres://user:passwort@host/rueder   # alte Datenbank übernehmen
cargo run -- account add <name>         # Konto anlegen, fragt nach dem Passwort; --admin für ein Admin-Konto
cargo run -- account password <name>    # neues Passwort setzen, beendet die Sitzungen des Kontos
cargo run -- account link <name> <subject>   # Konto mit einem Subject des Anbieters aus OIDC_ISSUER verknüpfen
cargo run -- account list               # Konten auflisten
cargo run --release -- reclean          # alle Artikel aus ihren Versionen neu bereinigen, mit --feed <id> nur einen Feed
cargo run --release -- bench generate   # 10 Konten, 300 Feeds, 10 Millionen Artikel in eine leere Datenbank
cargo run --release -- bench measure    # misst die Abfragen und gibt die Tabellen aus
```

Abfragen werden beim Kompilieren gegen die Datenbank aus `DATABASE_URL` geprüft; sie muss dafür das Schema haben (einmal `cargo run`), oder der Build nutzt mit `SQLX_OFFLINE=true` den Cache in `.sqlx/`, wie die CI. `cargo sqlx` kommt mit `cargo install sqlx-cli`. Das Konto der Datenbank muss Datenbanken anlegen dürfen, sonst laufen die Tests nicht.

Einstellungen des Logins: `CLIENT_IP_HEADER` nennt hinter einem Reverse-Proxy den Header mit der Adresse des Absenders, für das Rate-Limit. `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` und `OIDC_REDIRECT_URL` schalten den Login über OpenID Connect ein; beim Anbieter wird `<Adresse von rueder>/api/oidc/callback` als Redirect-Adresse eingetragen. Der Sitzungs-Cookie verlangt HTTPS, zum Entwickeln geht `http://localhost:3000`. Das Passwort liest `account add` ohne Terminal als eine Zeile von der Eingabe, für Skripte.

Einstellungen der Bereinigung: `CLEANING_RULES_DIR` nennt ein Verzeichnis mit eigenen Listen (`blocked-links.txt`, `tracking-parameters.txt`) an Stelle der eingebauten aus `server/rules/`. Nach einer Änderung der Listen oder der Regeln bringt `reclean` den Bestand auf den neuen Stand; ohne das holt ihn der nächste Abruf nur für die Artikel nach, die der Feed noch führt.

Einstellungen des Bild-Proxy: `IMAGE_CACHE_DIR` ist das Verzeichnis seines Cache (Standard `image-cache` im Arbeitsverzeichnis), `IMAGE_CACHE_MB` dessen Größe (Standard 1024), `IMAGE_PROXY_KEY` der Schlüssel für die Adressen. Ohne Schlüssel erzeugt der Server einen und legt ihn ins Cache-Verzeichnis.

Im Container, wie eine Installation läuft ([deploy/README.md](../deploy/README.md)):

```sh
docker build -t rueder .                                      # das Image, aus dem Wurzelverzeichnis
cd deploy && cp .env.example .env                             # POSTGRES_PASSWORD setzen
docker compose up --detach --build                            # rueder und PostgreSQL, http://127.0.0.1:3000
docker compose exec rueder rueder account add <name> --admin  # die Befehle von oben laufen im Container
deploy/smoke-test.sh                                          # baut, startet und prüft alles unter eigenem Namen und Port
```

Das alte System startet wie im [README](../README.md) beschrieben.

## CI

| Workflow | Was er prüft |
|---|---|
| `ci.yml`, Job `web` | Neues Frontend: Lint, svelte-check, Tests mit Abdeckung, Build, dazu die Ein-Datei-Fassung als Artefakt. |
| `ci.yml`, Job `server` | Neuer Server: PostgreSQL 16 als Dienst, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, Tests mit Abdeckung (`cargo llvm-cov`). Gebaut wird mit `SQLX_OFFLINE=true`. |
| `ci.yml`, Job `image` | Das Container-Image: `deploy/smoke-test.sh` baut es, startet es mit PostgreSQL nach `deploy/compose.yaml` und prüft Health, Frontend, Konto und Login, Neustart mit erhaltenen Daten sowie Betrieb ohne Root und mit schreibgeschütztem Dateisystem. Veröffentlicht wird nichts. |
| `backend-lint.yml`, `backend-test.yml`, `frontend-lint.yml`, `frontend-test.yml` | Das alte System. Bleiben bis zum Umstieg. |
| `gitleaks.yml`, `codeql-analysis.yml` | Geheimnisse im Repository und statische Analyse. Bleiben. |

Die Abdeckung geht an Codecov, getrennt nach den Flags `web` und `server` (`codecov.yml`). Der Upload braucht das Repository-Secret `CODECOV_TOKEN`. Die Status von Codecov sind informativ und blockieren keinen Merge.

In Schritt 2 kommen `cargo audit` und `cargo deny` dazu (Paket 2.9). Der Korpus der Fuzz-Targets läuft im Job `server` als gewöhnlicher Test mit; gefuzzt wird in der CI nicht.

## Arbeitsweise

- Jede Änderung kommt als Pull Request auf `main`. Die CI muss grün sein.
- Jedes Arbeitspaket und jeder Fix bekommt einen eigenen Branch von `origin/main`, benannt `feature/<name_in_snake_case>` oder `fix/<name_in_snake_case>`. Auf `main` wird nie direkt gepusht.
- Lieber ein Pull Request je Arbeitspaket als ein großer je Schritt.
- Vor dem Push laufen die Checks des betroffenen Teils lokal (siehe oben).
- Neuer Code kommt mit Tests. Ein Test wird nicht abgeschaltet, um die CI grün zu bekommen.
- Wer vom Plan oder von der Architektur abweicht, ändert das Dokument im selben Pull Request und nennt den Grund.
- Messwerte stehen mit Datum, Datenmenge und Maschine im Abschnitt [Messung](#messung).

## Arbeitspakete

Die Kürzel A1 bis A12 verweisen auf die [Anforderungen](plan.md#anforderungen).

### Schritt 1: Fundament und Daten

Ergebnis: ein Importprogramm, das an einer nachgebauten alten Datenbank geprüft ist, und Messwerte, die die Speicherstrategie bestätigen oder korrigieren. Abgeschlossen am 2026-11-10.

- [x] **1.1 Grundgerüst.** Crate `server/` mit `serve`, Konfiguration, Logs, Fehlertyp, `GET /api/health`, Auslieferung von `web/build` mit Rückfall auf `index.html`. Fertig, wenn `cargo run` den Entwurf der Oberfläche ausliefert und der CI-Job `server` grün ist.
- [x] **1.2 Schema.** Erste Migration mit den Tabellen aus [architecture.md](architecture.md#datenmodell): Konten, Sitzungen, Feeds, Ordner, Abos, die drei Artikeltabellen mit Monatspartitionen, Lesestand, Filterregeln, Labels. Fertig, wenn die Regeln des Modells als Constraints in der Datenbank stehen und getestet sind (eindeutige Nummer je Feed, höchstens fünf Ebenen, Position je Ebene).
- [x] **1.3 Abfragen (A1, A7).** Liste eines Feeds vor einer Nummer, gemischte Liste für Ordner und alles, Artikel öffnen, Ungelesen-Zahlen aller Feeds eines Kontos, lesen, wieder ungelesen, alle als gelesen, verdichten. Fertig, wenn die Tests dieselben Fälle prüfen wie `web/src/lib/unread.test.ts` und ein Property-Test den Lesestand gegen ein einfaches Modell vergleicht.
- [x] **1.4 Import.** Unterbefehl, der die alte Datenbank liest und die neue füllt, nach der Tabelle unter [Import der alten Daten](#import-der-alten-daten). Wiederholbar über `legacy_id`. Fertig, wenn ein Test eine nachgebaute alte Datenbank (Schema aus `backend/migrations/`) importiert und Feeds, Artikel, Nummern, Konten, Ordner und Abos prüft.
- [x] **1.5 Messung (A1, A2).** Generator für 10 User, 300 Feeds und 10 Millionen Artikel mit schiefer Verteilung der Feedgrößen, Messung der Abfragen aus 1.3 gegen das [Messziel](plan.md#messziel), Wiederholung mit zehnfacher Artikelzahl, soweit die Maschine es zulässt. Fertig, wenn die Werte unter [Messung](#messung) stehen und das Schema, falls nötig, danach korrigiert ist. Ergebnis: Ziel erreicht, nachdem die Listenabfrage umgebaut wurde; das Schema blieb. Die zehnfache Menge passte nicht auf die Platte, verglichen wurde stattdessen mit einem Zehntel.
- [x] **1.6 CI.** Job `server` in `ci.yml`, Flag `server` in `codecov.yml`, kurzes `server/README.md`.

### Schritt 2: Backend

Ergebnis: Der neue Server läuft gegen echte Feeds. Die API-Tests und die übernommenen Cleaner-Tests sind grün.

- [x] **2.1 Gehärteter HTTP-Client (A3).** Nur http und https, eigene DNS-Auflösung, Sperre für private, lokale und Link-Local-Adressen (IPv4 und IPv6), Verbindung genau zur geprüften IP, jede Weiterleitung neu geprüft, Grenzen für Größe, Zeit, Weiterleitungen und Dekompression. Fertig, wenn Tests jede Sperre einzeln belegen, auch die Weiterleitung auf eine interne Adresse, und ein Property-Test mit `proptest` zeigt, dass die Adressprüfung in keiner Schreibweise eine interne Adresse durchlässt (siehe [Fuzzing](architecture.md#fuzzing)). Dieses Paket zuerst, alle folgenden Abrufe hängen daran. Fertig am 2026-11-10, beschrieben unter [Gehärteter HTTP-Client](architecture.md#gehärteter-http-client). Dazugekommen ist eine Ausnahme, mit der ein Betreiber einzelne interne Netze freigibt. Nicht im Test: TLS gegen echte Server, das zeigt sich mit dem ersten echten Abruf in Paket 2.2.
- [x] **2.2 Abruf mit Versionen (A2).** Der HTTP-Client wird in `serve` einmal gebaut, mit den freigegebenen Netzen des Betreibers aus der Umgebung. Planer, bedingte Abrufe, `feed-rs`, Kennung und Prüfsumme je Artikel, neue Artikel und neue Versionen in einer Transaktion, höchstens 20 Versionen, Intervall nach Aktivität, Fehler am Feed, User-Agent je Feed. Vorlage für das Verhalten: `backend/pkg/worker/feed.go` und `backend/pkg/worker/scheduler/`. Fertig, wenn Tests mit aufgezeichneten Feeds neue Artikel, unveränderte Artikel, Scheinänderungen und echte Änderungen unterscheiden. Fertig am 2026-11-10, beschrieben unter [Abruf](architecture.md#abruf). Offen geblieben und dort vermerkt: Wechselnde Tracking-Parameter galten noch als Änderung (mit 2.3 erledigt), Filterregeln und Live-Meldung hängen an 2.6 und 2.7, und TLS gegen echte Server ist weiter ungetestet, weil die Entwicklungsumgebung keinen direkten Weg ins Internet hat.
- [x] **2.3 Bereinigung (A10).** `ammonia` mit Allowlist, die Regeln aus `frontend/src/rueder/helpers/ArticleCleaner.ts`, Listen als Datendateien, Befehl zum erneuten Bereinigen aus `article_versions`. Fertig, wenn die Fälle aus `ArticleCleaner.test.ts` übertragen und grün sind. Fertig am 2026-11-10, beschrieben unter [Bereinigung](architecture.md#bereinigung); die Fälle aus `ArticleCleaner.test.ts` und die für Tracking-Parameter aus `url.test.ts` stehen in `server/tests/cleaning.rs`. Dabei gefunden und behoben: Der HTML-Parser braucht für tief verschachtelte Texte quadratisch viel Zeit und kann aus wenig Text sehr viel Baum machen. Das betraf auch den Abruf aus Paket 2.2 und ist jetzt begrenzt. Die Prüfsumme für neue Versionen geht seither über den bereinigten Text, wechselnde Tracking-Parameter erzeugen also keine Version mehr. Geprüft an den echten Feeds von tagesschau.de, cre.fm und github.com, die über den Proxy der Entwicklungsumgebung geladen wurden. Offen geblieben: Bilder zeigen bis Paket 2.4 auf ihre Quelle. TLS des eigenen HTTP-Clients gegen echte Server bleibt ungetestet: Die Entwicklungsumgebung lässt nur ihren eigenen Proxy nach außen, der TLS selbst beendet, und der gehärtete Client verbindet sich mit Absicht direkt.
- [x] **2.4 Bild-Proxy (A11).** Signierte Adressen, Formatprüfung, feste Größen, Cache auf Platte, Grenzen. Vorlage für die heutigen Größen: `frontend/src/rueder/helpers/ImageProxy.ts`. Fertig, wenn Tests falsche Signaturen, Nicht-Bilder, zu große Bilder und interne Adressen ablehnen. Fertig am 2026-11-10, beschrieben unter [Bild-Proxy](architecture.md#bild-proxy). Wie entschieden, setzt der Server die Adresse des Proxy erst beim Ausliefern ein; weil es noch keinen Endpunkt gibt, der Artikel ausliefert, stehen dafür `Signer::in_html` und `Signer::address` bereit, und Paket 2.6 ruft sie auf. Geprüft auch an echten Bildern von cre.fm. Offen geblieben: Der Proxy verlangt noch keinen Login (kommt mit 2.5), `.ico`-Dateien werden abgelehnt, und wie beim Abruf ist der Weg zu echten Servern hier nicht testbar.
- [x] **2.5 Konten und Sitzungen (A3).** Argon2id, Sitzungs-Cookie, Login mit Rate-Limit, Befehle zum Anlegen von Konten und Setzen von Passwörtern, danach OpenID Connect. Fertig, wenn Tests Login, Ablauf, Abmelden und die Cookie-Attribute prüfen. Dazu gehört, dass der Bild-Proxy unter `/img/` nur noch mit Sitzung antwortet. Fertig am 2026-11-11, beschrieben unter [Login und Sitzungen](architecture.md#login-und-sitzungen). OpenID Connect ist gegen einen nachgebauten Anbieter getestet, nicht gegen einen echten. Abweichung: Die Signatur des ID-Tokens wird nicht eigens geprüft, weil das Token direkt vom Anbieter über TLS kommt; dafür muss der Anbieter https sein. Offen geblieben und dort vermerkt: eine Liste der eigenen Sitzungen, Konten anlegen durch einen Admin in der Oberfläche und der Einmal-Link für Konten ohne Passwort (zu 2.6), die Prüfung des `Origin`-Headers (2.9).
- [ ] **2.6 API (A4 bis A9, A12).** Baum, Listen, Artikel, Versionen, Lesestand, Regeln mit Vorschau, Labels, Einstellungen, Abonnieren mit Feed-Suche, OPML. Fertig, wenn es zu jedem Endpunkt einen Test gibt, der auch prüft, dass ein anderes Konto nichts davon sieht. Was die API ausliefert, zeigt bei Bildern immer auf den Bild-Proxy: der Text eines Artikels über `Signer::in_html`, Vorschaubild, Aufmacherbild und Icon über `Signer::address`. Ein Test prüft an jedem Endpunkt, der Bilder nennt, dass keine fremde Adresse hinausgeht.
- [ ] **2.7 Live-Updates (A6).** `GET /api/events` mit den Ereignissen aus [architecture.md](architecture.md#live-updates), Nachliefern nach `Last-Event-ID`. Fertig, wenn ein Test zwei Verbindungen desselben Kontos und eine fremde öffnet und nur die richtigen Ereignisse sieht.
- [ ] **2.8 API-Typen für das Frontend.** TypeScript-Typen aus den Rust-Typen erzeugen; die CI prüft, dass die eingecheckten Typen aktuell sind.
- [ ] **2.9 Härtung (A3).** Content-Security-Policy und weitere Header, Prüfung des `Origin`-Headers, Rate-Limits, Limits je Konto, `cargo audit` und `cargo deny` in der CI.
- [x] **2.10 Betrieb.** Container-Image mit Server und Frontend, Compose-Datei mit PostgreSQL, Anleitung zur Installation. Fertig am 2026-11-11, beschrieben unter [Betrieb](architecture.md#betrieb): `Dockerfile`, `deploy/compose.yaml`, [deploy/README.md](../deploy/README.md) und der CI-Job `image`, der das Image baut und mit `deploy/smoke-test.sh` startet. Dazugekommen ist der Unterbefehl `rueder health`, weil das Image keine Shell und kein curl für einen Healthcheck hat. Geprüft am gebauten Image (78 MB): Start mit PostgreSQL, Konto, Login, Neustart mit erhaltenen Daten, Betrieb ohne Root mit schreibgeschütztem Dateisystem. Offen geblieben: Das Image wird nicht veröffentlicht, eine Installation baut es selbst (rund drei Minuten auf vier Kernen); der Abruf echter Feeds aus dem Container heraus ließ sich in der Entwicklungsumgebung nicht prüfen, wie schon der Abruf selbst; das Backup ist beschrieben, eingerichtet wird es mit Paket 4.2.
- [ ] **2.11 Fuzzing (A3).** Targets in `server/fuzz/` (`cargo-fuzz`) für Bereinigung, Feed-Verarbeitung und Bild-Proxy mit den Eigenschaften aus [architecture.md](architecture.md#fuzzing), Startkorpus aus `backend/test/data/` und den Fällen der Cleaner-Tests, dazu ein Test, der den Korpus mit stabilem Rust abspielt. Starten mit `cargo +nightly fuzz run <target> -- -max_total_time=1800`. Fertig, wenn jedes Target eine halbe Stunde ohne Fund gelaufen ist, die Funde davor behoben sind und im Korpus liegen und der Korpus-Test in der CI läuft.
- [ ] **2.12 Security-Review** gegen die Tabelle in [architecture.md](architecture.md#security). Befunde werden behoben oder mit Begründung dort vermerkt.

### Schritt 3: Frontend

Ergebnis: rueder ist täglich benutzbar.

- [ ] **3.1 Anbindung.** Eine Fassung der Klasse `Reader`, die aus der API liest und schreibt, an Stelle von `web/src/lib/demo/reader.svelte.ts`. Änderungen am Lesestand wirken sofort und werden bei Ablehnung zurückgenommen. Dazu die Login-Seite. Die erfundenen Daten bleiben für Tests und die Ein-Datei-Fassung.
- [ ] **3.2 Live-Updates (A6).** SSE-Verbindung, Hinweis „N neue Artikel“, Lesestand von anderen Geräten, Wiederverbinden und Neuabgleich.
- [ ] **3.3 Einstellungen auf dem Server (A5)**, dazu die änderbare Tastaturbelegung.
- [ ] **3.4 Labels (A12).** Im Entwurf gibt es nur den Stern.
- [ ] **3.5 Abonnieren und OPML (A12).** Feed-Suche über den Server, Import und Export.
- [ ] **3.6 Tempo (A1, A4).** Virtualisierte Liste für sehr lange Feeds, Vorladen des nächsten Artikels.
- [ ] **3.7 Prüfung im Browser.** End-to-End-Tests der wichtigsten Wege gegen den echten Server mit Fixtures.

### Schritt 4: Umstieg

Ergebnis: Die Produktion läuft auf dem neuen rueder mit den alten Daten, der alte Code ist entfernt.

Es gibt keinen Parallelbetrieb: Die alte Produktion ist abgeschaltet, ihre Daten liegen in einem Backup. Der neue rueder geht ohne alte Daten in Betrieb, die alten Daten kommen später einmalig dazu (Entscheidung vom 2026-11-10, siehe [plan.md](plan.md#entscheidungen)).

- [ ] **4.1 Zweites Security-Review** vor der Inbetriebnahme. Davor laufen die Fuzz-Targets aus Paket 2.11 noch einmal, je eine halbe Stunde.
- [ ] **4.2 Inbetriebnahme.** Der neue rueder läuft in der Produktion, mit neu angelegten Konten und neu abonnierten Feeds. Backup der neuen Datenbank einrichten.
- [ ] **4.3 Import in den laufenden Bestand.** Den Import so erweitern, dass er alte Daten mit Konten, Feeds und Artikeln zusammenführt, die es schon gibt, wie unter [Einmaliger Import in den laufenden Bestand](#einmaliger-import-in-den-laufenden-bestand) beschrieben. Fertig, wenn ein Test eine nachgebaute alte Datenbank in eine Datenbank importiert, die schon Konten, Abos, Artikel und Lesestand hat, und Reihenfolge, Nummern, Lesestand, Ordner und Abos prüft, auch für ein Konto unter anderem Namen.
- [ ] **4.4 Probelauf.** Das Backup der alten Produktion in ein PostgreSQL einspielen und in eine Kopie der neuen Produktion importieren. Dieselben Messungen wie in Schritt 1, Dauer des Imports notieren.
- [ ] **4.5 Einmaliger Import in der Produktion.** Server anhalten, Backup, Import, Start. Weg zurück festhalten (das Backup von davor).
- [ ] **4.6 Lesestand und Labels aus dem Browser.** Das neue Frontend liest nach dem Import die Schlüssel `feedstate` und `labels` aus dem `localStorage` des alten Frontends ([user_data.md](user_data.md)) und schickt sie an einen einmaligen Endpunkt. Das geht nur, wenn das neue Frontend unter derselben Adresse läuft wie das alte und der Browser den Speicher noch hat. Weil der Import nach der Inbetriebnahme kommt, bietet das Frontend die Übernahme an, sobald das Konto importierte Daten hat, nicht beim ersten Login.
- [ ] **4.7 Abbau.** `backend/`, `frontend/`, die alten Workflows, Compose-Dateien und `config/` entfernen, README neu schreiben.

## Import der alten Daten

Die alte Datenbank ist PostgreSQL, das Schema steht in `backend/migrations/` (Fizz), die Modelle in `backend/pkg/repository/pop/models/`. Der Import (`rueder import-legacy --from <Adresse der alten Datenbank>`, Code in `server/src/import.rs`) liest sie über eine zweite Verbindung in einer einzigen Nur-Lese-Transaktion und ändert sie nicht.

| Alt | Neu | Hinweise |
|---|---|---|
| `feeds` (`id`, `feed_url`, `site_url`, `title`, `icon`, `fetched_at`, `fetch_delay_s`, `fetcher_state` als JSON mit `etag`, `last_modified`, `working`, `last_success`, `last_error`, `message`) | `feeds` | Die UUID wandert nach `legacy_id`. ETag und Abrufstand werden übernommen, damit der erste Abruf bedingt bleibt; er ist fällig, wenn der alte rueder wieder abgerufen hätte. Das Intervall wird auf 15 Minuten bis 12 Stunden begrenzt. Die Fehlermeldung kommt nur mit, wenn der Feed zuletzt nicht ging. `last_seq` ist die Zahl der Artikel des Feeds. Ein Feed ohne Adresse bleibt samt Artikeln weg und wird gemeldet. |
| `articles` (`id`, `seq` als globaler Zähler, `feed_id`, `site_guid`, `posted_at`, `link`, `thumbnail`, `image`, `image_title`, `title`, `teaser`, `content` als JSON mit `authors`, `tags`, `enclosures`, `text`) | `articles`, `article_content`, `article_versions` | Die neue Nummer im Feed ist `row_number() over (partition by feed_id order by seq)`, genau wie das alte System `feed_seq` berechnet. Das alte `seq` wird die neue ID, die Reihenfolge des Eintreffens bleibt also. Der heutige Text wird Version 1: im Original nach `article_versions` (Abrufzeit ist das alte `created_at`), als aktueller Text nach `article_content`. Die UUID steht in der Tabelle `legacy_articles`, nicht in einer Spalte, damit die heiße Tabelle schmal bleibt und die Zuordnung nach dem Umstieg wegkann. Der Text zum Lesen kommt aus der neuen [Bereinigung](architecture.md#bereinigung), wie bei einem Artikel, der heute abgerufen wird; auch Link, Vorschaubild und Bild verlieren ihre Tracking-Parameter. |
| `users` (`id`, `auth_origin`, `auth_subject`, `folders` als JSON) | `accounts`, `account_identities`, `folders`, `subscriptions` | `auth_subject` wird der Login-Name; ist er vergeben (Namen unterscheiden keine Groß- und Kleinschreibung) oder leer, bekommt das Konto einen anderen, und der Import meldet es. `auth_origin` und `auth_subject` stehen in `account_identities`. Die Ordner haben eine Ebene; ihre Reihenfolge und die der Feeds darin kommt aus der Reihenfolge im JSON. Hat ein User einem Feed einen eigenen Namen gegeben, wird er der Titel des Abos. |
| `user_feeds` (`user_id`, `feed_id`) | `subscriptions` | Ein Abo, das in keinem Ordner steht, landet auf oberster Ebene hinter den Ordnern. |
| `user_states` | nichts | Die Tabelle wird vom alten System nicht benutzt. Der Import meldet, wenn sie Zeilen enthält. |
| `labels` | nichts | Im alten Go-Code gibt es kein Modell dazu, die Labels liegen im Browser. Der Import meldet, wenn die Tabelle Zeilen enthält. |
| `localStorage` im Browser: `feedstate`, `labels` | Lesemarke, Ausnahmen, `labels`, `article_labels` | Nicht Teil des Imports, sondern von Paket 4.6. `readAllUntil` wird die Lesemarke, `readArticles` (Nummern im Feed) werden Ausnahmen, Artikel von Labels werden über ihre UUID (`legacy_id`) gefunden. |

Nach dem Import und vor der Übernahme aus dem Browser hat ein Konto keinen Lesestand. Gebaut ist der Vorschlag: Der Import setzt die Lesemarke jedes neuen Abos auf `last_seq` (nichts ist ungelesen), und die Übernahme aus dem Browser setzt den genauen Stand einmalig. Wer keinen alten Browser-Speicher mehr hat, beginnt dann mit null ungelesenen Artikeln statt mit zehntausenden.

### Wiederholung

So ist der Import heute gebaut. Gebraucht wird davon nur noch der Neustart nach einem Abbruch: Seit der Entscheidung vom 2026-11-10 gibt es keinen Parallelbetrieb und nur einen einmaligen Import (siehe [unten](#einmaliger-import-in-den-laufenden-bestand)).

Der Import lässt sich beliebig oft starten: einmal zum Ausprobieren und nach einem Abbruch einfach erneut.

- Ein Lauf fügt nur hinzu, was der neuen Datenbank fehlt, erkannt an den alten UUIDs: Feeds, Artikel, Konten und Abos. Er ändert und löscht nichts, was schon da ist. Umbenannte Feeds, verschobene Abos und der Lesestand bleiben.
- Neue Artikel eines bekannten Feeds werden hinten angehängt und sind für bestehende Abos ungelesen.
- Der Baum eines Kontos wird nur beim Anlegen des Kontos übernommen. Abonniert ein schon importierter User im alten rueder später einen weiteren Feed, kommt das Abo ans Ende der obersten Ebene, auch wenn es im alten rueder in einem Ordner liegt.
- Geschrieben wird in kleinen Transaktionen (2.000 Artikel). Fügt währenddessen jemand anderes Artikel in einen Feed ein, bricht der Import mit einem Hinweis ab; der neue Server sollte beim Import nicht abrufen.
- Hat der neue rueder zwischen zwei Läufen selbst abgerufen, erkennt der Import Artikel, die es schon gibt, an ihrer Kennung im Feed und legt sie nicht doppelt an. Später importierte Artikel stehen dann hinter den selbst abgerufenen, also in der falschen Reihenfolge, und die Nummern weichen von denen des alten rueder ab. Der Import nennt diese Feeds am Ende. Für den einmaligen Import in den laufenden Bestand reicht das nicht; Paket 4.3 ändert es.

Am Ende gibt der Import aus, was er hinzugefügt und was er ausgelassen hat: Feeds ohne Adresse, Abos auf Feeds, die es nicht gibt, Artikel und Ordner, deren JSON er nicht lesen konnte (sie kommen ohne Text beziehungsweise ohne Ordner), geänderte Login-Namen, und ob `user_states` oder `labels` Zeilen enthalten.

Gemessen am 2026-11-10 an einer nachgebauten alten Datenbank mit 1 Million Artikeln (1,4 GB): 3 Minuten 50 Sekunden, also rund 4.300 Artikel je Sekunde und hochgerechnet etwa 40 Minuten für 10 Millionen. Ein zweiter Lauf ohne Neues dauert 35 Sekunden. Das Ergebnis stimmte in Nummern, Reihenfolge, Titeln und Texten aller Artikel mit der Quelle überein (Prüfsumme über beide Datenbanken). Seit Paket 2.3 bereinigt der Import jeden Text; das ist in dieser Messung nicht enthalten und kostet nach den Zeiten der Bereinigung grob 0,2 Millisekunden je Artikel zusätzlich, bei 10 Millionen also etwa eine halbe Stunde. Die Dauer an echten Daten gehört zu Paket 4.4.

### Einmaliger Import in den laufenden Bestand

Geplant, noch nicht gebaut (Paket 4.3). Der neue rueder läuft schon, mit neuen Konten und Feeds, wenn die alten Daten aus dem Backup dazukommen. Der heutige Import erkennt Bekanntes nur an den alten UUIDs und hängt Artikel hinten an. Für diesen Fall gilt stattdessen:

- **Ablauf.** Das Backup der alten Produktion wird in ein PostgreSQL eingespielt, das der Import erreicht. Der neue Server steht während des Imports, davor gibt es ein Backup der neuen Datenbank. Ein Lauf mit `--dry-run` zeigt vorab, welche Konten und Feeds zusammengeführt und welche neu angelegt würden.
- **Feeds.** Ein alter Feed wird mit einem vorhandenen zusammengeführt, wenn die Adresse gleich ist. Der vorhandene behält Titel, Intervall und Abrufstand und bekommt die alte UUID. Alle anderen Feeds kommen neu dazu.
- **Artikel eines zusammengeführten Feeds.** Die alten Artikel sind älter als alles, was der neue rueder abgerufen hat, und müssen in den Listen davor stehen. Sie bekommen deshalb die Nummern 1 bis n in ihrer alten Reihenfolge und IDs unterhalb aller vorhandenen. Die vorhandenen Artikel des Feeds rücken um n nach hinten, zusammen mit Lesemarken, Ausnahmen und Labels ihrer Abonnenten. Das ist die einzige Stelle, an der vergebene Nummern sich ändern; sie ist vertretbar, weil der Server steht und es einmal geschieht. Ein Artikel, den beide Seiten haben (gleiche Kennung im Feed), bleibt, wie der neue rueder ihn hat, und wird nicht doppelt angelegt. Seine alte UUID zeigt danach auf ihn.
- **Konten.** Ein alter User wird mit dem Konto gleichen Namens zusammengeführt (Groß- und Kleinschreibung zählt nicht) oder mit dem Konto, das der Aufruf nennt: `--account <alter Name>=<neuer Name>`, mehrfach möglich. Alte User ohne Gegenstück bekommen ein neues Konto ohne Passwort.
- **Ordner und Abos beim Zusammenführen.** Was das Konto im neuen rueder eingerichtet hat, bleibt: Ein Abo, das es schon hat, behält Ordner, Titel und Platz. Die übrigen alten Abos kommen mit ihren alten Ordnern dazu; ein alter Ordner, den es auf oberster Ebene mit gleichem Titel schon gibt, geht in diesem auf.
- **Lesestand.** Alte Artikel gelten als gelesen, bis der Stand aus dem Browser kommt (Paket 4.6). Für ein vorhandenes Abo rückt der Lesestand nur mit den Nummern mit. Bei einem Abo, das der Import neu anlegt, steht die Lesemarke am Ende der alten Artikel: Was der neue rueder seither für andere abgerufen hat, ist ungelesen.
- **Zuordnung für den Browser.** Der alte Browser-Speicher nennt Artikel mit ihrer alten Nummer im Feed. Wo Artikel doppelt waren, stimmt sie nicht mehr mit der neuen überein. `legacy_articles` bekommt deshalb die alte Nummer als Spalte, in einer neuen Migration.

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

**Zehnfache Artikelzahl:** 100 Millionen Artikel brauchen rund 180 GB, die Messmaschine hat 30 GB Platte. Statt nach oben wurde deshalb nach unten verglichen: Dieselben Abfragen brauchen bei 1 Million und bei 10 Millionen Artikeln gleich lang (die beiden rechten Spalten gegen die linken). Die Abfragen lesen je Feed und je Zeile der Seite eine feste Zahl von Indexseiten, die Tiefe des Index wächst mit dem Zehnfachen um höchstens eine Ebene. Die Wiederholung mit mehr Daten gehört zum Probelauf auf einer Kopie der Produktion (Paket 4.4).

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
- **Nicht gemessen:** der Index für eine Volltextsuche (Detailfrage in [plan.md](plan.md#detailfragen)). Das gehört zu Paket 2.6, bevor die Suche gebaut wird. Ebenfalls offen ist die Messung an echten Daten (Paket 4.4).
- **Reserve, falls es später eng wird:** Die Übersetzung der Lesemarke und die wieder ungelesenen Artikel unter der Marke kosten je Feed einen Zugriff auf die Tabelle. Ein Index auf `(feed_id, seq)`, der die ID mitführt, würde ihn sparen. Bei den gemessenen Zeiten lohnt das den größeren Index nicht.

## Wissen über das Altsystem

Wo im alten Code steht, was der neue nachbauen oder übernehmen muss:

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
| Betrieb heute | `docker-compose.yaml`, `config/` |
