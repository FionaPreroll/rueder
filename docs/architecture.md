# rueder: Architektur des neuen Systems

Stand 2026-11-12. Dieses Dokument beschreibt, wie der neue rueder aufgebaut ist und warum. Den Plan mit Anforderungen und Entscheidungen führt [plan.md](plan.md), die Umsetzung mit Stand und Arbeitspaketen [implementation.md](implementation.md).

Gebaut sind die Schritte 1 bis 3; das erste Security-Review (Paket 2.12) ist am 2026-11-12 gelaufen, sein Ergebnis steht unter [Security-Review vom 2026-11-12](#security-review-vom-2026-11-12). Jeder Abschnitt nennt das Paket, mit dem sein Teil kam. Was noch Entwurf ist, sagt der Abschnitt selbst, so der [einmalige Import in den laufenden Bestand](implementation.md#einmaliger-import-in-den-laufenden-bestand) aus Schritt 4. Wo ein Teil gebaut ist, gilt der Code: für das Schema die Migrationen in `server/migrations/`, für Artikelspeicher und Lesestand `server/src/articles.rs` und `server/src/reading.rs`, für den HTTP-Client `server/src/http_client.rs`, für die Bereinigung `server/src/cleaning.rs`, für den Bild-Proxy `server/src/images.rs`, für die Endpunkte die Module in `server/src/`, für die Oberfläche `web/src/lib/`. Wer von diesem Dokument abweicht, ändert es im selben Pull Request.

- [Überblick](#überblick)
- [Bausteine](#bausteine)
- [Datenmodell](#datenmodell)
- [Artikelspeicher und Versionen](#artikelspeicher-und-versionen)
- [Lesestand und Ungelesen-Zahlen](#lesestand-und-ungelesen-zahlen)
- [Ordner und Abos](#ordner-und-abos)
- [Filterregeln](#filterregeln)
- [Abruf](#abruf)
- [Bereinigung](#bereinigung)
- [Bild-Proxy](#bild-proxy)
- [Live-Updates](#live-updates)
- [Login und Sitzungen](#login-und-sitzungen)
- [Umzug eines Kontos](#umzug-eines-kontos)
- [Security](#security)
- [API](#api)
- [Frontend](#frontend)
- [Betrieb](#betrieb)
- [Status für Admins](#status-für-admins)
- [Android-App](#android-app)

## Überblick

Ein Rust-Programm übernimmt API, Abruf, Bereinigung, Live-Updates, Login und Bild-Proxy und liefert das gebaute Frontend aus. Daneben läuft nur noch PostgreSQL.

```mermaid
flowchart LR
    browser["Browser: Svelte-SPA"]
    subgraph rueder["rueder (ein Rust-Programm)"]
        api["API, Sitzungen, statische Dateien"]
        events["Live-Updates (SSE)"]
        fetcher["Abruf-Planer"]
        cleaner["Bereinigung, Filterregeln"]
        images["Bild-Proxy"]
        client["gehärteter HTTP-Client"]
    end
    db[("PostgreSQL")]
    net["Feeds, Webseiten, Bilder"]

    browser -->|"/api, /api/events, /img"| api
    api --> events
    api --> images
    fetcher --> cleaner
    fetcher --> client
    images --> client
    api --> db
    cleaner --> db
    client --> net
```

Zwei Regeln prägen den Aufbau:

- **Der Browser spricht ausschließlich mit rueder.** Kein Bild, kein Skript, kein Tracker wird direkt von fremden Seiten geladen.
- **Ins Internet geht nur der gehärtete HTTP-Client.** Feeds, Webseiten für die Feed-Suche und Bilder holt allein er. Deshalb gibt es genau eine Stelle für alle Schutzregeln gegen SSRF.

## Bausteine

| Baustein | Wahl | Grund |
|---|---|---|
| Server | Rust mit `axum` auf `tokio` | Ein Programm für alle Aufgaben, SSE ist eingebaut. |
| Datenbank | PostgreSQL 16 mit `sqlx` | Die Daten liegen schon dort. Partitionierung und Volltextsuche sind vorhanden, Abfragen werden beim Kompilieren gegen das Schema geprüft. |
| Feed-Abruf | `reqwest` mit `rustls`, Parser `feed-rs` | RSS, Atom und JSON Feed. Bedingte Abrufe mit ETag schonen die Quellen. |
| Bereinigung | `ammonia` plus eigene Regeln für Links und Bilder | Allowlist statt Sperrliste. |
| Live-Updates | SSE, intern ein Broadcast-Kanal | Ersetzt RabbitMQ. Bei mehreren Prozessen übernimmt `LISTEN/NOTIFY` von PostgreSQL. |
| Bilder | Eigener Proxy mit dem `image`-Crate | Ersetzt imgproxy, der Schlüssel für die Signatur bleibt auf dem Server. |
| Login | Lokale Konten mit Argon2id, OpenID Connect als Option | Ersetzt loginsrv und den Dienst `authbackend`. |
| Frontend | SvelteKit als SPA, Svelte 5, TypeScript, TailwindCSS 4 | Wird vom Server mit ausgeliefert. Die API-Typen werden aus dem Rust-Code erzeugt. |
| Betrieb | Ein Container plus PostgreSQL hinter einem Reverse-Proxy | Heute sind es acht Container plus imgproxy. |

Das Programm hat Unterbefehle: `rueder serve` (Standard) startet Server und Abruf, weitere Befehle erledigen einmalige Arbeiten (Import der alten Datenbank, Konten anlegen, Admins bestimmen, Messdaten erzeugen). Migrationen laufen beim Start von selbst.

## Datenmodell

Die Tabellen im Überblick. Die genauen Spalten stehen in den Migrationen, `server/migrations/0001_initial.sql` und den folgenden.

| Bereich | Tabellen | Idee |
|---|---|---|
| Konten | `accounts`, `account_identities`, `sessions`, `registration`, `invites` | Ein Konto hat einen Login-Namen, optional einen Argon2id-Hash und Einstellungen als JSON, dazu das Recht als Admin, die Zustimmung zu [Fehlerberichten](#fehlerberichte-aus-dem-browser) (`sends_reports`) und, woher es stammt (`origin`, `invite_id`, `invite_quota`, siehe [Registrierung und Einladungen](#registrierung-und-einladungen)). `account_identities` verknüpft Konten mit OpenID-Connect-Anbietern (Aussteller und Subject) und mit dem alten Login (`auth_origin`, `auth_subject`). Eine Sitzung ist der SHA-256 des Cookie-Tokens mit Ablaufzeit. |
| Feeds | `feeds` | Ein Feed gehört allen seinen Abonnenten gemeinsam. Er kennt seine höchste Artikelnummer (`last_seq`), den Abrufstand (nächster Abruf, Intervall, ETag, Last-Modified, letzter Erfolg, letzter Fehler) und den User-Agent, mit dem er abgerufen wird. |
| Baum | `folders`, `subscriptions` | Ordner mit Elternordner und Position. Ein Abo verbindet Konto und Feed, liegt in einem Ordner oder auf oberster Ebene, hat eine Position und trägt die Lesemarke. |
| Artikel | `articles`, `article_content`, `article_versions` | Heiß, warm und kalt getrennt, siehe [Artikelspeicher und Versionen](#artikelspeicher-und-versionen). Der Index der [Suche](#suche) liegt auf `article_content`. |
| Lesestand | `subscriptions.read_upto`, `article_states` | Lesemarke je Abo plus Ausnahmen je Artikel, siehe [Lesestand und Ungelesen-Zahlen](#lesestand-und-ungelesen-zahlen). Nach einem [Umzug](#umzug-eines-kontos) in einen Feed ohne ersten Abruf merkt sich `subscriptions.read_before` die Zeit, bis zu der gelesen ist. |
| Regeln | `filter_rules` | Filterregeln je Konto, siehe [Filterregeln](#filterregeln). |
| Labels | `labels`, `article_labels` | Labels mit Farbe je Konto und ihre Zuordnung zu Artikeln. Der Stern ist kein Label, sondern eine Ausnahme in `article_states`. |
| Fehlerberichte | `client_reports` | Fehler aus den Browsern der User, die zustimmen, je Konto und Fingerabdruck gezählt, siehe [Fehlerberichte aus dem Browser](#fehlerberichte-aus-dem-browser). |

Schlüssel sind fortlaufende Zahlen, keine UUIDs, `bigint` für Artikel und Fehlerberichte, `integer` für alles andere (Konten, Feeds, Ordner, Regeln, Labels, Einladungen): Sie halten Zeilen und Indizes schmal, und neue Artikel liegen im Index beieinander. Feeds und Konten behalten ihre alte UUID in einer Spalte `legacy_id`, Artikel in der Tabelle `legacy_articles`, damit der Import weiß, was er schon übernommen hat, und Lesestand und Labels aus dem Browser zugeordnet werden können. Die Tabelle statt einer Spalte hält die heiße Artikeltabelle schmal und kann nach dem Umstieg weg.

## Artikelspeicher und Versionen

Listen lesen nie Artikeltext, und alte Versionen berühren nie die Tabellen, aus denen neue Artikel gelesen werden. Dafür wird die eine Tabelle von heute in drei geteilt:

| Tabelle | Inhalt | Wird gelesen | Größe und Pflege |
|---|---|---|---|
| `articles` (heiß) | Eine schmale Zeile je Artikel: Feed, laufende Nummer im Feed, Titel, Anriss, Autor, Datum, Vorschaubild, Nummer der aktuellen Version, Zeitpunkt der letzten Änderung | Bei jeder Liste und jeder Zählung | Klein genug für den Arbeitsspeicher. Index auf Feed und Nummer. |
| `article_content` (warm) | Genau eine Zeile je Artikel: der bereinigte Text der aktuellen Version, dazu was nur die Artikelansicht braucht (Schlagwörter, Anhänge, Kennung des Feeds für den Artikel), und für die [Suche](#suche) eine Kopie von Titel, Autor und ID | Beim Öffnen eines Artikels, per Primärschlüssel, und bei der Suche | Wächst mit der Zahl der Artikel, nicht mit der Zahl der Versionen. |
| `article_versions` (kalt) | Jede Version im Original, mit zstd komprimiert, dazu Abrufzeit und Prüfsumme | Nur in der Versionsansicht und beim erneuten Bereinigen | Nach Monat partitioniert, es wird nur angehängt. Alte Partitionen können auf langsameren Speicher. |

So greifen die Teile ineinander:

1. **Neue Version nur bei echter Änderung.** Der Abruf vergleicht eine Prüfsumme über den normalisierten Inhalt. Tracking-Parameter und Leerraum zählen nicht mit, sonst erzeugen wechselnde Werbe-Token bei jedem Abruf eine Scheinversion.
2. **Die aktuelle Fassung bleibt an einem Ort.** Eine neue Version ersetzt die Zeile in `article_content` und hängt das Original an `article_versions` an. Wer die aktuelle Fassung liest, kommt mit der Historie nie in Berührung.
3. **Feste Nummer statt Fensterfunktion.** Jeder Artikel bekommt beim Einfügen seine laufende Nummer im Feed (`seq`). Blättern heißt dann „die 50 Artikel vor Nummer 4711“ und kostet immer gleich viel, egal wie groß der Feed ist.
4. **Keine Zählabfragen.** Der Feed kennt seine höchste Nummer. `COUNT(*)` entfällt.
5. **Alte Artikel kosten nur Plattenplatz.** Neue Zeilen liegen physisch beieinander, der Cache hält deshalb von selbst die neuen Seiten.
6. **Bereinigung ist wiederholbar.** Weil das Original gespeichert ist, lassen sich bessere Filterlisten oder ein Sanitizer-Fix nachträglich auf alte Artikel anwenden.

Regeln, auf die sich der Rest verlässt:

- **Nummern sind lückenlos und werden nie neu vergeben.** Ein Feed vergibt `last_seq + 1` in derselben Transaktion, in der der Artikel eingefügt wird; je Feed läuft immer nur ein Abruf. Die Ungelesen-Zahl rechnet mit `last_seq`, eine Lücke würde als ungelesener Artikel zählen.
- **Ein Artikel wird über seine Kennung im Feed wiedererkannt** (GUID, ersatzweise der Link), eindeutig je Feed.
- **Eine neue Version ändert den Lesestand nicht.** Der Artikel bleibt gelesen; die heiße Zeile bekommt die neue Versionsnummer und den Zeitpunkt, daraus macht die Oberfläche den Hinweis „aktualisiert“.
- **Höchstens 20 Versionen je Artikel.** Bei der 21. bleiben die Erstfassung und die 19 neuesten, die zweitälteste fällt weg.
- **Partitionen entstehen im Voraus.** Der Server legt beim Start und danach regelmäßig die Partitionen für den laufenden und den nächsten Monat an.

Gemischte Listen (ein Ordner, „Alle Artikel“) brauchen eine Reihenfolge über Feeds hinweg. Das ist die Reihenfolge des Eintreffens, also die fortlaufende ID des Artikels; innerhalb eines Feeds stimmt sie mit `seq` überein. Die Abfrage führt die Feeds zusammen wie sortierte Listen: Der Index `(feed_id, id)` liefert je Feed den neuesten passenden Artikel, der neueste davon kommt auf die Seite, und sein Feed liefert den nächsten. Das kostet einen Schritt in den Index je Feed und einen je Zeile der Seite, unabhängig von der Größe der Feeds. Bei „nur ungelesen“ ist die Lesemarke jedes Abos eine Untergrenze, unter der der Index nicht gelesen wird; wieder ungelesene Artikel unter der Marke kommen aus `article_states` dazu. Ein Feed, ein Ordner und alle Feeds sind dieselbe Abfrage. Bei 300 Feeds und 10 Millionen Artikeln bleibt sie mit 18 ms im 99. Perzentil unter dem [Messziel](plan.md#messziel), siehe [Messung](implementation.md#messung).

Beim Import wird der heutige Text jedes Artikels zu Version 1. Die laufenden Nummern werden so vergeben, wie das alte System sie heute bei jeder Abfrage berechnet (`row_number() over (partition by feed_id order by seq)`), damit der Lesestand aus dem Browser weiter passt.

## Lesestand und Ungelesen-Zahlen

Der Server speichert je Konto und Feed eine **Lesemarke** („alles bis Nummer N ist gelesen“, `subscriptions.read_upto`) und die **Ausnahmen** davon in `article_states`, eine Zeile je Konto, Feed und Nummer:

| Ausnahme | Bedeutung |
|---|---|
| gelesen | Der Artikel liegt über der Lesemarke und ist gelesen. |
| ungelesen gehalten | Der Artikel liegt an oder unter der Lesemarke und ist wieder ungelesen. |
| ausgeblendet | Eine Filterregel hat den Artikel ausgeblendet. Er zählt nie als ungelesen. |
| Stern | Unabhängig vom Lesestand. |

Die Ungelesen-Zahl eines Feeds wird immer aus diesen Daten berechnet:

```text
ungelesen = (last_seq − read_upto)
          − Artikel über der Marke, die gelesen oder ausgeblendet sind
          + Artikel an oder unter der Marke, die ungelesen gehalten und nicht ausgeblendet sind
```

Die Zahl eines Ordners ist die Summe seiner Feeds. Es gibt keinen gespeicherten Zähler, der auseinanderlaufen kann.

Die Operationen:

- **Lesen:** über der Marke entsteht die Ausnahme „gelesen“, unter der Marke verschwindet „ungelesen gehalten“.
- **Wieder ungelesen:** umgekehrt.
- **Alle als gelesen markieren (bis Nummer N):** Die Marke rückt auf N vor, alle Ausnahmen „gelesen“ und „ungelesen gehalten“ bis N entfallen. Die Marke bewegt sich nie zurück.
- **Verdichten:** Folgen auf die Marke lückenlos gelesene oder ausgeblendete Artikel, rückt sie darüber vor. So bleibt die Zahl der Ausnahmen klein, auch wenn jemand jahrelang Artikel einzeln liest.

Das maßgebliche Modell mit Tests ist `web/src/lib/unread.ts`. Der Server bildet genau dieses Verhalten nach und wird mit denselben Fällen geprüft, am besten mit Property-Tests gegen ein einfaches Modell („eine Menge gelesener Nummern“).

Der Lesestand hängt an (Konto, Feed, Nummer) und nicht an der ID des Artikels. Die Liste eines Feeds und seine Zählung kommen dadurch ohne Verbund mit der Artikeltabelle aus.

## Ordner und Abos

- Ordner lassen sich bis zu **fünf Ebenen** tief verschachteln. Ein Zug, der tiefer führen würde, wird abgelehnt, im Frontend und auf dem Server.
- Ordner und Abos einer Ebene teilen sich eine Reihenfolge. Jede Zeile trägt eine **Position** als Sortierschlüssel zwischen ihren Nachbarn. Ein Zug ändert genau eine Zeile („verschiebe X vor Y in Ordner Z“); gleichzeitige Änderungen von zwei Geräten überschreiben sich nicht mehr.
- Ein gelöschter Ordner gibt seinen Inhalt an die Ebene darüber ab.
- Grenzen je Konto für Ordner und Abos, wie heute 100 Ordner und 1.000 Feeds je Ordner, dazu 2.000 Feeds insgesamt.

Das Modell mit Tests ist `web/src/lib/tree.ts` (`canMove`, `moveNode`, `nudge` für die Tastatur).

Gebaut in Paket 2.6 (`server/src/tree.rs`, `server/src/subscribe.rs`):

- **Züge.** `POST /api/tree/move` nimmt Knoten, Ziel und `before`, `after` oder `inside`, wie das Modell im Frontend. Ein Test spielt zufällige Züge gegen ein Modell, das `tree.ts` nachgebaut ist. Jede Änderung am Baum sperrt die Zeile des Kontos, es läuft also je Konto nur eine.
- **Grenzen.** 100 Ordner und 2.000 Feeds je Konto, 1.000 Feeds je Ordner und ebenso auf der obersten Ebene, Namen bis 100 Zeichen (siehe [Limits je Konto](#header-herkunft-und-limits)).
- **Feed-Suche.** `POST /api/feeds/find` sieht sich eine Adresse an. Ist sie ein Feed, ist das der eine Treffer. Ist sie eine Webseite, nennt die Antwort die Feeds, auf die die Seite mit `<link rel="alternate">` zeigt, höchstens 20. Gelesen werden die ersten 512 kB der Seite, mit dem Parser der [Bereinigung](#bereinigung) und in dessen Grenzen. Die Suche legt nichts an. **YouTube:** Die Seite eines Kanals nennt seinen Feed zuerst, `https://www.youtube.com/feeds/videos.xml?channel_id=UC…`, mit dem Namen des Kanals. Bei Adressen wie `youtube.com/@name` steht die Kennung des Kanals im `<meta property="og:url">` der Seite (`…/channel/UC…`); eine Adresse `youtube.com/channel/UC…` führt ohne Umweg zum Feed, die Seite wird gar nicht erst geholt. Gilt nur für `youtube.com`, `www.youtube.com` und `m.youtube.com` und nur für eine Kennung aus „UC“ und 22 Buchstaben, Ziffern, `-` oder `_`. Schickt YouTube statt der Seite seine Frage nach Cookies (`consent.youtube.com`, wie es in der EU vorkommt), findet die Suche bei `@name` nichts.
- **Abonnieren.** `POST /api/feeds` abonniert die Adresse eines Feeds. Einen Feed, den noch niemand liest, ruft der Server dabei ab und behält ihn nur, wenn das gelingt: Aus einer beliebigen Adresse wird so kein Feed. Eine Adresse mit Name und Passwort wird abgelehnt, denn ein Feed gehört allen, die ihn lesen. Ohne Schema gilt eine Adresse als https.
- **Lesemarke eines neuen Abos.** Wer einen Feed abonniert, den andere schon lesen, findet die 10 neuesten Artikel ungelesen und alles davor gelesen, wie in den [Detailfragen](plan.md#detailfragen) entschieden (`subscribe::NEWEST_UNREAD`). Ein neuer Feed bringt mit, was sein erster Abruf liefert. Die [Filterregeln](#filterregeln) des Kontos sehen sich den Feed sofort an.
- **OPML.** `GET /api/opml` gibt den Baum aus, Ordner als Gliederung um ihre Feeds. `POST /api/opml` nimmt eine Datei bis 1 MB mit höchstens 2.000 Feeds und einer Gliederung bis 32 Ebenen. Ordner werden am Namen an ihrer Stelle wiedererkannt oder angelegt; was tiefer als fünf Ebenen liegt, landet im tiefsten Ordner, der passt. Die Reihenfolge der Datei bleibt, und Feeds, die das Konto schon liest, bleiben, wo sie sind. Der Import ruft nichts ab: Neue Feeds sind sofort fällig, die nächste Runde des [Abrufs](#abruf) holt sie, und ein Feed, der keiner ist, zeigt seinen Fehler im Baum. Der Leser kennt nur die fünf Entitäten von XML. Die Datei muss als `text/x-opml` oder XML gesendet werden, was ein Formular einer fremden Seite nicht kann.
- **User-Agent.** `PUT /api/feeds/{id}/agent` stellt den Abruf eines Feeds auf den Browser-User-Agent um oder zurück. Der Feed gehört allen Abonnenten gemeinsam, die Einstellung gilt deshalb für alle, und jeder von ihnen kann sie setzen.

Abweichungen: Die Grenze von 1.000 Feeds gilt auch für die oberste Ebene. Wer ein Abo kündigt, verliert seinen Lesestand und seine Labels in diesem Feed; der Feed und seine Artikel bleiben, auch wenn ihn niemand mehr liest, und werden dann nicht mehr abgerufen.

## Filterregeln

Eine Regel hat:

| Teil | Werte |
|---|---|
| Bereich | alle Feeds, ein Ordner (mit allem darunter), ein Feed |
| Feld | überall, Titel, Text, Autor, Schlagwort |
| Vergleich | enthält, ganzes Wort, regulärer Ausdruck |
| Aktion | ausblenden, als gelesen markieren |

- Regeln greifen **beim Eintreffen** eines Artikels, für jeden Abonnenten des Feeds. Treffer zählen nicht als ungelesen.
- Wird eine Regel angelegt, geändert oder gelöscht, bewertet der Server den Bestand des Kontos neu: die jüngsten 200 Artikel jedes Feeds und alles, was ausgeblendet ist. Eine **Vorschau** zeigt vor dem Speichern, wie viele Artikel betroffen wären.
- Ausgeblendetes bleibt unter „Ausgeblendet“ einsehbar.
- Der Vergleich arbeitet auf Text ohne HTML-Tags, ohne Rücksicht auf Groß- und Kleinschreibung; „ganzes Wort“ behandelt Umlaute als Buchstaben.
- Reguläre Ausdrücke laufen mit dem `regex`-Crate in garantiert linearer Zeit. Muster sind höchstens 200 Zeichen lang, die Zahl der Regeln je Konto ist begrenzt.

Das Modell mit Tests ist `web/src/lib/filters.ts`. Der Server muss dieselben Fälle gleich entscheiden; die Syntax für reguläre Ausdrücke ist die des `regex`-Crates, das Frontend prüft Muster deshalb über den Server und nicht mit `RegExp` des Browsers.

Gebaut in Paket 2.6 (`server/src/filters.rs`), mit den Fällen aus `filters.test.ts` als Tests:

- **Ausgeblendet ist, was eine Regel trifft.** Ein Artikel ist genau dann ausgeblendet, wenn eine eingeschaltete Regel ihn trifft, in deren Bereich sein Feed liegt. Geht die Regel, wird sie geändert oder ausgeschaltet, kommt der Artikel zurück, auch wenn er längst nicht mehr zu den jüngsten gehört. Von Hand blendet niemand aus. „Als gelesen markieren“ wird dagegen nie zurückgenommen.
- **Ausblenden geht vor.** Trifft eine Regel zum Ausblenden und eine zum Markieren denselben Artikel, wird er ausgeblendet und nicht hinter dem Rücken des Lesers als gelesen markiert.
- **Beim Eintreffen** wendet der Abruf die Regeln aller Abonnenten in der Transaktion an, die die Artikel speichert. Niemand sieht einen Artikel ungefiltert.
- **Der Baum zählt mit.** Ein Zug in einen Ordner oder aus ihm heraus und das Löschen eines Ordners bewerten neu, wenn eine Regel an einem Ordner hängt oder etwas ausgeblendet ist. Eine Regel verschwindet mit ihrem Ordner und mit dem Abo ihres Feeds.
- **Ein Motor für alle Vergleiche.** „Enthält“ und „ganzes Wort“ werden in denselben regulären Ausdruck übersetzt, den auch ein eigenes Muster ergibt. Das übersetzte Programm darf höchstens 1 MB groß werden, sonst gilt das Muster als zu aufwendig. Rückbezüge und Vorausschau kennt das `regex`-Crate nicht; solche Muster lehnt der Server ab.
- **Abseits der Anfragen.** Das Vergleichen läuft auf eigenen Threads, damit eine Regel über viele Artikel den Server nicht aufhält. Den Text ohne Tags gewinnt ein einfacher Abschneider aus dem schon bereinigten Text, nicht der HTML-Parser.
- **Grenzen.** 200 Regeln je Konto, Muster bis 200 Zeichen.
- **Die Reihenfolge** der Regeln ist nur die der Liste; was sie entscheiden, hängt nicht von ihr ab.

Abweichungen vom Entwurf: Neu bewertet werden die jüngsten 200 Artikel je Feed und alles Ausgeblendete, nicht alle ungelesenen. Bei 300 Feeds sind das höchstens 60.000 Artikel statt möglicherweise Millionen, und was eine Regel einmal ausgeblendet hat, bleibt trotzdem nicht hängen. Neue Versionen eines Artikels bewertet der Abruf nicht neu; das holt die nächste Änderung an den Regeln nach.

## Suche

Gebaut in Paket 2.13 (`server/src/search.rs`, `server/migrations/0005_search.sql`). `GET /api/search` sucht in den Artikeln der eigenen Abos: in allen, in einem Feed (`feed`) oder in einem Ordner mit allem darunter (`folder`).

- **Was gefunden wird.** Ein Artikel, der alle Wörter der Suche hat, in Titel, Autor, Schlagwörtern oder Text seiner aktuellen Fassung. Ein Wort ab drei Buchstaben findet auch, was mit ihm beginnt („wahl“ findet „Wahlen“ und „Wahlkampf“), ein kürzeres nur sich selbst. Groß- und Kleinschreibung zählt nicht, Umlaute und ß bleiben, was sie sind. Wörter zerlegt der Parser von PostgreSQL; HTML-Tags und Entities lässt er aus, ein Wort mit Bindestrich findet er ganz und in seinen Teilen. Höchstens 200 Zeichen und 10 Wörter.
- **Was nicht.** Keine Stammformen: Die Feeds sind deutsch, englisch und gemischt, eine falsche Sprache wäre schlimmer als keine, und die Suche nach Wortanfängen gleicht einen Teil davon aus. Keine Wortfolgen in Anführungszeichen und keine Rangfolge: Die Treffer kommen wie die Listen, der neueste zuerst. Ausgeblendetes fehlt, mit `unread` auch Gelesenes. Vom Text zählen die ersten 100.000 Zeichen, weil die Wörter eines Artikels in PostgreSQL höchstens 1 MB groß sein dürfen und ein Artikel, an dem das scheitert, den Abruf seines Feeds scheitern ließe.
- **Der Index** ist ein GIN-Index über einen Ausdruck auf `article_content`: die Wörter (`to_tsvector('simple', …)`) aus Titel, Autor, Schlagwörtern und Text. Die Wörter selbst werden nicht gespeichert. In einer eigenen Spalte wären sie größer als der Text: In einer Probe mit 200.000 erfundenen Artikeln brauchten sie 830 Byte je Artikel, der Text 560. Damit der Ausdruck alles in einer Zeile findet, trägt `article_content` eine Kopie von Titel, Autor und ID aus `articles`. Trigger halten sie gleich, wer auch immer schreibt (Abruf, Import, erneutes Bereinigen); nur wer beide Tabellen in einer Anweisung füllt, wie die Messdaten, setzt sie selbst. Die Migration baut den Index für alles, was schon gespeichert ist. Er hat keine Warteliste (`fastupdate = off`): Mit ihr sammelt PostgreSQL neue Wörter und arbeitet sie in einem Rutsch ein, was in der Messung einzelne Abrufe Sekunden warten ließ; ohne sie kostet jeder Artikel beim Schreiben gleichmäßig rund 1,5 ms mehr.
- **Zwei Wege zu den neuesten Treffern.** Der Index weiß, welche Artikel ein Wort haben, nicht, in welcher Reihenfolge sie kamen, und was er zu einem Wort hält, zu lesen kostet so viel, wie es Artikel damit gibt, in allen Feeds und Jahren: für ein seltenes Wort fast nichts, für eines in jedem zehnten Artikel bei 10 Millionen Artikeln eine halbe Sekunde. Der andere Weg geht vom neuesten Artikel der Suche rückwärts und bestimmt die Wörter jedes Artikels neu. Das kostet je Artikel gleich viel (in der Messung rund 60 µs), ist also schnell, wenn jeder zehnte ein Treffer ist, und langsam für ein seltenes Wort. Zwischen beiden wählt die Suche selbst, nicht der Planer von PostgreSQL: Der schätzt grob, wie viele Wörter mit einem Anfang beginnen, und nahm in der Messung für häufige Wörter in der Hälfte der Fälle den Index (0,5 s statt 30 ms) und für seltenere manchmal den Weg rückwärts durch Millionen Artikel, bis die Zeit um war (siehe [Messung](implementation.md#suche-vom-2026-11-11)).
- **Wie die Suche wählt.** Sie sieht sich zuerst die neuesten 500 Artikel der Suche an. Das zeigt, wie häufig die Treffer und wie häufig die einzelnen Wörter darin sind, und was ein Artikel anzusehen kostet; das misst sie selbst, weil es an der Länge der Texte hängt und daran, wie weit die Artikel der Suche auseinanderliegen. Daraus schätzt sie, was der Rest der Seite auf beiden Wegen kostet. Für den Index rechnet sie 1,3 µs je Artikel mit dem seltensten Wort, gemessen bei 10 Millionen Artikeln; ist der Weg über den Index billiger und bleibt er unter einer Sekunde, fragt sie ihn, sonst sieht sie weiter Artikel an, in Schritten von höchstens 4.000. Den Index fragt sie nur nach dem seltensten Wort, die anderen sucht sie in dem, was er findet: Nach allen gefragt, läse er jeden Artikel mit irgendeinem davon, auch mit dem häufigsten. Und sie fragt ihn in Zeitfenstern der Reihenfolge des Eintreffens, zuerst 20.000 Artikel, dann doppelt so viele davor und so weiter, damit sie nur die Treffer in einem Fenster liest; dafür nimmt PostgreSQL den Index der ID dazu. Was das Neubestimmen der Wörter kostet, sagt ihm die Funktion des Ausdrucks mit `COST 2000`. Die Statistik über den Index kennt 10.000 Wörter statt 1.000, damit auch die mittlerer Häufigkeit richtig geschätzt werden. Die ganze Tabelle zu lesen ist für die Suche ausgeschaltet.
- **Blättern mit `next`.** Eine Anfrage sucht höchstens eine halbe Sekunde, dann hört sie am Ende eines Schritts auf und sagt mit `next`, wo es weitergeht. Eine Seite kann deshalb weniger Treffer haben als verlangt, auch keinen, und trotzdem weitergehen; `next` ist `null`, wenn nichts Älteres mehr kommt. Die nächste Seite fragt mit `before` gleich `next`. Ein einzelner Schritt, der länger als drei Sekunden braucht, wird abgebrochen, die Antwort ist 503. So hält keine Suche eine Verbindung zur Datenbank lange fest.
- **Im Frontend.** Die Suchleiste fragt den Server, sobald das Tippen 0,3 Sekunden ruht, im gezeigten Feed, im gezeigten Ordner oder in allen Artikeln; von der Startseite aus wechselt die Seite dafür zu „Alle Artikel“. Die Treffer sind eine Liste wie die anderen, mit Lesestand, Stern, Tastatur und Nachladen. Kommt eine Seite wegen der Zeit zu kurz, fragt der Reader gleich weiter, höchstens zehnmal. In den Listen der markierten und der ausgeblendeten Artikel und in denen eines Labels sucht der Server nicht; dort durchsucht die Suchleiste weiter, was geladen ist. Der Reader mit erfundenen Daten durchsucht alle seine Artikel selbst, in Titel, Anriss und Autor.
- **Was es kostet,** steht unter [Messung](implementation.md#suche-vom-2026-11-11): Größe des Index, Zeit zum Bauen, Mehrkosten beim Schreiben und die Zeiten der Suche bei 10 Millionen Artikeln.

## Abruf

Ein Planer im Server ruft jeden Feed mit mindestens einem Abonnenten in seinem Intervall ab.

1. Fällige Feeds kommen aus der Datenbank (`next_fetch_at`), begrenzt viele laufen gleichzeitig, je Host nur einer.
2. Der gehärtete HTTP-Client holt den Feed, bedingt mit ETag und Last-Modified. „Nicht geändert“ beendet den Abruf.
3. `feed-rs` liest RSS, Atom oder JSON Feed, mit Grenzen für Größe und Zahl der Artikel. Atom 0.3, das manche Websites noch senden (die Tagesschau etwa), schreibt der Server vorher als Atom 1.0 um, weil `feed-rs` nur dessen Namensraum liest und die Elemente der älteren Fassung übergeht: Namensraum, die umbenannten Elemente (`issued`, `modified`, `tagline`, `copyright`), die Art der Texte (`mode="escaped"` wird `type="html"`) und `dc:date` als `updated`. Escapter HTML-Text, der zusätzlich in einer CDATA-Markierung als Text steckt, verliert diese Hülle. Davor bekommt `feed-rs` jeden Feed in XML als UTF-8 und ohne Deklarationen, die eine andere Kodierung nennen: Die Bibliothek wechselt sonst mitten im Lesen die Kodierung und bricht mit einer Panic ab, wenn die Deklaration nach den ersten 64 Bytes steht, bei einer langen Deklaration oder einer zweiten weiter unten (gefunden beim [Fuzzing](#fuzzing)). Bytes, die nicht zur Kodierung gehören, werden dabei Ersatzzeichen, denn die Bibliothek ließe sie weg und läse weiter, und aus den Buchstaben drumherum würde eine Deklaration (`enc\xC0oding`, gefunden beim Fuzzing zum [Security-Review](#security-review-vom-2026-11-12)). Die Kodierung nennt das Byte Order Mark, sonst die Deklaration. Adressen des Feeds (Seite, Icon, Links der Artikel, Vorschaubilder, Anhänge) bleiben nur, wenn sie ganze `http`- oder `https`-Adressen mit Host sind; die Kennung eines Artikels nimmt den Link weiter so, wie der Feed ihn nennt.
4. Je Artikel: Kennung bestimmen, Inhalt normalisieren, Prüfsumme bilden. Unbekannt heißt neuer Artikel, andere Prüfsumme heißt neue Version, sonst nichts.
5. Bereinigung des Inhalts (siehe unten), Anriss und Vorschaubild ableiten.
6. In einer Transaktion: Nummern vergeben, die drei Tabellen schreiben, Filterregeln der Abonnenten anwenden, `last_seq` und Abrufstand des Feeds setzen.
7. Meldung an die Live-Updates.

Das Intervall passt sich wie heute der Aktivität des Feeds an, zwischen 15 Minuten und 12 Stunden (heutige Logik: `backend/pkg/worker/feed.go`, `updateFetchDelay`). Fehler verlängern das Intervall schrittweise und stehen am Feed, die Oberfläche zeigt ein Warnzeichen. Der User-Agent ist ehrlich („rueder“ mit Version und Adresse des Projekts) und lässt sich je Feed auf einen Browser-User-Agent umstellen.

Die Feed-Suche auf Webseiten (heute der Dienst `feedfinder`) und der OPML-Import nutzen denselben Client.

Gebaut in Paket 2.2 (`server/src/feeds.rs` liest und speichert, `server/src/fetch.rs` ruft ab):

- **Runde.** `serve` sieht alle 30 Sekunden nach fälligen Feeds mit mindestens einem Abo, höchstens 100 je Runde. Ein genommener Feed ist für zehn Minuten vergeben, damit ihn keine zweite Runde und kein zweiter Server nimmt. Höchstens acht Abrufe laufen gleichzeitig, je Host einer.
- **Kennung.** Die ID, die der Feed dem Eintrag gibt, sonst sein Link, sonst eine Prüfsumme aus Titel und Text. Sie ist bei jedem Abruf dieselbe; ein Eintrag ohne alles wird ausgelassen.
- **Änderung.** Die Prüfsumme geht über Titel und bereinigten Text, Folgen von Leerraum zählen als ein Leerzeichen. Ein nur neu umbrochener Text ist also keine neue Version, und ebenso wenig einer, in dem sich nur ändert, was die [Bereinigung](#bereinigung) entfernt, etwa Tracking-Parameter.
- **Grenzen.** 500 Artikel je Abruf, gezählt vom Anfang des Feeds, 1 MB je Text; was darüber liegt, bleibt weg. Der Feed selbst ist durch den HTTP-Client auf 20 MB begrenzt.
- **Text zum Lesen.** Kommt aus der [Bereinigung](#bereinigung). Das Original bleibt als Version erhalten. Lesen und Bereinigen eines Feeds läuft auf einem eigenen Thread, damit ein großer Feed den Server nicht aufhält: Der Feed von CRE mit 226 Folgen und 2,4 MB Text braucht 0,4 Sekunden, der von tagesschau.de 7 Millisekunden.
- **Adressen.** Der Link des Artikels, Vorschaubild, Anhänge und die Adresse der Seite des Feeds verlieren ihre Tracking-Parameter. Die Kennung eines Artikels ohne eigene ID bleibt der Link, wie der Feed ihn sendet, denn so kennt ihn der alte rueder.
- **Fehler.** Der Grund steht am Feed (`error_message`), jeder Fehlschlag verdoppelt das Intervall bis zwölf Stunden, der erste Erfolg setzt es zurück.
- **Einstellungen.** `FETCH_ALLOW_NETS` gibt interne Netze frei (siehe [Gehärteter HTTP-Client](#gehärteter-http-client)), `FETCH=off` startet den Server ohne Abruf.

Abweichungen vom Entwurf oben: Die Filterregeln aus Schritt 6 kamen mit Paket 2.6, die Meldung aus Schritt 7 mit Paket 2.7 (`events::Events::publish_to_readers`). Anders als der alte rueder probiert der Abruf bei einer http-Adresse nicht zuerst https, und eine dauerhafte Weiterleitung ändert die gespeicherte Adresse des Feeds nicht.

## Bereinigung

Die Bereinigung läuft **einmal je Version auf dem Server** statt bei jedem Anzeigen im Browser. Gebaut in Paket 2.3 (`server/src/cleaning.rs`), in zwei Schritten und in dieser Reihenfolge:

1. **Eigene Regeln auf dem Dokumentbaum**, übernommen aus `frontend/src/rueder/helpers/ArticleCleaner.ts` samt Tests.
2. **`ammonia` mit fester Allowlist.** Es hat immer das letzte Wort: Was der erste Schritt auch tut oder übersieht, das Modul verlässt nur Auszeichnung von der Liste und keine Adresse, die nicht mit `http` oder `https` beginnt.

Die eigenen Regeln:

| Was | Regel |
|---|---|
| Links | Bekommen ihre vollständige Adresse, ohne Tracking-Parameter. Ein Link zu einer gesperrten Adresse (Teilen-Knöpfe) fällt samt Beschriftung weg. Ein Link, der nirgends ins Web führt (kein Ziel, `mailto:`, `javascript:`, relative Adresse ohne Bezug), lässt seine Worte stehen. `ammonia` setzt `rel="nofollow noopener noreferrer"` und `target="_blank"`. |
| Bilder | Bekommen ihre vollständige Adresse, ohne Tracking-Parameter, und `loading="lazy"`. Bilder gesperrter Adressen und Zählpixel (Breite und Höhe 0 oder 1) fallen weg. Fehlt der Titel, wird der Alternativtext auch Titel. `srcset` bleibt nicht, es würde der Seite die Bildschirmgröße verraten. |
| Audio und Video | Die Player fallen weg, ihre Dateien werden Anhänge des Artikels (`enclosures`, mit `"inline": true` markiert). So lädt nichts, bevor jemand es anklickt. |
| Relative Adressen | Werden zuerst von der Seite des Artikels aus aufgelöst, sonst von der Adresse des Feeds aus; `#anker` nur von der Seite des Artikels. Ohne Bezug fallen sie weg, denn im Browser würden sie in rueder selbst zeigen. |
| Leeres | Leere `p` und `span`, Kommentare und Folgen von Zeilenumbrüchen fallen weg oder zusammen, Folgen von Leerraum werden ein Leerzeichen (außer in `pre`). |
| Unbekannte Tags | Fallen weg, ihr Inhalt bleibt. `script` und `style` fallen mit Inhalt weg. |

Erlaubt sind die Tags für Text, Listen, Tabellen, Zitate, Code, Bilder und Links. An Attributen bleiben `href` an Links, `src`, `alt`, `width` und `height` an Bildern, `colspan` und `rowspan` in Tabellen, `start` an Listen, `datetime` an `time` sowie `title`, `lang` und `dir` überall. Es gibt kein `id`, `name`, `class` oder `style`: Nichts in einem Artikel kann sich als Teil von rueders eigener Seite ausgeben oder ihr Aussehen ändern.

Worauf sich der Rest verlässt:

- **Grenzen für den Parser.** Der HTML-Parser sucht bei jedem Tag durch alle offenen Tags, sein Aufwand wächst also mit dem Quadrat der Tiefe: Ein Megabyte aus nichts als öffnenden Tags beschäftigte ihn über eine Minute. Offen gelassene Formatierung wiederholt er in jedem folgenden Absatz, aus einem Megabyte Text kann so das Hundertfache an Baum werden. Deshalb zählt die Bereinigung beim Lesen mit: höchstens 256 Ebenen tief, höchstens 200.000 Knoten. Ein Text darüber wird nicht als HTML gelesen, sondern behält nur seine Worte. Das gilt für alles, was als HTML gelesen wird, auch für Titel und Anriss. Außerhalb von `cleaning.rs` darf niemand fremdes HTML lesen; `clippy.toml` verbietet die Aufrufe.
- **Erneutes Bereinigen ändert nichts.** Was herauskommt, wird als HTML aufgeschrieben und vom Browser wieder gelesen, und in seltenen Fällen lesen die Regeln von HTML es anders, als es gemeint war (eine Tabelle kann nicht in einem Absatz stehen). Deshalb wird das Ergebnis erneut bereinigt, bis es gleich bleibt, in aller Regel eine Runde. Ändert es sich nach vier Runden noch, bleiben nur die Worte.
- **Die Prüfsumme geht über das Bereinigte.** Ob ein Artikel eine neue Version bekommt, entscheidet die Prüfsumme über Titel und bereinigten Text. Was die Bereinigung entfernt, zählt also nicht: Ein Tracking-Token, das bei jedem Abruf wechselt, erzeugt keine Scheinversion.
- **Geänderte Regeln erzeugen keine Version.** Liest sich ein Artikel nach einer Änderung der Regeln anders, wurde aber gleich gesendet wie zuvor, stellt der nächste Abruf nur den Text neu hin.
- **Das Original bleibt.** `article_versions` hält den Text, wie er kam. `rueder reclean` bereinigt den Bestand daraus neu, alle Feeds oder einen; Versionen und Lesestand bleiben unberührt.

Die Listen für gesperrte Adressen und Tracking-Parameter sind Daten, keine Programmzeilen: `server/rules/blocked-links.txt` und `server/rules/tracking-parameters.txt`, mit einer Regel je Zeile. Sie sind ins Programm eingebaut, damit es ohne weitere Dateien läuft. Wer eigene Listen will, legt beide Dateien in ein Verzeichnis und nennt es in `CLEANING_RULES_DIR`; der Server liest sie beim Start.

Abweichungen vom Entwurf und vom alten Cleaner:

- Im gespeicherten Text zeigen Bilder auf ihre Quelle. Die Adresse des [Bild-Proxy](#bild-proxy) kommt erst beim Ausliefern hinein; ausgeliefert wird ein Text nie mit den eigenen Adressen seiner Bilder.
- Der alte Cleaner löste relative Adressen von der Adresse des Feeds aus auf. Ein Text, der `bilder/1.png` sagt, wurde aber für die Seite des Artikels geschrieben, und Feeds liegen oft bei einem anderen Anbieter.
- Der alte Cleaner entfernte jeden Link ohne gültiges Ziel samt Text. Das riss Worte aus Sätzen; jetzt fällt nur bei gesperrten Adressen der Text mit weg.
- Der alte Cleaner merkte sich, ob ein Text ein Bild enthält, um das Aufmacherbild nicht doppelt zu zeigen. Das entscheidet seit Paket 3.1 das Frontend: Es zeigt das Aufmacherbild nur zu einem Text ohne eigenes Bild (`ArticleBody.svelte`).

## Bild-Proxy

Jedes Bild, das jemand sieht, kommt von rueder und nie von der Seite, die es veröffentlicht hat. Die Seite erfährt so nicht, wer was liest. Gebaut in Paket 2.4 (`server/src/images.rs`).

- **Signierte Adressen.** `/img/<signatur>/<größe>/<adresse>`: Die Adresse ist die Quelle in Base64, die Signatur die ersten 16 Byte eines HMAC-SHA-256 über Größe und Quelle mit einem Schlüssel, den nur der Server kennt. Der Browser kann keine eigenen bilden, der Proxy ruft also nur ab, was in einem Artikel stand. Eine Signatur gilt für genau eine Größe.
- **Die Adresse kommt beim Ausliefern in den Text** (von Fiona am 2026-11-10 entschieden). Gespeichert bleibt der bereinigte Text mit den eigenen Adressen der Bilder. Erst wenn der Server einen Artikel ausliefert, setzt er die Adressen des Proxy ein (`Signer::in_html` für den Text, `Signer::address` für Vorschaubild, Aufmacherbild und Icon). Ein neuer Schlüssel oder eine andere Form der Adressen braucht deshalb kein erneutes Bereinigen des Bestands. Ein Bild, für das es keine Proxy-Adresse gibt, geht nicht hinaus.
- **Der Schlüssel.** `IMAGE_PROXY_KEY`, mindestens 32 Zeichen. Ohne ihn erzeugt der Server beim ersten Start einen eigenen und legt ihn als Datei `key` ins Cache-Verzeichnis, nur für sich lesbar. Er bleibt über Neustarts gleich, damit Adressen weiter gelten, die Browser sich gemerkt haben.
- **Abruf** über den gehärteten HTTP-Client, ohne Cookies und Referrer, mit festem User-Agent. Höchstens acht Quellen gleichzeitig. Eine Quelle, die fehlschlägt, wird fünf Minuten in Ruhe gelassen.
- **Nur Bilder.** PNG, JPEG, GIF und WebP, erkannt an den ersten Bytes und nie an dem, was die Quelle behauptet. Alles andere wird abgelehnt, auch SVG: Das wird nie durchgereicht und in dieser Fassung auch nicht gerastert.
- **Drei feste Größen.** `icon` passt in 256×256, `thumbnail` in 800×800 (beide wie die Presets des alten imgproxy), `image` in 2000×2000. Ein Bild, das passt, geht durch, wie es kam, samt Animation. Ein größeres wird verkleinert und neu geschrieben: als PNG, wenn es durchsichtige Stellen hat, sonst als JPEG; von einer Animation bleibt dabei das erste Bild. Die Ausrichtung aus den Kameradaten wird vorher angewendet.
- **Grenzen.** Höchstens 20 MB je Datei und 30 Millionen Pixel. Die Maße werden aus dem Kopf der Datei gelesen und geprüft, bevor irgendetwas dekodiert wird; eine kleine Datei, die sich zu Gigabytes entpackt, wird so in Millisekunden abgelehnt. Dekodiert wird abseits der Threads, die Anfragen bedienen, höchstens zwei Bilder gleichzeitig.
- **Cache auf Platte** in `IMAGE_CACHE_DIR`, eine Datei je Bild und Größe. Über `IMAGE_CACHE_MB` (Standard 1024) hinaus fallen die am längsten ungenutzten Dateien weg, bis der Cache bei vier Fünfteln der Grenze liegt. Er enthält nichts, was sich nicht neu holen ließe.
- **Nur für eingeloggte User** (seit Paket 2.5). Ohne Sitzung antwortet der Proxy mit 401, auch aus dem Cache und auch auf eine richtig signierte Adresse; die Quelle wird dann gar nicht gefragt.
- **Auslieferung** mit dem Typ, den die Bytes haben, `X-Content-Type-Options: nosniff`, `Content-Security-Policy: default-src 'none'; sandbox`, `Cross-Origin-Resource-Policy: same-origin` und `Cache-Control: private, max-age=31536000, immutable`. Abgelehnt wird mit 404 (keine Adresse des Servers), 502 (Quelle nicht erreichbar oder nicht erlaubt), 415 (kein Bild) und 413 (zu groß), immer ohne Inhalt.

Abweichungen vom Entwurf:

- Die Adresse wird nicht bei der Bereinigung erzeugt, sondern beim Ausliefern (siehe oben).
- `.ico`-Dateien, wie sie viele Seiten als Icon haben, sind keines der vier Formate und werden abgelehnt. Das Frontend zeigt dann seinen Ersatz für Feeds ohne Icon.

## Live-Updates

Was sich für ein Konto ändert, erreicht sofort alle seine offenen Tabs und Geräte: über eine SSE-Verbindung je Tab (`GET /api/events`). Gebaut in Paket 2.7 (`server/src/events.rs`).

Woher die Ereignisse kommen:

- **Vom Abruf.** Eine Runde des Abrufs meldet den Abonnenten eines Feeds neue Artikel, neue Versionen und den Beginn oder das Ende eines Fehlers.
- **Von der API, an einer einzigen Stelle.** Um die ganze API liegt eine Schicht, die nach jeder Anfrage, die etwas ändert und gelingt, ein Ereignis für das Konto veröffentlicht. Was sich geändert hat, liest sie allein an der Adresse der Anfrage ab. Kein Endpunkt braucht dafür eigenen Code, und einer, der später dazukommt, meldet sich von selbst. Eine Adresse, die die Schicht nicht kennt, zählt als Änderung des Baums: Zu viel neu zu laden schadet nicht, zu wenig ließe einen Tab etwas zeigen, das nicht mehr stimmt. Ausgenommen sind Fragen, die wie Änderungen gesendet werden (Vorschau einer Regel, Feed-Suche), und was nur Admins sehen.

Ereignisse sind Hinweise, nicht die Daten selbst. Sie sagen, was sich geändert hat, und tragen nur die wenigen Zahlen, die eine Anfrage sparen:

| Ereignis | Auslöser | Inhalt | Was der Client tut |
|---|---|---|---|
| `articles` | Abruf hat Artikel eingefügt | `feed_id`, `last_seq`, `new` | Ungelesen-Zahl des Feeds aus dem neuen `last_seq` berechnen, Hinweis „N neue Artikel“ über der Liste zeigen. Die Liste wird nicht weggeschoben. |
| `updated` | Neue Version von Artikeln eines Feeds | `feed_id`, `count` | Zeigt er Artikel dieses Feeds, lädt er die sichtbare Liste neu und bekommt so den Hinweis „aktualisiert“. |
| `feed` | Abruffehler beginnt oder endet | `feed_id`, `error` (Grund oder leer) | Warnzeichen setzen oder entfernen. |
| `reading` | Gelesen, wieder ungelesen, Stern, „alle als gelesen“ | `feed` (Nummern des Feeds nach der Änderung: `last_seq`, `read_upto`, `unread`; leer, wenn es um mehrere Feeds ging), `seq` (der Artikel, wenn es um einen ging), `by` | Zahlen des Feeds übernehmen oder, ohne `feed`, den Baum neu laden; die sichtbare Liste neu laden, wenn sie betroffen ist. |
| `tree` | Ordner oder Abo angelegt, umbenannt, verschoben, gelöscht | `by` | Baum neu laden. |
| `tree`, `labels`, `settings`, `rules` zugleich | Umzug eines Kontos (`POST /api/import`) | `by` | Was jedes einzelne sagt. |
| `labels` | Label angelegt, geändert, gelöscht, zugeordnet | `by` | Labels und die sichtbare Liste neu laden. |
| `settings` | Einstellungen oder die Zustimmung zu Fehlerberichten geändert | `by` | Einstellungen und Zustimmung neu laden. |
| `rules` | Filterregel geändert | `by` | Regeln und Baum neu laden, denn Regeln blenden Artikel aus. |
| `session` | Eine Sitzung des Kontos wurde beendet: Logout, Beenden von einem anderen Gerät aus, neues Passwort | `by` | Mit `GET /api/me` prüfen, ob die eigene noch gilt. |
| `hello` | Verbindung steht | | Nichts; die ID ist der Stand, ab dem Ereignisse kommen. |
| `resync` | Der Server kann nicht nachliefern | | Alles neu laden: Baum, Zahlen, sichtbare Liste. |

So greift es ineinander:

- **Wer was hört.** Jedes Ereignis nennt die Konten, für die es gilt, und eine Verbindung reicht nur weiter, was ihr eigenes nennt. Ereignisse des Abrufs gehen an alle, die den Feed abonnieren.
- **Der eigene Tab.** Ein Tab kann sich in der Kopfzeile `X-Rueder-Tab` einen Namen geben (bis 64 Zeichen aus Buchstaben, Ziffern, `-` und `_`). Der Name kommt im Ereignis als `by` zurück; so erkennt der Tab seine eigene Änderung und lädt nichts doppelt.
- **Nachliefern.** Ereignisse tragen fortlaufende Nummern, die ID ist die Kennung des Serverlaufs plus Nummer. Der Server hält die letzten 2.000. Nach einem Abbruch meldet sich der Browser von selbst mit `Last-Event-ID` und bekommt, was er verpasst hat, in der richtigen Reihenfolge. Ist das nicht mehr da oder stammt die ID aus einem früheren Lauf des Servers, bekommt er `resync`.
- **Reihenfolge beim Start.** Der Client öffnet erst die Verbindung und lädt dann Baum und Zahlen. Umgekehrt ginge verloren, was dazwischen geschieht.
- **Grenzen.** Höchstens 20 Verbindungen je Konto, die nächste bekommt 429. Liest eine Verbindung zu langsam und stauen sich mehr als 256 Ereignisse, bekommt sie `resync`, statt den Server aufzuhalten. Alle 20 Sekunden hält eine leere Zeile die Verbindung bei Proxys offen.
- **Sitzung.** Die Verbindung gibt es nur mit Sitzung. Nach 30 Minuten schließt der Server sie; der Browser öffnet die nächste von selbst, bekommt nachgeliefert, und der Server prüft die Sitzung dabei erneut. Endet eine Sitzung, durch Logout, von einem anderen Gerät aus oder durch ein neues Passwort, enden ihre Verbindungen sofort.

Abweichungen vom Entwurf:

- Der Entwurf ließ offen, ob Ereignisse die Daten tragen. Sie tragen sie nicht: Beim Lesestand kommen die Nummern des Feeds mit, aber nicht die Ausnahmen, und der Baum wird neu geladen statt nachgeführt. Das hält Server und Client einfach, und ein Tab kann nie von einem verpassten Ereignis falsch werden.
- `updated` nennt den Feed und die Zahl der geänderten Artikel, nicht die einzelnen Artikel.
- Ereignisse des Abrufs entstehen nur in den Runden des Servers, nicht bei einem einzelnen Abruf von Hand.
- Läuft rueder später in mehreren Prozessen, müssen `LISTEN/NOTIFY` von PostgreSQL die Ereignisse zwischen ihnen tragen. Heute hält ein Prozess sie im Speicher; nach einem Neustart bekommen alle Verbindungen `resync`.

## Login und Sitzungen

Gebaut in Paket 2.5 (`server/src/accounts.rs`, `server/src/sessions.rs`, `server/src/login.rs`, `server/src/oidc.rs`).

- **Lokale Konten** mit Passwort-Hash nach Argon2id (19 MiB, 2 Durchläufe, ein Salt je Hash). Konten legt die Kommandozeile an (`rueder account add`, `password`, `link`, `list`) oder ein Admin; selbst anlegen kann sich jemand nur ein Konto, wenn der Betreiber es erlaubt, siehe [Registrierung und Einladungen](#registrierung-und-einladungen). Ein Name ist höchstens 64 Zeichen lang und unterscheidet keine Groß- und Kleinschreibung, ein Passwort hat mindestens 8 Zeichen und höchstens 1024 Byte. Höchstens vier Hashes werden gleichzeitig gerechnet, damit Login-Versuche den Speicher nicht füllen.
- **Sitzung** als zufälliges Token (32 Byte) im Cookie `__Host-rueder` mit `HttpOnly`, `Secure`, `SameSite=Lax` und `Path=/`. In der Datenbank liegt nur der SHA-256 des Tokens. Eine Sitzung endet nach 30 Tagen ohne Nutzung, beim Abmelden und sobald das Konto ein neues Passwort bekommt; ein neuer Login ersetzt die Sitzung, die der Browser noch hatte. Dass eine Sitzung genutzt wurde, schreibt der Server höchstens einmal je Stunde auf. Abgelaufene Sitzungen räumt er stündlich weg.
- **Wer eingeloggt ist**, sagt der Extraktor `Session` mit der Nummer des Kontos. Jeder Handler, der ihn nimmt, antwortet ohne Sitzung mit 401; an seine Kontonummer binden die Endpunkte aus Paket 2.6 ihre Abfragen. Der Bild-Proxy unter `/img/` nimmt ihn schon.
- **Login** über `POST /api/login` mit Name und Passwort als JSON, `POST /api/logout`, `GET /api/me`. `GET /api/login/ways` sagt der Login-Seite ohne Sitzung, welche Wege es außer Name und Passwort gibt (`{"oidc": true}`), und sonst nichts. Ein falscher Name, ein falsches Passwort und ein Konto ohne Passwort bekommen dieselbe Antwort nach derselben Arbeit: Für einen Namen, den es nicht gibt, wird gegen einen Hash geprüft, dessen Passwort niemand kennt.
- **Rate-Limit** auf falsche Passwörter: 10 je Name und 30 je Adresse in 15 Minuten, danach 429 mit `Retry-After`, auch für das richtige Passwort. Bei IPv6 zählt ein /64 als eine Adresse. Gezählt wird jeder Versuch, bevor das Passwort geprüft ist (`login::Limiter::attempt`); ein Login, der klappt, setzt den Zähler des Namens zurück und gibt der Adresse ihren Versuch wieder. So kommen auch Versuche, die alle zugleich unterwegs sind, nicht über die Grenze. Dasselbe gilt für das alte Passwort beim Ändern. Gezählt wird im Speicher des Servers, für höchstens 10.000 Namen und Adressen; ist er voll, weicht der Eintrag mit den wenigsten Versuchen, ein gesperrter Name bleibt gesperrt. Hinter einem Reverse-Proxy kommt jede Verbindung vom Proxy; `CLIENT_IP_HEADER` nennt den Header, in den der Proxy die Adresse des Absenders schreibt (etwa `X-Forwarded-For`), und es zählt dessen letzter Eintrag, also der, für den der Proxy selbst bürgt.
- **OpenID Connect** als Option, eingestellt über `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` und `OIDC_REDIRECT_URL`. Authorization Code Flow mit PKCE: `GET /api/oidc/start` schickt den Browser zum Anbieter, `GET /api/oidc/callback` tauscht den Code gegen das ID-Token. State, Nonce und Code-Verifier eines Logins liegen zehn Minuten lang in einem eigenen Cookie (`__Host-rueder-oidc`, dieselben Attribute). Geprüft werden State, Aussteller, Empfänger (`aud`, `azp`), Ablauf und Nonce. Der Weg endet mit einer Weiterleitung auf `/`, bei einem Fehler auf `/?oidc=failed`, `denied` oder `unknown`.
- **Niemand bekommt durch OpenID Connect ein Konto.** `account_identities` sagt, welches Konto zu Aussteller und Subject gehört. Die Verknüpfung entsteht auf der Kommandozeile (`rueder account link <name> <subject>`) oder dadurch, dass jemand eingeloggt ist und den Weg mit `/api/oidc/start?link` beginnt. Ein unbekanntes Subject wird abgewiesen.
- **Verknüpfen nur von rueders eigener Seite** (seit Paket 2.12). `GET /api/oidc/start?link` ist eine lesende Anfrage und geht deshalb an der Prüfung der Herkunft vorbei. Ohne eigene Prüfung könnte eine fremde Seite einen eingeloggten Reader dorthin schicken, und sein Konto bekäme die Identität, als die sein Browser gerade beim Anbieter angemeldet ist; hat ein Angreifer ihm dort die eigene untergeschoben, gehört ihm danach das Konto. Der Weg beginnt deshalb nur, wenn der Browser mit `Sec-Fetch-Site` sagt, dass die Anfrage von rueder selbst kommt oder von Hand (`same-origin`, `none`), und eine Sitzung mitbringt. Ein Browser ohne diesen Header kann nicht verknüpfen; dafür bleibt die Kommandozeile. Der Cookie des Wegs merkt sich das Konto, und verknüpft wird nur mit ihm, wenn es beim Rückweg noch eingeloggt ist.
- Der Anbieter wird wie alles andere über den [gehärteten HTTP-Client](#gehärteter-http-client) erreicht. Der kann dafür Formulare senden; ein solcher Abruf folgt keiner Weiterleitung, damit Code und Secret bei dem bleiben, für den sie bestimmt sind.
- Das Frontend sieht nie ein Token. JWTs als Sitzung entfallen.

Abweichungen vom Entwurf, und was offen ist:

- **Die Signatur des ID-Tokens wird nicht eigens geprüft** (von Fiona am 2026-11-10 bestätigt, ebenso dass durch OpenID Connect nie ein Konto entsteht). Das Token kommt beim Code Flow direkt vom Token-Endpunkt des Anbieters über TLS und nie durch den Browser; OpenID Connect Core 3.1.3.7 lässt dafür die Prüfung des TLS-Zertifikats an Stelle der Signatur zu. Das erspart die Abhängigkeiten für JWT-Signaturen und Schlüsselabruf. Dafür müssen alle Adressen des Anbieters https sein, sonst startet der Server nicht. Das Security-Review vom 2026-11-12 hat sie noch einmal angesehen und lässt sie stehen: Das Token holt der Server selbst, mit Client-Secret und PKCE, über den gehärteten Client mit den Zertifizierungsstellen von Mozilla; der Anbieter muss sich mit genau dem eingestellten Aussteller melden, und geprüft werden Aussteller, Empfänger, Ablauf und Nonce. Wer das Token fälschen wollte, müsste die TLS-Verbindung zum Anbieter fälschen und hätte dann auch dessen Schlüssel in der Hand.
- Der Cookie verlangt HTTPS, weil `Secure` und das Präfix `__Host-` fest sind. Ohne TLS lässt sich nur über `localhost` einloggen, wo Browser eine Ausnahme machen.
- Das Rate-Limit je Name kann ein Konto 15 Minuten lang aussperren, wenn jemand absichtlich falsche Passwörter schickt. Wer schon eine Sitzung hat, merkt davon nichts.
- **Eigene Sitzungen** (seit Paket 2.6, `server/src/profile.rs`). `GET /api/sessions` zeigt, wo ein Konto eingeloggt ist, mit Beginn und letzter Nutzung; `DELETE /api/sessions/{id}` beendet eine davon, `DELETE /api/sessions` alle anderen. Der Name einer Sitzung in dieser Liste sind die ersten acht Byte des SHA-256 ihres Tokens; aus ihm lässt sich kein Token bilden. Gerät oder Browser merkt sich der Server nicht.
- **Neues Passwort.** `PUT /api/password` verlangt das bisherige Passwort, zählt Fehlversuche wie der Login (eine gestohlene Sitzung kann also nicht endlos raten), beendet alle Sitzungen des Kontos und gibt dem fragenden Browser eine neue. Ein Konto ohne Passwort, etwa nach einem Login über OpenID Connect, setzt sein erstes ohne das alte.
- **Konten anlegen.** `GET` und `POST /api/accounts` sind einem Admin vorbehalten: die Liste der Konten und ein neues Konto mit einem ersten Passwort, das der Admin weitergibt. Der Einmal-Link aus den [Detailfragen](plan.md#detailfragen) ist für importierte Konten ohne Passwort gedacht und kommt mit dem Import (Paket 4.3); bis dahin setzt `rueder account password` ein Passwort.
- Die Prüfung des `Origin`-Headers gibt es seit Paket 2.9, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). Seither hat ein Konto auch höchstens 100 Sitzungen; mit der nächsten endet die, die am längsten nicht genutzt wurde.

Wie die bestehenden Konten umziehen, steht bei den [Detailfragen](plan.md#detailfragen): Der Import führt sie mit den Konten zusammen, die es im neuen rueder schon gibt, und legt nur die übrigen ohne Passwort an.

### Registrierung und Einladungen

Von Fiona am 2026-11-12 entschieden, gebaut in `server/src/registration.rs` und der Migration `0003_registration.sql`. Bis der Betreiber etwas öffnet, ist alles zu: keine Registrierung, kein Kontingent für Einladungen.

- **Zwei Wege zu einem Konto.** Offene Registrierung, wenn der Betreiber sie einschaltet, und Einladungslinks. Beide führen über `POST /api/register` (Name, Passwort, optional das Token der Einladung); die Antwort setzt wie ein Login den Sitzungs-Cookie.
- **Zwei Kontingente, die sich nicht berühren.** Das allgemeine: Konten vom Betreiber, aus offener Registrierung und aus Einladungen von Usern zählen gegen `max_accounts`, die beiden letzten außerdem gegen `per_day` (neue Konten in 24 Stunden). Das der Admins: Konten aus Einladungen von Admins zählen nur gegen `admin_invite_accounts` und belegen keinen Platz im allgemeinen. Woher ein Konto stammt, steht in `accounts.origin`.
- **Einladungen von Usern.** Jeder User darf insgesamt so viele Konten einladen, wie sein Kontingent sagt: der Wert für alle (`invites_per_account`) oder ein eigener (`accounts.invite_quota`). Gezählt wird, was seine Links gebracht haben und noch bringen können; ein abgelaufener oder zurückgezogener Link gibt die Plätze zurück, die er nicht gebraucht hat. Die Grenzen für Gesamtzahl und Tag gelten für diese Einladungen wie für die offene Registrierung.
- **Einladungen von Admins** gehen immer, ohne Kontingent je Admin und unabhängig davon, ob die Registrierung offen ist. Ein Link bleibt der eines Admins, auch wenn sein Konto später keiner mehr ist.
- **Ein Link** ist `/?invite=<token>`. Das Token sind 32 zufällige Byte wie bei einer Sitzung; gespeichert ist nur sein SHA-256, gezeigt wird der Link genau einmal, beim Erzeugen. Er gilt für 1 bis 1000 Konten (Standard 1) und so viele Tage, wie eingestellt ist (Standard 7); wer ihn erzeugt hat, kann ihn zurückziehen.
- **Einstellungen in der Datenbank**, gesetzt mit `rueder registration set` (`--open`, `--max-accounts`, `--per-day`, `--invites`, `--invite-days`, `--admin-invite-accounts`) und für ein einzelnes Konto mit `rueder account invites <name> <n|default>`. Eine Änderung gilt sofort, ohne Neustart; `rueder registration show` zeigt Einstellungen und Stand.
- **Gleichzeitige Registrierungen** laufen nacheinander: Zählen und Anlegen geschehen in einer Transaktion, die die Zeile der Einstellungen sperrt. Es entstehen nie mehr Konten, als Platz ist, und ein Link bringt nie mehr als seine Zahl.
- **Ablehnungen** sind 403 mit einem Wort, das die Oberfläche in ihre Sprache übersetzt: `closed`, `invite`, `full`, `today`. Ob Platz ist, wird geprüft, bevor der Name angesehen und das Passwort gehasht wird: Wer nicht hineinkann, erfährt nicht, welche Namen vergeben sind, und lässt den Server keine Hashes rechnen. Ein vergebener Name ist 409, ebenfalls ohne Hash (seit Paket 2.12; vorher konnte eine Adresse mit vergebenen Namen die vier Plätze für Hashes belegen und so jeden Login bremsen).
- **Die Login-Seite** fragt `GET /api/login/ways` (`registration` ist wahr, wenn die Registrierung offen und Platz ist) und, wenn die Adresse eine Einladung trägt, `GET /api/register?invite=…`, ob der Link gilt. Die Einladungen eines Users stehen in den Einstellungen im Reiter „Einladungen“.

Was offen ist:

- Es gibt keine Bestätigung über E-Mail und kein CAPTCHA; rueder kennt keine E-Mail-Adressen. Gegen massenhaftes Registrieren stehen die Grenze je Tag, die Gesamtgrenze und das Rate-Limit je Adresse. Wer die Registrierung öffnet, sollte `--per-day` setzen.
- Offene Registrierung verrät, welche Namen vergeben sind (409), solange Platz ist. Das liegt in der Sache.
- Ein Admin sieht nicht, wer wen eingeladen hat; es steht in der Datenbank (`accounts.invite_id`), aber in keiner Ansicht.
- Das Security-Review vom 2026-11-12 hat diese Fläche angesehen; was es fand, steht unter [Security-Review vom 2026-11-12](#security-review-vom-2026-11-12).

## Umzug eines Kontos

Gebaut in Paket 3.12 (`server/src/transfer.rs`, Migration `0004_moving.sql`). Ein Konto zieht als eine JSON-Datei von einem rueder zum anderen: `GET /api/export` gibt sie aus, `POST /api/import` nimmt sie in das eingeloggte Konto, auf demselben rueder oder einem anderen. Für Admins tun `rueder account export <name>` und `rueder account import <name> <datei>` dasselbe auf der Kommandozeile, etwa um mehrere Konten auf einmal umzuziehen.

**Was in der Datei steht.** Die Einstellungen, der Baum von oben nach unten in seiner Reihenfolge (Ordner mit ihrem Pfad aus Namen, Abos mit Feed-Adresse, Website, Titel des Feeds, eigenem Namen, Ordnerpfad und User-Agent), die Labels mit Farbe, die Filterregeln mit ihrem Bereich als Ordnerpfad oder Feed-Adresse, und je Abo der Lesestand: der Artikel, an dem die Lesemarke steht, und die Artikel, die davon abweichen (gelesen über der Marke, ungelesen behalten darunter), die einen Stern oder Labels haben. Jeder Artikel steht mit Nummer im Feed, Kennung (GUID), Adresse seiner Seite, Zeit und Titel darin, jeder Feed mit seiner Nummer. Kopf: `format` (`rueder-account`), `version` (1) und `exported_at`.

**Was nicht darin steht.** Passwort, Sitzungen, die Identitäten bei einem Anbieter von OpenID Connect, das Recht als Admin, die Zustimmung zu Fehlerberichten (sie gilt dem Server, dem sie gegeben wurde), die Texte der Artikel und ihre alten Versionen und „ausgeblendet“, das die Regeln am Ziel neu entscheiden.

**Keine Texte** (von Fiona am 2026-11-12 entschieden, nach einem Befund des Security-Reviews). Bis dahin brachten Artikel mit Stern oder Label ihren Text mit, und einen, den das Ziel nicht hatte, legte der Import im Feed an. Ein Feed gehört aber allen, die ihn lesen: Wer die Datei selbst schrieb, konnte so erfundene Artikel unter dem Namen einer echten Quelle in Feeds stellen, die andere lesen, mit Links, wohin er wollte, und neue Abonnenten sahen sie oben und ungelesen. Die Bereinigung hält Skripte ab, aber keine falsche Meldung. Statt Kopien nur für das eigene Konto (eine Migration und eine Bedingung in jeder Liste) gilt jetzt: Ein Artikel, den das Ziel nicht hat, kommt nicht dazu, und was an ihm gelesen, markiert oder mit Labels versehen war, geht verloren. Eine Datei eines älteren Stands mit Texten wird gelesen; die Texte bleiben draußen.

**Nummern zählen nicht.** Die Nummern von Feeds und Artikeln sind die des Servers, von dem die Datei kommt; ein anderer rueder hat andere. Sie stehen nur zur Orientierung darin. Beim Import gilt:

- Ein **Feed** wird an seiner Adresse erkannt, wie beim Abonnieren. Gibt es ihn nicht, legt der Import ihn an, mit dem User-Agent aus der Datei, und die nächste Runde des [Abrufs](#abruf) holt ihn, wie beim OPML-Import. Ordner werden wie dort an ihrem Namen an ihrer Stelle wiedererkannt oder angelegt (dafür ist der OPML-Import in `subscribe::take` herausgelöst).
- Ein **Artikel** wird im Feed an seiner Kennung erkannt, sonst an der Adresse seiner Seite, aber nur, wenn kein anderer Artikel des Feeds dieselbe hat.
- Die **Lesemarke** steht am Artikel, an dem sie in der Datei stand. Ist er nicht zu finden, gilt seine Zeit: Die Marke rückt so weit, wie alles darunter nicht jünger ist (wie „älter als“ bei „alle als gelesen“). Hat ein Feed am Ziel noch nie einen Abruf hinter sich, merkt sich das Abo die Zeit in `subscriptions.read_before`, und der erste Abruf markiert, was davor erschien, als gelesen (`reading::settle_read_before`). Ohne das wären nach einem Umzug alle Feeds, die am Ziel neu sind, voller Ungelesener.
- **Ausnahmen, Sterne und Labels** werden an den gefundenen Artikeln gesetzt. Was nicht zu finden ist, geht verloren; die Antwort zählt es. Der Import schreibt nie in die Artikel eines Feeds.

**Zusammenführen.** Ein Import fügt hinzu und nimmt nichts weg: Ein Abo, das das Konto schon hat, bleibt an seinem Platz, seine Lesemarke rückt nur vor, Sterne und Labels kommen dazu. Labels werden am Namen erkannt, Regeln, die es genau so schon gibt, nicht doppelt angelegt. Nur die Einstellungen werden ersetzt, wenn die Datei welche hat. Alles geschieht in einer Transaktion, am Ende bewerten die Regeln alles neu (`filters::reapply`). Ein zweiter Import derselben Datei ändert nichts mehr. Die Antwort zählt neue und bekannte Feeds, neue Ordner, gefundene und verlorene Artikel, neue Labels und Regeln, was ausgelassen wurde und ob die Einstellungen kamen. Andere Tabs des Kontos laden alles neu: `["import"]` ist in `events::changed` alles zugleich (`Changed::Everything`), die Schicht der API meldet `tree`, `labels`, `settings` und `rules`.

**Grenzen.** Die Datei darf 32 MB groß sein (`transfer::MAX_FILE`) und höchstens 2.000 Feeds, 100 Ordner, 100 Labels, 200 Regeln und 50.000 Artikel nennen (`transfer::MAX_ARTICLES`); was darüber liegt, lehnt der Import als Ganzes ab, ebenso eine fremde `format` oder `version`. Was nur falsch ist (ein Feed ohne abrufbare Adresse, ein Artikel ohne Kennung, ein Label mit falscher Farbe, eine Regel mit kaputtem Muster oder ohne ihren Ordner), wird ausgelassen und gezählt. Ordnerpfade werden nach fünf Ebenen abgeschnitten. Weil ein Import viel auf einmal schreibt, darf ein Konto drei auf einmal und danach einen alle zwei Stunden (`rate::IMPORTS`), und Export wie Import zählen zu den [teuren Anfragen](#header-herkunft-und-limits). Die Datei muss als `application/json` kommen, was ein Formular einer fremden Seite nicht kann.

**Als Download.** Die Datei kommt mit `Content-Disposition: attachment` und der Policy `default-src 'none'` der API, so dass ein Browser, der sie öffnet, nichts daraus lädt. Seit sie keine Texte mehr trägt, nennt sie auch keine Bilder mehr.

## Security

Der Server ruft Adressen ab, die User eingeben, und zeigt HTML an, das Fremde geschrieben haben. Diese beiden Stellen sind die größten Angriffsflächen und bekommen je eine zentrale, getestete Komponente. Gegen diese Tabelle laufen die Security-Reviews am Ende von Schritt 2 und vor der Inbetriebnahme.

| Angriffsfläche | Maßnahme |
|---|---|
| Login und Sitzung | Sitzung als Cookie mit `HttpOnly`, `Secure` und `SameSite=Lax`. Das Frontend sieht kein Token mehr. Passwörter mit Argon2id, Rate-Limit auf dem Login, das jeden Versuch zählt, bevor er geprüft ist, OpenID Connect als Option. |
| CSRF | SameSite-Cookie und Prüfung der Herkunft (`Sec-Fetch-Site`, sonst `Origin`) bei jeder ändernden Anfrage. Gebaut, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). |
| Zugriff auf Daten | Jede Abfrage ist an die Konto-ID gebunden. Feeds und Artikel sind nur für Abonnenten lesbar. Kein Endpunkt listet fremde Feeds. Auch der Status für Admins zählt nur, wie viele Feeds aus welchem Grund fehlschlagen, nie welche. |
| Abruf fremder Adressen (SSRF) | Ein gemeinsamer HTTP-Client für Feed-Abruf, Feed-Suche und Bild-Proxy. Er erlaubt nur http und https, löst DNS selbst auf, sperrt private, lokale und Link-Local-Adressen und verbindet sich genau mit der geprüften IP. Jede Weiterleitung wird neu geprüft. Grenzen für Größe, Zeit, Weiterleitungen und Dekompression. Gebaut, siehe [Gehärteter HTTP-Client](#gehärteter-http-client). |
| Artikel-HTML (XSS) | Bereinigung auf dem Server; der letzte Schritt ist immer `ammonia` nach fester Allowlist, ohne `id`, `class` und `style`, und nur vollständige `http`- und `https`-Adressen bleiben. Zweite Linie ist eine strenge Content-Security-Policy ohne Inline-Skripte, Bilder nur vom eigenen Proxy. Links bekommen `rel="noopener noreferrer"`. |
| Bild-Proxy | Signierte Adressen, der Schlüssel bleibt auf dem Server. Durchgelassen werden nur echte Bildformate nach Prüfung der ersten Bytes, SVG nur gerastert oder gar nicht. Größenlimit und Cache auf Platte. Zur Quelle gehen keine Cookies, kein Referrer und ein fester User-Agent. Auslieferung mit `nosniff` und `Content-Security-Policy: sandbox`. |
| Privacy beim Lesen | Der Browser spricht nur mit dem eigenen Server. Audio und Video laden erst nach Klick, weil sie direkt von der Quelle kommen. `Referrer-Policy: no-referrer`. |
| Filterregeln der User | Reguläre Ausdrücke laufen mit dem `regex`-Crate in garantiert linearer Zeit. Limits für Länge und Anzahl. |
| Feed-Parser | Limits für Feed-Größe und Artikelzahl, keine externen XML-Entitäten. |
| HTML-Parser | Grenzen für Tiefe und Größe des Baums, den ein Text ergibt (siehe [Bereinigung](#bereinigung)). Gelesen wird fremdes HTML nur in `cleaning.rs`, auf einem Thread, der den Server nicht aufhält. |
| Unerwartete Eingaben (Panics, Hänger, Speicherverbrauch) | [Fuzzing](#fuzzing) von Bereinigung, Feed-Verarbeitung und Bild-Proxy, Property-Test für die Adressprüfung des HTTP-Clients. |
| Dauerbetrieb (Speicher, Dateien, Verbindungen, die mit der Zeit ausgehen) | [Soak-Test](#soak-test) des ganzen Programms unter der Last von Lesern und Abruf. |
| Missbrauch | Limits je Konto für Feeds, Ordner, Regeln, Labels und Sitzungen. Rate-Limits je Adresse auf allem und je Konto auf Abrufen fremder Adressen. Teure Anfragen nur wenige zugleich, je Konto und auf dem ganzen Server. Gebaut, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). |
| Lieferkette und Betrieb | `cargo audit` und `cargo deny` in der CI, Lockfiles, Container ohne Root mit schreibgeschütztem Dateisystem. Geheimnisse kommen nie in den Frontend-Build. |

### Security-Review vom 2026-11-12

Paket 2.12, gegen die Tabelle oben, am Code von `main` nach Pull Request #39: alle Module des Servers mit ihren Abfragen und Endpunkten, die Schichten um den Router, der HTTP-Client, der Bild-Proxy, die Bereinigung bis zu ihrer Allowlist, der Login mit OpenID Connect, Registrierung und Umzug, und im Frontend die Stellen, an denen fremder Text in die Seite kommt. Gelesen und mit Tests belegt, nicht mit einem Werkzeug von außen geprüft. Das zweite Review vor der Inbetriebnahme (Paket 4.1) beginnt bei den Restrisiken unten.

**Befunde, behoben:**

| Nr. | Befund | Schwere | Was geändert ist |
|---|---|---|---|
| S1 | Die Grenze für falsche Passwörter zählte einen Versuch erst, nachdem das Passwort geprüft war. Versuche, die alle zugleich unterwegs waren, kamen deshalb alle durch: aus einer Adresse bis zu 1200 auf einmal statt 10 je Name. Dasselbe beim alten Passwort, wenn eine fremde Sitzung das Passwort ändern will. Dazu konnte eine Flut anderer Namen den Zähler eines gesperrten Namens aus dem Speicher drängen. | hoch | Jeder Versuch zählt vorher, ein richtiger gibt seinen zurück; aus dem vollen Speicher weicht der Eintrag mit den wenigsten Versuchen. Test mit 30 gleichzeitigen Versuchen. Siehe [Login und Sitzungen](#login-und-sitzungen). |
| S2 | Der Umzug legte Artikel mit Stern oder Label, die das Ziel nicht hatte, mit ihrem Text im geteilten Feed an. Mit einer selbst geschriebenen Datei ließen sich so erfundene Artikel in Feeds anderer stellen. | hoch | Von Fiona entschieden: Der Umzug bringt keine Texte mehr mit, siehe [Umzug eines Kontos](#umzug-eines-kontos). |
| S3 | `GET /api/oidc/start?link` ließ sich von einer fremden Seite auslösen; zusammen mit einem untergeschobenen Login beim Anbieter hätte das ein fremdes Konto übernommen. Verknüpft wurde außerdem mit dem, der beim Rückweg eingeloggt war, nicht mit dem, der den Weg begann. | mittel | Nur mit `Sec-Fetch-Site` `same-origin` oder `none` und einer Sitzung; der Weg merkt sich das Konto. Siehe [Login und Sitzungen](#login-und-sitzungen). |
| S4 | Teure Anfragen hatten keine Grenze. Ein Konto konnte mit wenigen gleichzeitigen Anfragen an die Treffer der Regeln oder die Versionen den Pool der Datenbank und die Prozessoren belegen. | mittel | Höchstens drei zugleich auf dem Server, zwei je Konto, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). |
| S5 | Eine Registrierung mit vergebenem Namen rechnete den Hash, bevor sie abgelehnt wurde. Eine Adresse konnte so die vier Plätze für Hashes belegen und jeden Login bremsen, ohne dass etwas gezählt wurde. | niedrig | Der Name wird vor dem Hash geprüft. |
| S6 | Lehnte der HTTP-Client einen Namen ab, weil er auf eine interne Adresse zeigt, nannte die Meldung diese Adresse (`intranet is at 10.0.0.5`). Über die Feed-Suche ließ sich so das DNS im Netz des Servers abfragen. | niedrig | Die Meldung nennt Namen und Grund, nicht die Adresse. |
| S7 | Beim Fuzzing zum Review: Ein Feed, der kein gültiges UTF-8 ist und keine andere Kodierung nennt, ging unverändert an `feed-rs`. Die Bibliothek lässt ein Byte, das sie nicht dekodieren kann, weg und liest weiter; aus `enc\xC0oding` wurde so `encoding`, eine Deklaration nach dem Anfang, und die Bibliothek brach mit einer Panic ab. Ein Feed konnte damit den Abruf und den Server zu Fall bringen. | hoch | Solche Bytes werden vorher Ersatzzeichen, die Bibliothek bekommt immer gültiges UTF-8 (siehe [Abruf](#abruf)). Als `found-broken-declaration` im Korpus. |

**Angesehen und in Ordnung:** jede Abfrage nach Daten eines Kontos ist an seine Nummer gebunden, auch über Ordner, Labels und Regeln, und die Schlüssel des Schemas verhindern Ordner und Abos über Konten hinweg; Sitzungen nur als Hash, mit `__Host-`-Cookie; die Prüfung der Herkunft und die Header an jeder Antwort; die Policy und das einzige `{@html}` des Frontends für bereinigte Texte; Adressen von Artikeln, Anhängen und Bildern nur als `http` und `https`; die Signatur des Bild-Proxy mit HMAC und Vergleich in fester Zeit, das Format nach den ersten Bytes und die Grenzen vor dem Dekodieren; der HTTP-Client mit eigener Namensauflösung, Prüfung jeder Weiterleitung und ohne Zugangsdaten bei Weiterleitungen; die Prüfung des ID-Tokens (siehe oben); die Grenzen von Feeds, OPML, Umzug und Fehlerberichten; der Container ohne Root und mit schreibgeschütztem Dateisystem.

**Restrisiken, bewusst offen:**

- **Geteilte Feeds verraten, was gelesen wird.** Wer eine Adresse abonniert, die schon jemand auf diesem Server liest, bekommt sofort eine Antwort und ihre Geschichte, ohne Abruf. So lässt sich erfahren, ob jemand eine bestimmte Adresse liest, nicht wer. Das folgt aus geteilten Feeds.
- **OPML und Umzug rufen ohne `LOOKUPS` ab.** Ein Import legt bis zu 2000 Feeds an, die die nächste Runde des Abrufs holt. Mit Abbestellen und neuem Import kann ein Konto den Server nach und nach viele fremde Adressen abrufen lassen: höchstens 2000 zugleich, jede höchstens alle 15 Minuten und immer über den gehärteten Client.
- **Langsame Bildquellen.** Der Bild-Proxy holt höchstens acht Bilder zugleich, für alle. Wer einen eigenen Feed voller Bilder von einem Server abonniert, der jede Antwort 30 Sekunden hinzieht, macht Bilder für alle langsam, die nicht im Cache sind. Abhilfe wäre eine Grenze je Host.
- **Viele Adressen gegen den Login.** Die Grenze je Name hält, aber wer sehr viele Adressen hat (ein IPv6-/48 hat 65.536 Netze), kann mit falschen Passwörtern die vier Plätze für Hashes belegen und Logins aller langsam machen, und einen Namen absichtlich sperren. Wer eine Sitzung hat, merkt davon nichts.
- **Offene Live-Verbindungen** sehen erst bei einem Ereignis zu Sitzungen oder nach spätestens 30 Minuten, dass ihre Sitzung vorbei ist, etwa nach einem neuen Passwort von der Kommandozeile. Ereignisse sind nur Hinweise; jede Anfrage nach Daten braucht die Sitzung.
- **Kleinigkeiten:** `GET /api/health` nennt ohne Sitzung die Zahl der Feeds. Ein Admin, dem das Recht genommen wird, behält Einladungen, die gegen das Kontingent der Admins zählen. Offene Registrierung verrät vergebene Namen (siehe [Registrierung und Einladungen](#registrierung-und-einladungen)).

### Nachtrag: App, Service Worker, CI und Deploy

Auf Fionas Wunsch vom 2026-11-12 nach dem Merge von Pull Request #40, weil das Review oben diese Teile nicht angesehen hatte: die Android-App in `app/` (Startseite, Plugin, WebView-Client, Adressprüfung, Updates, Manifest, Backup), der Service Worker des Frontends mit `web/src/lib/offline.ts`, die Workflows in `.github/workflows/` mit Dependabot und die Vorlage in `deploy/`. Wieder gelesen und, wo es geht, mit Tests belegt; die App konnte hier nicht gebaut werden, das tut die CI.

**Befunde, behoben:**

| Nr. | Befund | Schwere | Was geändert ist |
|---|---|---|---|
| S8 | Das Update der App lag nach dem Download im Ordner der App auf dem geteilten Speicher und ging von dort an Androids Installer. Bis Android 9 darf dort jede App mit Speicher-Berechtigung schreiben. Eine solche App konnte das APK zwischen Download und Installation gegen eines mit anderem Paketnamen tauschen: Androids Installer prüft den Schlüssel nur bei einem Update desselben Pakets und hätte die fremde App installiert, nach einer Frage, die wie das Update aussah. | mittel | Die App kopiert das APK in ihren eigenen Cache, prüft dort Paketname und Build gegen `version.json` und gibt nur diese Kopie weiter; der `FileProvider` gibt nur noch diesen Ordner frei (`Updates.java`, `Release.describes`, Test in `ReleaseTest.java`). |
| S9 | Der Job `app` gab den Signier-Schlüssel der App auch Pull Requests aus Zweigen dieses Repositorys. Ein Pull Request führt seine eigenen Build-Skripte aus, bevor jemand sie gemergt hat; eine geänderte `build.gradle` oder eine Abhängigkeit mit eigenem Skript hätte den Schlüssel lesen können, mit dem jedes künftige Update signiert sein muss. | mittel | Schlüssel und Passwörter gibt es nur bei einem Push auf `main`; ein Pull Request baut immer ein Debug-APK. |
| S10 | `ci.yml` und `gitleaks.yml` sagten nicht, was der Token eines Jobs darf; es galt die Voreinstellung des Repositorys, bei älteren Repositorys Schreiben. Jede Action eines Drittanbieters in diesen Jobs hätte damit ins Repository schreiben können. | niedrig | Beide lesen nur noch, wenn ein Job nicht mehr sagt; nur `app-release` und `publish` schreiben, jeder in das, was er veröffentlicht. |

**Angesehen und in Ordnung:** Die Startseite der App setzt alles mit `textContent`, auch die Adresse aus `?unreachable=`; die Adresse prüfen Startseite und native Seite (nur der Ursprung einer https-Adresse, ohne Zugangsdaten, Leer- oder Steuerzeichen). Andere Adressen öffnet die App im Browser, Artikel-Links sind nach der Bereinigung nur `http` und `https`. Die App hat nur die Berechtigung fürs Internet (die Build `github` dazu die zum Installieren und zum Netzstatus), kein Backup und keine Übertragung auf ein neues Gerät, WebView-Debugging nur in Debug-Builds; dass andere Apps sie mit der Startseite öffnen können, ändert nichts. `version.json` hat Grenzen für Größe, Build und Commit, das APK eine für die Größe. Der Service Worker hält nur die Dateien des Builds; die API, Bilder und alles Fremde gehen immer zum Server, Antworten mit Daten eines Kontos liegen nie in seinem Cache, auch nicht nach dem Logout, und ein alter Build wird beim Wechsel gelöscht. Im `localStorage` liegen nur Einstellungen und zugeklappte Ordner. In den Workflows kommt kein Text aus Pull Requests oder Issues in ein Skript; der Cache von Docker und Rust ist pro Zweig, ein Pull Request kann den Build von `main` darüber nicht beeinflussen; `publish` und `app-release` laufen nur nach allen Prüfungen auf `main`. In `deploy/` lauscht rueder nur auf `127.0.0.1`, die Datenbank hat keinen Port nach außen, ihr Passwort ist Pflicht, `.env` bleibt aus dem Repository, der Container läuft ohne Root, schreibgeschützt und ohne Capabilities, und die Beispiele für Caddy und nginx setzen `X-Forwarded-For` selbst, von dem rueder den letzten Eintrag nimmt.

**Restrisiken, bewusst offen:**

- **Alte WebViews.** Auf Android System WebView vor Version 86 gibt Capacitor seine Brücke jeder Seite in der WebView, auch der des Servers. Das Plugin der App prüft dann, welche Seite gezeigt wird; die eingebauten Plugins von Capacitor (etwa `WebView.setServerBasePath`) prüfen nichts. Ausnutzen kann das nur ein Skript in der Seite des Servers, also nach einem Fehler, an dem die Content-Security-Policy vorbeikäme. Version 86 kam im Oktober 2020; eine ältere WebView ist seitdem nicht mehr aktualisiert worden.
- **Actions nach Tag, nicht nach Commit** (behoben am selben Tag, auf Fionas Wunsch): Actions stehen mit dem Hash ihres Commits, die Basis-Images des Image mit Digest, die Tools der CI mit fester Version und geprüfter Prüfsumme, und der Gradle-Wrapper wird geprüft (siehe [CI](implementation.md#ci)). Die Distribution von Gradle prüft der Wrapper gegen ihre Prüfsumme.
- **Der Signier-Schlüssel in den Secrets von GitHub.** Wer `main` ändern kann, kann ihn lesen. Ein Schlüssel, den nur ein geschützter Release-Weg sieht (eine Environment mit Freigabe), wäre enger.

### Header, Herkunft und Limits

Gebaut in Paket 2.9 (`server/src/hardening.rs`, `server/src/rate.rs`). Drei Schichten liegen um alles, was der Server beantwortet, auch um das Frontend und den Bild-Proxy. Von außen nach innen: Header, Rate-Limit, Herkunft. Eine abgelehnte Anfrage bekommt also dieselben Header wie jede andere, und sie erreicht weder einen Endpunkt noch die Live-Updates.

**Header an jeder Antwort.** Ein Endpunkt, der einen davon selbst setzt, behält seinen Wert; das tut nur der Bild-Proxy.

| Header | Wert | Wozu |
|---|---|---|
| `Content-Security-Policy` | Für das Frontend die Policy unten, für `/api` und `/img` `default-src 'none'; frame-ancestors 'none'` | Zweite Linie hinter der Bereinigung. |
| `X-Content-Type-Options` | `nosniff` | Der genannte Typ gilt, was auch immer in den Bytes steht. |
| `Referrer-Policy` | `no-referrer` | Verlinkte Seiten erfahren nicht, woher jemand kommt. |
| `X-Frame-Options` | `DENY` | Kein Rahmen auf fremden Seiten, für Browser ohne `frame-ancestors`. |
| `Cross-Origin-Opener-Policy`, `Cross-Origin-Resource-Policy` | `same-origin` | Eine geöffnete oder öffnende Seite bekommt keinen Zugriff auf das Fenster, und keine fremde Seite lädt, was rueder ausliefert. |
| `Permissions-Policy` | Kamera, Mikrofon, Standort, Bildschirmaufnahme, Zahlung und USB aus | Braucht ein Reader nicht, also bekommt es auch kein Artikel. |
| `Cache-Control` | `no-store` für `/api` | Antworten gehören einem Konto und bleiben in keinem Cache. Bilder behalten ihr `private, immutable`. |

**Die Content-Security-Policy des Frontends:**

```text
default-src 'none'; script-src 'self' 'sha256-…' 'sha256-…'; style-src 'self';
img-src 'self' data:; font-src 'self'; connect-src 'self'; media-src http: https:;
manifest-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'
```

- **Skripte** nur aus den eigenen Dateien. Die Seite von SvelteKit enthält zwei Skripte im HTML: das für das Thema vor dem ersten Bild und das, das die App startet. Der Server liest `index.html` beim Start, bildet ihre SHA-256 und nennt sie in der Policy. So kommt sie ohne `'unsafe-inline'` und ohne `'unsafe-eval'` aus: Ein Skript, ein `onerror` oder ein `javascript:`-Link in einem Artikeltext läuft nicht, auch wenn die Bereinigung es einmal durchließe. Nach einem neuen Build des Frontends braucht der Server deshalb einen Neustart; im Container ist das ohnehin so.
- **Styles** nur aus den eigenen Dateien, auch keine `style`-Attribute. Svelte setzt berechnete Styles über das DOM, das ist davon nicht betroffen. Die eine Ausnahme ist das `style`-Attribut, mit dem SvelteKit sein Element für Screenreader versteckt (`#svelte-announcer`): Der Server liest es beim Start aus den Skripten des Builds (`_app/immutable`, höchstens 500 Dateien zu je 4 MB) und erlaubt genau diesen Wert über seinen Hash (`style-src-attr 'unsafe-hashes' 'sha256-…'`). Bis 2026-11-12 lehnte der Browser es ab und meldete das bei jedem Laden in der Konsole, in Firefox als Fehler. Findet der Server das Attribut nicht, bleibt es abgelehnt; dieselben Regeln stehen deshalb weiter in `web/src/routes/layout.css`.
- **Bilder** nur vom eigenen Server, also vom Bild-Proxy. `data:` ist für das Icon in der Seite.
- **Verbindungen** (API, Live-Updates) nur zum eigenen Server.
- **Audio und Video** von überall: Sie kommen direkt von der Quelle und laden erst nach Klick, wie unter [Security](#security) beschrieben.
- Geprüft im Browser (Chromium) am gebauten Frontend: Die Oberfläche läuft, und ein eingeschleustes Skript, ein Event-Handler, `eval`, ein fremdes Bild und ein Abruf an einen fremden Server werden abgelehnt.
- Für Schritt 3: Der Login über OpenID Connect beginnt mit einem Link auf `/api/oidc/start`, nicht mit einem Formular, weil `form-action 'self'` die Weiterleitung zum Anbieter sonst sperrt.

`Strict-Transport-Security` setzt der Server nicht: TLS macht der Reverse-Proxy, und nur der weiß, ob die ganze Domain HTTPS spricht. Die Beispiele in [deploy/README.md](../deploy/README.md) setzen den Header dort.

**Herkunft ändernder Anfragen.** Jede Anfrage außer `GET`, `HEAD` und `OPTIONS` wird geprüft, bevor sie einen Endpunkt erreicht:

1. Sagt der Browser mit `Sec-Fetch-Site`, woher die Anfrage kommt, gilt das: `same-origin` und `none` (von Hand ausgelöst) gehen durch, alles andere wird mit 403 abgelehnt, auch `same-site`, also eine andere Subdomain. Diesen Header kann eine Seite nicht setzen oder ändern.
2. Ältere Browser nennen nur `Origin`. Dann muss dessen Host mit Port der sein, an den die Anfrage ging (`Host`). `null` wird abgelehnt.
3. Eine Anfrage ohne beides kommt von keinem Browser, etwa von `curl` mit dem Cookie in der Hand; die lässt sich von einer fremden Seite aus nicht senden und geht durch.

Der Server braucht dafür keine Einstellung mit seiner eigenen Adresse. Der Reverse-Proxy muss für Punkt 2 den `Host`-Header durchreichen; Caddy tut das von selbst, das nginx-Beispiel setzt ihn. Lesende Anfragen von fremden Seiten bleiben erlaubt, sonst ließe sich rueder nicht über einen Link öffnen; dort schützt `SameSite=Lax` des Cookies. Die eine lesende Anfrage, die etwas ändert, `GET /api/oidc/start?link`, prüft ihre Herkunft selbst, siehe [Login und Sitzungen](#login-und-sitzungen).

**Rate-Limits**, gezählt im Speicher des Servers für höchstens 10.000 Adressen und Konten. Wer darüber liegt, bekommt 429 mit `Retry-After`.

| Was | Grenze | Gezählt je |
|---|---|---|
| Jede Anfrage, auch an Frontend und Bild-Proxy | 1200 auf einmal, danach 50 je Sekunde | Adresse (bei IPv6 das /64) |
| Abrufe fremder Adressen: Feed-Suche und das Abonnieren eines Feeds, den noch niemand liest | 30 auf einmal, danach einer alle zwei Sekunden | Konto |
| Umzüge in ein Konto (`POST /api/import`) | 3 auf einmal, danach einer alle zwei Stunden | Konto |
| Fehlerberichte (`POST /api/reports`) | 100 auf einmal, danach etwa 100 am Tag, siehe [Fehlerberichte aus dem Browser](#fehlerberichte-aus-dem-browser) | Konto |
| Falsche Passwörter | 10 je Name und 30 je Adresse in 15 Minuten, siehe [Login und Sitzungen](#login-und-sitzungen) | Name und Adresse |
| Offene Live-Verbindungen | 20, siehe [Live-Updates](#live-updates) | Konto |

Die Grenze je Adresse liegt weit über dem, was Lesen braucht: Eine Liste lädt bis zu 200 Vorschaubilder auf einmal, und mehrere Leser können eine Adresse teilen. Sie soll verhindern, dass einer den Server für alle belegt. Hinter einem Reverse-Proxy ohne `CLIENT_IP_HEADER` zählen alle Leser als eine Adresse. Die Bilder des Proxy brauchen keine eigene Grenze je Konto, weil nur der Server ihre Adressen signiert: Niemand kann ihn darüber zu beliebigen Abrufen bringen. Ein OPML-Import ruft nichts ab, seine Feeds holt die nächste Runde des Abrufs.

**Teure Anfragen** (seit Paket 2.12, `rate::Rates::heavy`). Manche Anfragen lesen viel: die Vorschau und die Treffer der Filterregeln und jede Änderung an einer Regel (bis zu 200 Artikel in jedem der bis zu 2000 Feeds), die Versionen eines Artikels (bis zu 20 Fassungen zu je 1 MB, jede neu bereinigt), der Export und der Import eines Kontos und der OPML-Import. Ohne Grenze belegte ein einziges Konto mit wenigen gleichzeitigen Anfragen die zehn Verbindungen zur Datenbank und die Prozessoren, und mit offener Registrierung kann jeder ein Konto haben. Deshalb laufen auf dem ganzen Server höchstens drei davon zugleich (`rate::HEAVY_AT_ONCE`), weitere warten, ohne eine Verbindung zu halten; ein Konto darf zwei unterwegs haben (`rate::HEAVY_PER_ACCOUNT`), die dritte bekommt 429. Lesen, Lesestand und alles andere sind davon nicht betroffen. Nicht darunter fällt das Verschieben im Baum, das Regeln mit einem Ordner als Bereich neu bewertet, weil Drag & Drop sonst abgewiesen würde.

**Limits je Konto.** Was darüber hinausgeht, lehnt die API mit 400 ab.

| Was | Grenze | Wo |
|---|---|---|
| Abonnierte Feeds | 2000, davon höchstens 1000 in einem Ordner | `tree::MAX_FEEDS`, `tree::MAX_FEEDS_IN_FOLDER` |
| Ordner | 100, höchstens fünf Ebenen | `tree::MAX_FOLDERS` |
| Filterregeln | 200, ein Muster höchstens 200 Zeichen | `filters::MAX_RULES`, `filters::MAX_PATTERN` |
| Labels | 100 | `labels::MAX_LABELS` |
| Sitzungen | 100, die am längsten ungenutzte endet | `sessions::MAX_PER_ACCOUNT` |
| Einstellungen | 16 KB | `tree::MAX_SETTINGS` |
| Eine Anfrage an die API | 256 KB, eine OPML-Datei 1 MB mit höchstens 2000 Feeds, die Datei eines Kontos 32 MB mit höchstens 50.000 Artikeln | `MAX_BODY`, `subscribe::MAX_OPML`, `transfer::MAX_FILE` |

Die Grenze für Feeds ist mit diesem Paket dazugekommen: Jeder abonnierte Feed wird abgerufen, solange ihn jemand liest, und ohne sie hätte ein Konto den Server über wiederholte OPML-Importe beliebig viele Feeds abrufen lassen können. Ein Import, der sie überschreiten würde, wird als Ganzes abgelehnt.

**Abhängigkeiten.** Der CI-Job `dependencies` läuft bei jedem Pull Request und jeden Montag. `cargo audit` prüft `Cargo.lock` gegen die Datenbank von RustSec. `cargo deny check` prüft, was wirklich gebaut wird (`server/deny.toml`): bekannte Lücken und nicht mehr gepflegte Crates, Lizenzen nach einer festen Liste, Quellen (nur crates.io) und gesperrte Crates (`openssl`, `native-tls`, damit TLS bei `rustls` bleibt). Eine Ausnahme steht mit Begründung in `server/.cargo/audit.toml`: `libsqlite3-sys` (RUSTSEC-2026-0459) steht nur im Lockfile, weil `sqlx` seinen SQLite-Treiber als Option nennt; gebaut wird es nicht, und `cargo deny` sieht es deshalb auch nicht. Sie fällt weg, sobald ein Update von `sqlx` eine neuere Version zulässt.

### Gehärteter HTTP-Client

Gebaut in Paket 2.1 (`server/src/http_client.rs`, die Adressregeln in `server/src/http_client/address.rs`). Er ist der einzige Weg ins Internet; `server/clippy.toml` verbietet, an anderer Stelle einen `reqwest`-Client zu bauen.

- **Nur http und https.** Alles andere wird abgelehnt, auch als Ziel einer Weiterleitung.
- **Adressen.** Gesperrt ist, was nicht zum öffentlichen Internet gehört, nach den Listen der IANA: der eigene Rechner, private Netze, Link-Local (darunter `169.254.169.254`, der Metadaten-Dienst der Cloud-Anbieter), Carrier-NAT, Multicast, reservierte und Dokumentations-Netze, bei IPv6 alles außerhalb von `2000::/3` sowie Teredo und 6to4. Eine IPv4-Adresse in IPv6-Schreibweise (`::ffff:…`, NAT64 `64:ff9b::…`) zählt als diese IPv4-Adresse.
- **Zahlen als Host.** Steht im Host eine Adresse, wird sie vor der Anfrage geprüft, in jeder Schreibweise, die der URL-Parser liest (dezimal, oktal, hexadezimal, verkürzt, mit Zugangsdaten davor). An den HTTP-Teil geht die geparste Adresse, nie der eingegebene Text; zwei Lesarten derselben Eingabe gibt es so nicht.
- **Namen.** Der Client löst Namen selbst auf, prüft jede Adresse der Antwort und verbindet sich genau mit den geprüften Adressen. Ein Name kann also nicht mit einer Adresse die Prüfung bestehen und mit einer anderen verbunden werden (DNS-Rebinding). Hat ein Name auch nur eine gesperrte Adresse, wird er ganz abgelehnt.
- **Weiterleitungen** folgt der Client selbst, eine nach der anderen, und prüft jedes Ziel wie die erste Adresse. Höchstens 5.
- **Grenzen.** 30 Sekunden für den ganzen Abruf mit Namensauflösung, Weiterleitungen und Lesen (nicht je Schritt, damit ein tröpfelnder Server nichts gewinnt), 10 Sekunden für den Verbindungsaufbau, 20 MB für den Inhalt, gezählt nach dem Entpacken (gzip, Brotli, deflate, zstd). Ein Aufrufer kann die Größe enger setzen, nicht weiter; der Bild-Proxy wird das tun.
- **Nichts nebenbei.** Kein Proxy aus der Umgebung, denn ein Proxy würde die Namen an unserer Stelle auflösen. Keine Cookies, kein Referrer. Zugangsdaten in einer Adresse gehen nur an deren Host und erscheinen in keiner Fehlermeldung.
- **TLS** mit `rustls` und den Zertifizierungsstellen von Mozilla, einkompiliert (`webpki-roots`). So verhält sich der Client in jedem Container gleich.
- **Ausnahme für den Betreiber.** Wer einen eigenen Feed-Generator im internen Netz betreibt, kann einzelne Netze freigeben (`Config::allow`, etwa `192.168.1.0/24`). Ohne Angabe ist nichts freigegeben. Das weicht vom ersten Entwurf ab, der keine Ausnahme kannte; ohne sie ließe sich weder so ein Feed lesen noch der Client gegen einen lokalen Server testen. Die Einstellung kommt mit Paket 2.2 aus der Umgebung.

Die Tests belegen jede Sperre einzeln und lassen den Client gegen lokale Server laufen, von denen einer nie erreicht werden darf. Ein Property-Test vergleicht die Prüfung für zufällige Adressen in 16 Schreibweisen mit schlichten Tabellen.

### Fuzzing

Der Server verarbeitet unbeaufsichtigt Eingaben, die Fremde bestimmen. Tests von Hand decken nur die Fälle ab, an die jemand gedacht hat. Gefuzzt wird deshalb dort, wo eigener Code über Sicherheit entscheidet, und geprüft wird nicht nur „stürzt nicht ab“, sondern die Eigenschaft, auf die sich der Rest verlässt:

| Ziel | Eingabe | Was gelten muss |
|---|---|---|
| Bereinigung (`ammonia` plus eigene Regeln) | Beliebiges HTML | Keine Panic. Das Ergebnis enthält kein Skript, keinen Event-Handler, kein Formular und keine Bildadresse außer der des eigenen Proxy. Erneutes Bereinigen ändert es nicht mehr. |
| Feed-Verarbeitung, von den Bytes bis zum fertigen Artikel | Beliebige Bytes als Feed | Keine Panic, Ende in begrenzter Zeit und mit begrenztem Speicher. Die Grenze für die Artikelzahl gilt, jeder Artikel hat eine Kennung, dieselbe Eingabe ergibt dieselbe Prüfsumme. |
| Bild-Proxy: Formatprüfung und Dekodierung | Beliebige Bytes als Bild | Keine Panic. Angenommen werden nur die erlaubten Formate, die Grenzen für Dateigröße und Bildfläche greifen vor dem Dekodieren. |
| Adressprüfung des HTTP-Clients | Adressen in allen Schreibweisen (dezimal, oktal, hexadezimal, IPv4 in IPv6, mit Zugangsdaten, mit Port) | Nur http und https. Keine private, lokale oder Link-Local-Adresse wird erlaubt. Hier ist die Eingabe strukturiert, deshalb ein Property-Test mit `proptest` statt eines Fuzz-Targets. |

- Die Targets liegen in `server/fuzz/` und laufen mit `cargo-fuzz`, das Nightly-Rust braucht. Der Startkorpus sind die aufgezeichneten Feeds des alten Systems (`backend/test/data/`) und die Fälle der Cleaner-Tests, dazu ein paar Feeds in RSS, Atom und JSON Feed und kleine Bilder jedes Formats.
- Was gelten muss, steht einmal in `server/fuzz/checks.rs`. Die Targets binden die Datei ein, und `server/tests/fuzz_corpus.rs` auch: Der gespeicherte Korpus läuft so in der CI als gewöhnlicher Test mit stabilem Rust und prüft dasselbe wie der Fuzzer. Jeder Fund kommt als Datei in den Korpus, mit einem Namen, der sagt, was er fand (`found-…`), und bleibt so als Test erhalten.
- Die Bereinigung wird mit zwei Basisadressen geprüft, mit und ohne Seite des Artikels, und danach so, wie der Text ausgeliefert wird: Jedes Bild zeigt dann auf `/img/`. Beim Bild-Proxy sagt das erste Byte der Eingabe, für welche Größe; was herauskommt, muss das Format haben, das der Proxy nennt, und in diese Größe passen.
- Richtig gefuzzt wird von Hand vor den beiden Security-Reviews, etwa eine halbe Stunde je Target.
- Für die Bereinigung gibt es seit Paket 2.3 schon einen Property-Test mit denselben Eigenschaften (`server/tests/cleaning.rs`): Er setzt Texte aus Bruchstücken von Auszeichnung zusammen. Das Fuzz-Target kommt dazu, es ersetzt ihn nicht.
- Nicht gefuzzt werden die fremden Parser für sich und einmalige Eingaben (die alte Datenbank, der `localStorage` des alten Frontends). Für sie gelten Grenzen und Tests mit regelwidrigen Eingaben.

Der Ertrag ist kleiner als in einem Programm mit `unsafe`: Fuzzing findet hier Panics, Hänger und Speicherverbrauch, keine Speicherfehler, und eine Panic beendet nur die eine Anfrage oder den einen Abruf. Deshalb die Beschränkung auf wenige Ziele.

### Soak-Test

Tests und Fuzzing prüfen einzelne Anfragen. Was erst mit der Zeit ausgeht, sehen sie nicht: Speicher, der nicht zurückkommt, Dateien und Verbindungen, die offen bleiben, Aufgaben, die sich stauen, Antworten, die langsamer werden. Dafür läuft das ganze Programm eine Weile unter gleichmäßiger Last (`server/tests/soak.rs`, auf Fionas Wunsch vom 2026-11-11):

- Der Test startet `rueder serve` als eigenen Prozess, so wie er in Betrieb läuft, auf einer eigenen Datenbank. Neben ihm läuft ein kleiner Server des Tests mit 24 erfundenen Feeds und acht Bildern auf 127.0.0.1 (`FETCH_ALLOW_NETS`). Die Feeds wachsen mit der Zeit, jeder fünfte Artikel bekommt alle anderthalb Minuten einen neuen Text, und ein Feed fällt jede zweite Minute aus. Damit der Abruf in jeder Runde des Servers etwas zu tun hat, setzt der Test die Feeds alle fünf Sekunden auf fällig.
- Sechs erfundene Leser, jeder mit eigener Adresse in `X-Forwarded-For` wie hinter einem Reverse-Proxy, melden sich an, halten den Strom der Ereignisse offen und lesen: Baum, Liste mit dem Neuen eines Feeds, Artikel mit ihren Bildern über den Proxy, gelesen, Stern, Versionen, Ordner, alle Artikel und weiter unten, als gelesen bis hier und älter als eine Stunde, eine Filterregel samt Treffern, die wieder geht, ab und zu Ab- und wieder Anmelden. Zwischen zwei Anfragen eines Lesers liegen 20 bis 80 Millisekunden.
- Verlangt wird, dass jede Anfrage die Antwort bekommt, die sie bekommen soll, dass Ereignisse ankommen, dass die Ungelesen-Zahlen im Baum am Ende mit den Listen übereinstimmen, je Feed und über alle, dass im Log weder `ERROR` noch eine Panic steht und dass der Server auf `SIGTERM` sauber endet. Der Server hält nie mehr als die zehn Verbindungen seines Pools.
- Alle paar Sekunden notiert der Test aus `/proc`, was der Prozess hält (Speicher, Threads, offene Dateien), dazu die Verbindungen zur Datenbank und Median und 99. Perzentil der Antwortzeiten. Läuft er fünf Minuten oder länger, vergleicht er das Ende mit der Zeit nach dem Aufwärmen: Speicher darf um ein Viertel wachsen, mindestens um 32 MB, offene Dateien und Threads um 16, das 99. Perzentil auf das Dreifache, mindestens auf 250 ms.
- In der CI läuft er mit `cargo test` 20 Sekunden lang, länger von Hand mit `SOAK_SECONDS`. Ergebnisse stehen unter [Messung](implementation.md#messung).

## API

JSON unter `/api`, Sitzung über das Cookie. Gebaut in den Paketen 2.5 und 2.6; die TypeScript-Typen des Frontends werden aus den Rust-Typen erzeugt (Paket 2.8), damit beide Seiten nicht auseinanderlaufen, siehe [Typen für das Frontend](#typen-für-das-frontend).

| Bereich | Endpunkte |
|---|---|
| Zustand | `GET /api/health`; für Admins `GET /api/status`, siehe [Status für Admins](#status-für-admins) |
| Sitzung | `POST /api/login`, `GET /api/login/ways`, `POST /api/logout`, `GET /api/me`, `GET /api/oidc/start`, `GET /api/oidc/callback` |
| Registrierung | `GET` und `POST /api/register` (ohne Sitzung), `GET` und `POST /api/invites`, `DELETE /api/invites/{id}` |
| Konto | `GET /api/sessions`, `DELETE /api/sessions`, `DELETE /api/sessions/{id}`, `PUT /api/password`; für Admins `GET` und `POST /api/accounts` |
| Baum | `GET /api/tree` (Ordner, Abos, Feeds mit `last_seq`, Lesemarken, Ungelesen-Zahlen und dem Fehler des letzten Abrufs, dazu die Zahl der ausgeblendeten Artikel), `POST /api/tree/move`, `POST /api/folders`, `PATCH` und `DELETE /api/folders/{id}` |
| Abos | `POST /api/feeds/find`, `POST /api/feeds`, `PATCH` und `DELETE /api/feeds/{id}`, `PUT /api/feeds/{id}/agent`, `GET` und `POST /api/opml` |
| Listen | `GET /api/feeds/{id}/articles`, `GET /api/folders/{id}/articles`, `GET /api/articles` (alle), `GET /api/starred`, `GET /api/hidden`, `GET /api/labels/{id}/articles`, jeweils mit `before` oder `after`, `limit` und `unread` |
| Suche | `GET /api/search` mit `q`, dazu `feed` oder `folder`, `before`, `limit` und `unread`, siehe [Suche](#suche) |
| Artikel | `GET /api/feeds/{id}/articles/{seq}` (bereinigter Text, Anhänge, Aufmacherbild, die ausblendende Regel), `GET /api/feeds/{id}/articles/{seq}/versions` |
| Lesestand | `PUT /api/feeds/{id}/articles/{seq}/state` mit `read` und `starred`; „alle als gelesen“ mit `POST /api/feeds/{id}/read`, `POST /api/folders/{id}/read` und `POST /api/read` |
| Regeln | `GET` und `POST /api/rules`, `PUT` und `DELETE /api/rules/{id}`, `POST /api/rules/order`, `POST /api/rules/preview`, `GET /api/rules/hits` |
| Labels | `GET` und `POST /api/labels`, `PATCH` und `DELETE /api/labels/{id}`, `POST /api/labels/order`, `PUT` und `DELETE /api/feeds/{id}/articles/{seq}/labels/{label}` |
| Einstellungen | `GET` und `PUT /api/settings` |
| Fehlerberichte | `POST /api/reports`, `GET` und `PUT /api/reports/consent`, siehe [Fehlerberichte aus dem Browser](#fehlerberichte-aus-dem-browser) |
| Umzug | `GET /api/export`, `POST /api/import`, siehe [Umzug eines Kontos](#umzug-eines-kontos) |
| Live | `GET /api/events` (SSE), siehe [Live-Updates](#live-updates) |
| Bilder | `GET /img/<signatur>/<größe>/<adresse>` |

Was für alle gilt:

- **Ohne Sitzung 401.** Jeder Handler nimmt den Extraktor `Session` und bindet seine Abfragen an dessen Kontonummer. Was einem anderen Konto gehört oder was es nicht gibt, ist beides 404; die Antwort verrät nicht, welches von beiden. Zu jedem Endpunkt gibt es einen Test, in dem ein zweites Konto nichts davon sieht und nichts daran ändert.
- **Listen blättern mit einem Anker** statt mit einem Offset: `before` ist die ID des letzten gezeigten Artikels, `limit` höchstens 200 (Standard 50). Mit `after` statt `before` läuft die Liste vom ältesten Artikel an, `after=0` beginnt ganz vorn; beides zugleich ist 400. Die aufsteigende Liste ist dieselbe Zusammenführung der Feeds, nur in die andere Richtung, und kostet dasselbe. Jeder Eintrag trägt Lesestand, Stern und Labels.
- **Bilder zeigen immer auf den Bild-Proxy.** Der Text eines Artikels geht durch `Signer::in_html`, Vorschaubild, Aufmacherbild und Icon durch `Signer::address`. Ein Server ohne Bild-Proxy liefert Texte ohne Bilder aus.
- **Fehler** sind 400 mit einem Satz als Text (eine Eingabe, die gegen eine Regel verstößt), 422 (eine Anfrage, die sich nicht lesen lässt), 403 (nur für Admins, falsches bisheriges Passwort, eine ändernde Anfrage von einer fremden Seite), 413 (eine Anfrage über 256 KB), 429 mit `Retry-After` (zu viele Fehlversuche, zu viele Anfragen) und 500 ohne Einzelheiten. Was für jede Anfrage und jede Antwort gilt, steht unter [Header, Herkunft und Limits](#header-herkunft-und-limits).
- **Alle als gelesen** nennt, bis wohin: `upto` (Nummer im Feed) oder `upto_id` (ID des neuesten Artikels, den die Liste zeigte). Was danach eintrifft, bleibt ungelesen. Mit `older_than` (Sekunden, höchstens hundert Jahre) gilt es nur für das, was länger her veröffentlicht wurde; der Server rechnet die Grenze mit seiner eigenen Uhr. Die Lesemarke rückt so weit vor, wie alles darunter alt genug ist; ein alter Artikel, der zwischen jüngeren eintraf, wird eine Ausnahme über der Marke.
- **Welche Regel ausblendet,** nennt ein geöffneter Artikel in `hidden_by`: die erste eingeschaltete Regel in der Reihenfolge der Liste, die für den Feed gilt, ausblendet und trifft. Gespeichert wird das nicht, es wird beim Öffnen nachgerechnet; nur wenn der Artikel ausgeblendet ist.
- **Treffer je Regel** zählt `GET /api/rules/hits` für alle Regeln des Kontos, auch die ausgeschalteten, über dieselben Artikel wie die Vorschau (die neuesten 200 je Feed). Jeder Artikel wird dabei einmal gelesen, wie viele Regeln es auch sind.
- **Änderungen am Lesestand** wirken im Frontend sofort und werden im Hintergrund bestätigt; lehnt der Server ab, nimmt das Frontend sie zurück.

Abweichungen vom Entwurf:

- Ein Artikel wird über Feed und Nummer angesprochen (`/api/feeds/{id}/articles/{seq}`), nicht über seine ID. So liest jede Abfrage mit dem Schlüssel der Tabellen und prüft das Abo in einem Zug.
- Alle Listen blättern mit der ID als Anker, auch die eines einzelnen Feeds. Die ID wächst mit dem Eintreffen und gibt gemischten Listen eine feste Reihenfolge.
- Alte Versionen eines Artikels sind gespeichert, wie sie kamen, und werden beim Ausliefern bereinigt, mit denselben Regeln wie der aktuelle Text. Das kostet beim seltenen Blick in die Versionen etwas Zeit und spart einen zweiten, bereinigten Bestand.
- Ausblenden von Hand gibt es nicht, siehe [Filterregeln](#filterregeln).
- Die Volltextsuche auf dem Server kam erst mit Paket 2.13, nach der Messung ihres Index, siehe [Suche](#suche).

### Typen für das Frontend

Was die API als JSON sendet und liest, gibt es als TypeScript-Typen in `web/src/lib/api/types.ts`. Die Datei schreibt der Server aus seinen eigenen Typen; von Hand geändert wird sie nicht.

- **Ableiten.** Jeder Typ, der als JSON hinausgeht oder hereinkommt, leitet neben `Serialize` oder `Deserialize` auch `TS` ab (Crate `ts-rs`), das die Angaben von `serde` mitliest: umbenannte Felder, Enums als Text, eingebettete Typen. Sein Modul nennt ihn in der Funktion `types`; die Typen, aus denen er besteht, kommen von selbst mit.
- **Schreiben.** `server/src/typescript.rs` fragt alle Module und schreibt eine Datei. Die Kommentare der Rust-Typen stehen darin als Kommentare der TypeScript-Typen und nennen zu jedem Typ seinen Endpunkt. In TypeScript teilen sich alle Typen einen Namensraum; ein Typ, dessen Name nur in seinem Modul etwas sagt, bekommt dort einen eigenen (`labels::Change` heißt `LabelChange`). Zwei Module dürfen denselben Namen nur für denselben Typ verwenden, sonst scheitert das Schreiben. Die Reihenfolge in der Datei ist fest: die Typen, wie die Module sie nennen, hinter jedem seine Teile nach Namen. In welcher Reihenfolge `ts-rs` die Teile findet, wechselt von einem Compiler zum nächsten und darf die Datei nicht ändern.
- **Prüfen.** Die Datei ist eingecheckt. Der Test `server/tests/api_types.rs` läuft mit `cargo test` und in der CI: Er scheitert, wenn die Datei nicht dem entspricht, was die Typen heute sagen, und schreibt sie mit `UPDATE_API_TYPES=1` neu. Ein zweiter Test liest den Quelltext und scheitert, wenn ein Typ JSON ist und in der Datei fehlt; ausgenommen ist nur, was nicht zur API gehört (gespeicherte Versionen, die alte Datenbank, die Antworten des OpenID-Anbieters). Auf der anderen Seite hält `web/src/lib/api/types.test.ts` Antworten des Servers gegen die Typen, was `pnpm check` prüft.
- **Was die Typen sagen.** Zeiten sind Text nach RFC 3339. Zahlen mit 64 Bit (IDs von Artikeln, Zähler) sind gewöhnliche Zahlen; keine kommt in die Nähe von 2^53, ab wo JavaScript sie nicht mehr unterscheidet. Ein Feld mit `?` darf in einer Anfrage fehlen. Fehler kommen als Text, nicht als JSON, und haben keinen Typ.
- **Ereignisse.** Die Live-Updates haben einen Typ für alles, was über die Verbindung geht (`ServerEvent`): der Name des Ereignisses und was es trägt. Der Server schreibt die Ereignisse aus genau diesem Typ, er kann also nichts senden, was das Frontend nicht kennt.
- **Anhänge.** Die Dateien eines Artikels (`enclosures`) sind gespeichert, wie sie kamen: aus dem Feed mit der Größe als Zahl, aus dem alten rueder mit der Größe als Text. Hinaus gehen sie in einer Form (`url`, `type`, `length`, `inline`), höchstens 100 je Artikel, und nur mit Adressen, die mit `http` oder `https` beginnen.

Ohne Typ bleiben die Einstellungen eines Kontos: Ihr Inhalt gehört dem Frontend, der Server speichert ihn als JSON und begrenzt nur die Größe. Welcher Endpunkt welchen Typ nimmt und gibt, steht im Kommentar am Typ; eine Tabelle, die das prüfbar festhält, gibt es nicht.

## Frontend

Das Frontend ist eine SvelteKit-Anwendung ohne Server-Rendering (`adapter-static`), die der Rust-Server als statische Dateien ausliefert. Der Aufbau folgt dem Google Reader ab 2011: Suchleiste oben, links der Abonnieren-Knopf und der Ordnerbaum mit Ungelesen-Zahlen, rechts die Werkzeugleiste über der Artikelliste.

Der abgenommene Entwurf in `web/` enthält bereits:

| Teil | Wo |
|---|---|
| Datentypen der Oberfläche (Feed, Artikel, Version, Baum) | `web/src/lib/model.ts` |
| Lesestand und Ungelesen-Zahlen | `web/src/lib/unread.ts` |
| Filterregeln | `web/src/lib/filters.ts`, `web/src/lib/rule-text.ts` |
| Ordnerbaum und Züge | `web/src/lib/tree.ts` |
| Versionsvergleich | `web/src/lib/diff.ts` |
| Tastaturbelegung | `web/src/lib/keys.ts` |
| Einstellungen (Thema, Akzentfarbe, Dichte, Schrift, Ansicht) | `web/src/lib/settings.svelte.ts` |
| Texte auf Englisch und Deutsch | `web/src/lib/i18n/` |
| Komponenten (Baum, Liste, Artikel, Werkzeugleiste, Dialoge) | `web/src/lib/components/` |
| Die Naht zur Datenquelle | `web/src/lib/reader.ts` |
| Anbindung an den Server: API, Reader, Sitzung | `web/src/lib/api/` |
| Stellvertreter für den Server mit erfundenen Daten | `web/src/lib/demo/` |

Die Naht zum Server ist das Interface `Reader` in `web/src/lib/reader.ts`: Die Komponenten holen ihre Daten nur von einem `Reader`. Es gibt zwei. `ServerReader` (`web/src/lib/api/reader.svelte.ts`) füllt es aus der API, `DemoReader` (`web/src/lib/demo/reader.svelte.ts`) aus erfundenen Daten im Browser, für die Tests, für `pnpm dev:demo` und für die Ein-Datei-Fassung.

Gebaut in Paket 3.1 (`web/src/lib/api/`):

- **Die Seite.** `src/routes/+page.svelte` fragt mit `GET /api/me`, wer da ist, und zeigt den Reader oder die Login-Seite. Die Seite selbst ist die Komponente `ReaderPage`, die ihren `Reader` gereicht bekommt; die Fassungen ohne Server (`src/single`) geben ihr den `DemoReader`.
- **Login.** Name und Passwort gehen an `POST /api/login`; die Sitzung ist das Cookie, von dem das Frontend nichts sieht. `GET /api/login/ways` sagt, ob es den Weg über OpenID Connect gibt, dann zeigt die Seite den Knopf dafür. Antwortet irgendein Endpunkt mit 401, ist die Sitzung zu Ende, und die Login-Seite erscheint wieder.
- **Die API im Frontend** ist `client.ts`: eine Funktion je Endpunkt, mit den Formen der Antworten. Die Formen kommen aus `types.ts`, das der Server aus seinen Rust-Typen schreibt (siehe [Typen für das Frontend](#typen-für-das-frontend)); `client.ts` gibt ihnen nur die Namen, unter denen die Seite sie kennt. Feeds und Ordner heißen in der Seite `feed-12` und `folder-3`, weil der Baum beide in einer Liste führt und ihre Nummern sich überschneiden.
- **Listen** kommen seitenweise (40 Artikel, Anker `before`, bei „Älteste zuerst“ `after`) und ohne Texte. Eine Liste, die beim ältesten beginnt, fragt zugleich nach dem neuesten Artikel, damit „alle als gelesen“ weiß, wo es aufhört. Der Text eines Artikels wird geholt, wenn er gezeigt wird, höchstens vier zugleich, und mit einem geöffneten Artikel der, den der nächste Schritt öffnen würde (Paket 3.6). Wie die Suche im Frontend arbeitet, steht unter [Suche](#suche).
- **Lange Listen** (Paket 3.6) zeichnen nur, was in der Nähe des Sichtbaren liegt: ab 61 Zeilen in der Listenansicht, ab 13 Karten in der erweiterten, die mit ihren Texten schwer sind. Was darüber und darunter liegt, ist leerer Raum in der Höhe, die die Zeilen beim Zeichnen hatten; für Zeilen, die nie gezeichnet wurden, gilt der Durchschnitt der gezeichneten. Gezeichnet wird bis 1200 Pixel über und unter dem Sichtbaren, damit schnelles Scrollen keine Lücke zeigt. Die Rechnung steht in `web/src/lib/virtual.ts`, ein Zufallstest hält sie gegen das Modell, jede Zeile einzeln zu fragen. Ein Schritt mit der Tastatur zu einem Artikel, der nicht gezeichnet ist, scrollt erst an seine Stelle. Dass in der erweiterten Ansicht gelesen ist, was oben hinausscrollt, ergibt sich aus denselben Höhen. Die erweiterte Ansicht holt so auch nur die Texte der Karten in der Nähe, statt einer ganzen Seite auf einmal.
- **Als App** (Paket 3.10): Das Frontend ist eine PWA. `static/manifest.webmanifest` gibt Name, Farben und Icons (`static/`, als SVG und als PNG gerendert, auch maskierbar), `app.html` verweist darauf. Der Service Worker (`src/service-worker.ts`, von SvelteKit als `/service-worker.js` gebaut) hält die Dateien seiner Version in einem Cache, der nach ihr benannt ist: die Seite selbst und alles unter `_app/immutable` und `static`. Jede Adresse der App bekommt die gehaltene Seite, jede Datei ihre gehaltene Fassung, beides ohne Netz; `/api`, `/img`, der Worker selbst und alles von anderen Adressen gehen immer an den Server und werden nie gehalten (`src/lib/offline.ts`). So liegt nichts, was einem Konto gehört, im Cache, und ohne Server startet die Seite und sagt, dass er nicht antwortet. Eine neue Version des Servers bringt einen neuen Worker; er wartet, bis der Leser „Neu laden“ wählt (`src/lib/updates.svelte.ts`, `UpdateNotice.svelte`), damit die Seite nicht unter ihm wechselt. Gesucht wird beim Start, stündlich und wenn die Seite nach mindestens fünf Minuten wieder sichtbar wird. Der erste Worker übernimmt ohne Frage, beim Aktivieren gehen die Caches älterer Versionen. Registriert wird nur in der Seite mit Server, nicht im Dev-Server und nicht in den Fassungen mit erfundenen Daten, die keinen Worker haben. Die Content-Security-Policy erlaubt den Worker über `script-src 'self'` und das Manifest über `manifest-src 'self'`; der Server ändert sich dafür nicht. Die Leisten des Browsers und der installierten App folgen dem Theme der Einstellungen, nicht nur dem des Systems: Das Skript in `app.html` setzt `theme-color` vor dem ersten Bild auf die Farbe der oberen Leiste (`panel`), und im dunklen Theme verweist die Seite auf `static/manifest-dark.webmanifest`, das bis auf die Farben gleich ist (`barColors` und `manifests` in `src/lib/settings.svelte.ts`). Firefox auf Android färbt die Leisten einer installierten App nur nach dem Manifest, und ein Manifest kann dem Theme nicht selbst folgen; es nimmt das Manifest, auf das die Seite beim Installieren verweist.
- **Änderungen wirken sofort.** Lesen, Stern, alle als gelesen und jeder Zug im Baum zeigen sich gleich und gehen im Hintergrund an den Server, eine nach der anderen in der Reihenfolge, in der sie gemacht wurden. Lehnt der Server eine ab, wird sie zurückgenommen, eine Zeile über der Liste sagt es, und Baum und Zahlen werden neu geholt. Eine abgelehnte Änderung, die eine spätere schon überholt hat, wird nicht zurückgenommen.
- **Artikel** zeigen mit ihrem Text die Anhänge und, wo der Text kein eigenes Bild hat, das Aufmacherbild über den Bild-Proxy. Audio und Video bekommen erst einen Spieler, wenn man auf „Abspielen“ klickt; vorher lädt die Seite nichts von der Adresse des Anhangs. Die Content-Security-Policy erlaubt Medien von `http:` und `https:`.
- **Filterregeln** kommen aus `/api/rules`. Die Treffer je Regel holt der Editor, wenn er geöffnet wird, und nach jeder Änderung neu. Eine Änderung zeigt sich in der Liste der Regeln sofort; was sie ausblendet und als gelesen markiert, rechnet der Server, deshalb werden Baum und Liste danach neu geholt. Während eine Regel geschrieben wird, fragt der Editor `POST /api/rules/preview`, sobald das Tippen eine Viertelsekunde ruht: So erfährt er, was die Regel träfe und ob ihr Muster eines ist, denn die regulären Ausdrücke des Servers sind nicht die des Browsers.
- **Live-Updates** (Paket 3.2): Der `ServerReader` öffnet beim Start erst `GET /api/events` und lädt dann, wie oben unter [Live-Updates](#live-updates) verlangt. Jede Anfrage trägt den Namen des Tabs (`X-Rueder-Tab`, zufällig je Tab); ein Ereignis, das ihn in `by` nennt, ist die eigene Änderung und ändert nichts mehr. Ein Ereignis sagt nur, was sich geändert hat, und der Reader fragt danach: den Baum für die Zahlen, die Regeln, die gezeigte Liste in ihrer ganzen Länge. Mehrere Ereignisse kurz hintereinander (eine Runde des Abrufs bringt viele) lösen eine Anfrage aus, und diese wartet auf die eigenen Änderungen, die noch unterwegs sind, damit sie sie nicht zurückdreht. Neue Artikel in der gezeigten Liste schieben sie nicht weg, sondern zählen in den Hinweis „N neue Artikel“. Bricht die Verbindung ab, öffnet der Browser sie selbst und bekommt nachgeliefert; lehnt der Server sie ab, prüft der Reader die Sitzung und versucht es nach 30 Sekunden neu, und weil eine neue Verbindung nichts nachliefert, lädt er dann alles neu.
- **Einstellungen** (Paket 3.3) liegen beim Konto (`GET` und `PUT /api/settings`) und als Kopie im Browser, damit die Seite vor der Antwort des Servers richtig aussieht. Was in ihnen steht, ist Sache der Seite (`web/src/lib/settings.svelte.ts`); was sie nicht versteht, wird zum Standard. Ein Konto ohne Einstellungen bekommt beim ersten Start die des Browsers. Dazu gehören die eigenen Tasten der Kürzel: Jedes Kürzel ist eine Aktion (`web/src/lib/keys.ts`), und die Seite schlägt nach, welche Aktion eine Taste hat. Enter, Escape und `g` (der Anfang von „g dann x“) bleiben fest.
- **Labels** (Paket 3.4) stehen in der Seitenleiste unter den festen Listen, jedes mit seiner Liste (`GET /api/labels/{id}/articles`). Welche Labels ein Artikel hat, sagen die Listen; eine Änderung wirkt sofort und geht mit `PUT` oder `DELETE /api/feeds/{id}/articles/{seq}/labels/{label}` an den Server. Ein neues Label erscheint, wenn der Server es hat. Farben sind eine von acht festen (`labelColors` in `web/src/lib/model.ts`); der Server nimmt jedes `#rrggbb`.
- **Eine Antwort, die zu spät kommt,** wird verworfen: Wer schnell von Feed zu Feed klickt, sieht nie die Liste des vorigen.
- **Abonnieren** (Paket 3.5) geht in zwei Schritten: Der Dialog fragt `POST /api/feeds/find`, was unter einer Adresse zu finden ist, und zeigt den Feed selbst oder die Feeds, die eine Website nennt; der gewählte wird mit `POST /api/feeds` abonniert, auf Wunsch in einen Ordner. Der Server ruft einen Feed, den noch niemand liest, dabei zuerst ab, das kann Sekunden dauern; erst dann erscheint er im Baum, und die Seite zeigt ihn. Suchen und Abonnieren stehen in derselben Reihe wie alle Änderungen, ein gerade angelegter Ordner ist also da, wenn ein Feed hinein soll. Was der Server ablehnt, sagt der Dialog mit eigenen Worten und darunter mit denen des Servers, der etwa nennt, was die Website geantwortet hat; zu viele Abfragen (429) und ein Server ohne Abruf (503) haben eigene Sätze. Die Suche nennt bei jedem gefundenen Feed, ob das Konto ihn schon abonniert hat (`feed_id`, nur für die eigenen Abos); dann bietet der Dialog „Zeigen“ statt „Abonnieren“. Hat die Website die Anfrage weitergeleitet (`moved_to`) und es gibt dort keinen Feed, sagt der Dialog, wohin: Manche Websites schicken alle, die nicht eingeloggt sind, auf ihre Login-Seite, Reddit etwa.
- **Ein neuer Ordner aus zwei Feeds:** Wird ein Feed auf die mittlere Zone eines anderen gezogen, entsteht an dessen Stelle ein Ordner „Neuer Ordner“ mit beiden darin. Der `ServerReader` legt ihn mit `POST /api/folders` im Ordner des Ziels an und zieht ihn und die beiden Feeds mit `POST /api/tree/move` an ihren Platz; der Ordner erscheint, wenn der Server ihn hat, wie jeder neue Ordner.
- **Umbenennen und Abbestellen** eines Feeds stehen im Menü seiner Zeile im Baum und wirken sofort. Ein leerer Name gibt dem Feed seinen eigenen zurück, den nur der Server kennt; deshalb wird der Baum danach neu geholt. Abbestellen fragt nach, weil mit dem Abo der Lesestand des Feeds geht; eine Regel, die für den Feed gilt, bleibt wie auf dem Server und greift wieder, wenn er erneut abonniert wird.
- **OPML** steht im selben Dialog. Eine Datei geht, wie sie ist, als `text/x-opml` an `POST /api/opml`; eine über 1 MB lehnt schon der Browser ab. Der Dialog nennt, wie viele Feeds und Ordner neu sind, welche schon abonniert waren und was ausgelassen wurde. Der Export holt `GET /api/opml` und bietet die Datei als `rueder-<Datum>.opml` zum Speichern an. Der `DemoReader` hat kein OPML (`Reader.can.opml`): Er bräuchte einen eigenen Parser im Browser, der nur dem Entwurf diente.
- **Umzug** (Paket 3.12) ist ein Reiter der Einstellungen. „Konto exportieren“ holt `GET /api/export` und bietet die Datei als `rueder-<Datum>.json` zum Speichern an; „Kontodatei importieren“ schickt eine gewählte Datei, wie sie ist, als `application/json` an `POST /api/import`, eine über 32 MB lehnt schon der Browser ab. Danach lädt der `ServerReader` Baum, Einstellungen, Labels, Regeln und die gezeigte Liste neu, denn die Ereignisse des Servers überhört er als eigene. Der Reiter nennt, was kam, was verloren ging und was ausgelassen wurde, und bei 429 die Minuten bis zum nächsten Versuch. Der `DemoReader` kann es nicht (`Reader.can.move`), der Reiter fehlt dort.

Abweichungen vom Entwurf:

- **Die Ungelesen-Zahlen rechnet der `ServerReader` nicht selbst.** Der Entwurf sah vor, dass das Modell aus `unread.ts` (Lesemarke und Ausnahmen) im Frontend bleibt. Die API liefert aber je Feed die fertige Zahl und je Artikel „gelesen“ und „Stern“, nicht die Ausnahmen. Der `ServerReader` übernimmt die Zahl des Servers und bewegt sie bei einer eigenen Änderung um eins; nach „alle als gelesen“ und nach jeder Ablehnung fragt er den Server neu. So gibt es weiterhin keine zweite Stelle, die zählt. Das Modell `unread.ts` bleibt das des `DemoReader` und der Maßstab für den Server.
- **Ein neuer Ordner erscheint erst, wenn der Server ihn hat,** weil der Server seine Nummer vergibt. Alle anderen Änderungen am Baum wirken sofort.
- **Welche Ordner zugeklappt sind,** merkt sich der Browser (`localStorage`), nicht der Server.
- Was der Oberfläche mit Server zuerst fehlte und mit welchem Paket es kam, steht bei [Paket 3.1](implementation.md#schritt-3-frontend).

### Prüfung im Browser

Gebaut in Paket 3.7 (`web/e2e/`, `web/playwright.config.ts`). Die Tests in `web/src/` prüfen die Oberfläche gegen einen nachgebauten Server; ob Oberfläche und echter Server zusammenpassen, zeigen erst diese.

- **Was läuft.** Playwright steuert Chromium gegen den gebauten Server mit dem gebauten Frontend, so wie beides in Betrieb geht. Der Server bekommt je Lauf eine eigene, neu angelegte Datenbank und für jede Spec ein eigenes Konto, damit das Lesen der einen nicht in der anderen auftaucht.
- **Fixtures.** Eine erfundene Website (`harness/site.js`) liefert drei Feeds, eine Seite je Feed, die ihren Feed nennt, und ein Bild. Die Artikel enthalten, was die Bereinigung entfernen muss: ein Skript, ein Zählpixel, einen Tracking-Parameter. Der Server ruft sie über den gehärteten HTTP-Client ab, mit `FETCH_ALLOW_NETS=127.0.0.0/8`; das ist die Ausnahme für den Betreiber, sonst ist nichts abgeschaltet oder ersetzt. Über `/_control` lassen die Tests einen Artikel eintreffen oder sich ändern. Jede Spec liest ihre eigene Kopie eines Feeds unter eigener Adresse (`/harbour-tree/` ist `harbour`), damit nicht davon abhängt, welche Spec ihn vorher abonniert hat; ob ein zweiter Leser nur die 10 neuesten Artikel ungelesen findet, prüft `accounts.spec.ts` eigens.
- **Wege.** Login mit falschem und richtigem Passwort, Attribute des Sitzungs-Cookies, Abmelden. Abonnieren über die Adresse einer Webseite, Lesen, Stern, wieder ungelesen, nur ungelesene, alle als gelesen, jeweils auch nach einem Neuladen. Ordner anlegen, mit Tastatur und Drag & Drop einsortieren, umbenennen, auflösen. Eine Filterregel mit Vorschau, der Liste der ausgeblendeten und dem Löschen. Live-Updates: ein neuer Artikel, eine neue Version mit Vergleich, Lesestand im zweiten Tab. Zwei Konten, von denen das eine nichts vom anderen sieht.
- **Was sie an Security mitprüfen.** Ohne Sitzung antworten API und Bild-Proxy mit 401. Der Cookie ist `HttpOnly`, `Secure` und `SameSite=Lax` und für Skripte unsichtbar. Der Browser fragt beim Lesen nichts außer rueder selbst. Das Bild kommt vom Proxy, das Skript des Artikels läuft nicht. Ein fremdes Konto bekommt für Feed, Artikel und Ordner 404.
- **Abruf auf Wunsch gibt es nicht.** Ein Test, der auf einen Abruf wartet, setzt in der Datenbank alle Feeds auf fällig und wartet die nächste Runde des Servers ab, höchstens 30 Sekunden.
- **Umzug** (`move.spec.ts`, Paket 3.12): Ein Konto liest und markiert einen Artikel, exportiert sich, ein zweites importiert die Datei und findet Feed, Lesestand und Stern wieder, auch nach einem Neuladen.
- **Nicht abgedeckt:** OpenID Connect (braucht einen Anbieter), OPML, Labels, Einstellungen außer dem Umzug, die Statusseite und die Ansicht fürs Handy. Geprüft wird nur in Chromium.

## Betrieb

- Ein Container mit dem Rust-Programm und dem gebauten Frontend, daneben PostgreSQL, davor ein Reverse-Proxy für TLS.
- Konfiguration über Umgebungsvariablen (`DATABASE_URL`, Adresse zum Lauschen, Verzeichnis des Frontends, Verzeichnis, Größe und Schlüssel des Bild-Proxy, `CLIENT_IP_HEADER` für die Rate-Limits hinter dem Reverse-Proxy, OpenID Connect). Geheimnisse stehen nie im Image und nie im Frontend-Build.
- Migrationen laufen beim Start. Der Container läuft ohne Root mit schreibgeschütztem Dateisystem; beschreibbar ist nur das Verzeichnis für den Bild-Cache, in dem auch der selbst erzeugte Schlüssel des Bild-Proxy liegt.
- Sicherung: PostgreSQL. Der Bild-Cache lässt sich jederzeit neu aufbauen.

Gebaut in Paket 2.10 (`Dockerfile`, `deploy/`). Die Anleitung zur Installation ist [deploy/README.md](../deploy/README.md).

- **Das Image** entsteht in drei Stufen: Node baut das Frontend, Rust den Server (mit `SQLX_OFFLINE=true`, also ohne Datenbank), und die letzte Stufe enthält nur das Programm, das Frontend und die C-Bibliothek (`gcr.io/distroless/cc-debian13`). Darin gibt es keine Shell und keinen Paketmanager. Es läuft als User `nonroot` (65532).
- **Schreiben** kann der Server nur nach `/var/cache/rueder`, ein Volume für den Bild-Cache und den selbst erzeugten Schlüssel des Bild-Proxy. Die Compose-Datei startet ihn mit schreibgeschütztem Dateisystem, ohne Capabilities und mit `no-new-privileges`.
- **Healthcheck.** `rueder health` fragt den laufenden Server nach `/api/health` und endet mit einem Fehler, wenn er nicht mit 200 antwortet, also keine Anfragen bedient oder seine Datenbank nicht erreicht. Der Unterbefehl öffnet selbst keine Datenbank. Er spricht nur mit dem eigenen Rechner und nutzt deshalb nicht den gehärteten HTTP-Client, der lokale Adressen ablehnt.
- **Die Compose-Datei** (`deploy/compose.yaml`) startet rueder und PostgreSQL 16. Die Datenbank ist nur für den rueder-Container erreichbar, rueder selbst nur auf `127.0.0.1:3000` für den Reverse-Proxy des Betreibers. Alle Einstellungen stehen in `deploy/.env`; die Vorlage `.env.example` nennt sie. Zwei Volumes: die Datenbank und der Bild-Cache.
- **Der Reverse-Proxy** gehört nicht zur Compose-Datei, weil meist schon einer läuft. Die Anleitung zeigt Caddy und nginx, samt `CLIENT_IP_HEADER` und ohne Pufferung für die Live-Updates.
- **Geprüft** wird das mit `deploy/smoke-test.sh`, lokal und im CI-Job `image`.

Abweichungen vom Entwurf, und was offen ist:

- Die Compose-Datei liegt in `deploy/` und nicht im Wurzelverzeichnis. Dort lagen die Compose-Dateien des alten Systems, als sie entstand; sie sind seit dem 2026-11-11 weg, `deploy/` bleibt trotzdem, weil `.env.example`, Anleitung und Smoke-Test dazugehören.
- **Veröffentlicht** wird das Image seit dem 2026-11-11 von der CI, auf Fionas Wunsch: Jeder Push auf `main`, dessen Jobs grün sind, wird `ghcr.io/fionapreroll/rueder:sha-<commit>` und, solange `main` noch darauf zeigt, `:latest`. Gebaut wird es einmal im Job `image` und vom Smoke-Test gestartet; der Job `publish` baut es aus deren Cache noch einmal und schiebt es. Wer installiert, kann es nehmen (`RUEDER_IMAGE` in `deploy/.env`) oder weiter selbst bauen. Es gibt das Image nur für amd64: Den Server unter Emulation für arm64 zu bauen, dauerte länger als alle anderen Jobs zusammen. Versionsnummern gibt es noch keine.
- PostgreSQL läuft mit seinen Standardwerten; nur `shared_buffers` lässt sich über `POSTGRES_SHARED_BUFFERS` setzen. Die [Messung](implementation.md#messung) lief mit 2 GB.

## Status für Admins

Gebaut in Paket 3.8 (`server/src/monitor.rs`, `server/src/status.rs`, `web/src/lib/components/StatusDialog.svelte`). Ein Admin öffnet im Menü seines Kontos den „Serverstatus“. Die Seite fragt `GET /api/status` und fragt alle 15 Sekunden neu, solange sie offen ist. Wer Admin ist, bestimmt die Kommandozeile: `rueder admin add <name>`, `rueder admin remove <name>`, `rueder admin list`, dazu wie bisher `rueder account add --admin`. Jede Anfrage liest das Recht neu aus der Datenbank, eine Änderung gilt also sofort und ohne neue Sitzung. Alle anderen bekommen 403.

**Was der Server über sich festhält.** Der `Monitor` hält die letzte Stunde im Speicher, in sechs Abschnitten zu zehn Minuten; ein Abschnitt, der älter als eine Stunde ist, wird wiederverwendet. Gezählt werden:

- jede Anfrage nach ihrer Route, mit der Zeit bis zum Beginn der Antwort und der Klasse der Antwort. Die Route ist Methode und Muster aus `MatchedPath`, etwa `GET /api/feeds/{id}/articles`, nie die Adresse selbst. Was keine Route trifft, zählt als `frontend` oder `other`; höchstens 200 Routen. Die Schicht liegt außen um Rate-Limit und Header und sieht also auch die 429. Ein Strom von Ereignissen zählt, bis er offen ist, nicht so lange er offen bleibt.
- jeder Abruf eines Feeds (mit Inhalt, unverändert, fehlgeschlagen, an der Datenbank gescheitert, neue Artikel, Dauer) und die letzte Runde.
- jede Feed-Suche (Feeds gefunden, keine, fehlgeschlagen, über dem Limit des Kontos, Dauer).
- jedes Bild des Proxy (aus dem Cache, abgerufen, abgelehnt nach Grund, Dauer der abgerufenen), dazu der Füllstand des Cache und die Quellen, die er gerade meidet.
- die Warnungen und Fehler des Logs. Eine eigene Schicht von `tracing` gibt sie an `Problems`. Gleicher Text vom gleichen Ort ist ein Problem mit Zähler; Felder, die sich jedes Mal ändern (Dauer, Zeilenzahl), fallen weg. Höchstens 100 Probleme zu je 1000 Zeichen, das neueste zuerst.

Zeiten stehen in Eimern, die bei 0,1 ms beginnen und um eine Drittel-Oktave wachsen. Median und 95. Perzentil sind die Obergrenze ihres Eimers, höchstens die längste gemessene Zeit; sie liegen also bis zu einem Viertel über dem wahren Wert. Ein Zufallstest hält das gegen die sortierten Werte.

**Was dazu aus PostgreSQL kommt**, bei jeder Anfrage: Zahl der Konten und Feeds, die geschätzte Zahl der Artikel (`reltuples`; ein `count(*)` über zehn Millionen Zeilen wäre zu teuer), Größe der Datenbank und der größten Tabellen samt ihrer Partitionen, Verbindungen, Anteil der Blöcke aus dem Speicher, Deadlocks, die am längsten laufende Abfrage und der Pool des Servers. Für den Abruf: Feeds, die mehr als zehn Minuten überfällig sind, Feeds ohne ersten Abruf und die fehlschlagenden Feeds, gruppiert nach Grund.

**Gründe statt Feeds.** Ein fehlschlagender Feed wird nur gezählt, nach dem Grund seines Fehlers: Antwort der Seite mit Status, Zeitüberschreitung, gesperrtes Netz, Name ohne Adresse, keine Verbindung, zu groß, Weiterleitungen, kein lesbarer Feed, anderes. Der Grund wird aus dem gespeicherten Fehlertext gelesen; ein Test bindet das an die Texte des HTTP-Clients und des Abrufs. Adresse, Titel und Abonnenten nennt der Status nie, damit kein Endpunkt fremde Feeds listet. Die Ausnahme sind die Texte des Logs: Die liest, wer den Server betreibt, ohnehin.

**Hinweise.** Der Server nennt, was einen Blick wert ist, als Art mit Zahlen; die Worte hat die Seite. Es gibt Hinweise auf Fehler im Log der letzten Stunde, einen voll belegten Pool, einen stillstehenden oder hinterherhinkenden Abruf, eine Abfrage, die seit 30 Sekunden läuft, Routen mit Fehlern des Servers, Routen mit mindestens fünf Anfragen, deren 95. Perzentil über 500 ms liegt, ein Fünftel oder mehr fehlschlagende Feeds, weniger als 90 % Lesezugriffe aus dem Speicher, einen Bild-Cache über 95 % und abgewiesene Anfragen.

Abweichungen und Grenzen:

- Alles außer den Zahlen aus PostgreSQL ist mit einem Neustart weg und gilt je Prozess. Mehrere Prozesse hätten je ihren eigenen Status; das passt, solange rueder ein Prozess ist.
- Ein eigenes Werkzeug statt Prometheus oder OpenTelemetry, weil es keinen weiteren Dienst neben PostgreSQL braucht. Ein Endpunkt für Prometheus ließe sich später aus denselben Zählern bauen.
- `pg_stat_statements` wird nicht genutzt, weil es eine Erweiterung in PostgreSQL braucht. Langsame Abfragen meldet `sqlx` selbst als Warnung, und die landen bei den Problemen.

### Fehlerberichte aus dem Browser

Gebaut in Paket 3.9 (`server/src/reports.rs`, `server/migrations/0002_client_reports.sql`, `web/src/lib/reports.ts`). Fehler, die im Browser eines Users auftreten, kommen auf die Statusseite, aber nur von Usern, die zustimmen, und ohne fremden Dienst: rueder sammelt sie selbst, der Code liegt offen im Repository, die Daten bleiben beim Betreiber. Fiona hat am 2026-11-11 entschieden: aus als Standard, 30 Tage Aufbewahrung, Admins sehen keine Namen von Konten.

- **Zustimmung.** Der Reiter „Datenschutz“ der Einstellungen hat einen Haken „Fehlerberichte aus diesem Browser an diesen Server schicken“, aus als Standard, und sagt, was ein Bericht enthält. Die Zustimmung steht beim Konto (`accounts.sends_reports`, `GET` und `PUT /api/reports/consent`), nicht in den Einstellungen der Seite, damit der Server sie prüfen kann. Nur der User selbst schaltet sie ein; Admins können das nicht für andere. Ausschalten löscht die Berichte des Kontos. Andere Tabs hören davon über das Ereignis `settings`. Mit `CLIENT_REPORTS=off` nimmt der Server keine Berichte an, und der Reiter fehlt.
- **Was ein Bericht enthält.** Unbehandelte Fehler (`error`), fehlgeschlagene Promises (`unhandledrejection`), Teile der Seite, die beim Zeichnen scheitern (`<svelte:boundary>` um die Seite, die dann anbietet, neu zu laden), und Anfragen an die API, an denen der Server gescheitert ist (5xx) oder die keine Antwort bekamen. Was der Server ablehnt (4xx), ist kein Fehler der Seite. Dazu die Version des Frontends und der Browser mit Hauptversion, sonst nichts aus dem User-Agent. Der Text wird vor dem Senden auf eine Zeile und 500 Zeichen gebracht, Adressen werden `<address>`, Zahlen nach `/`, `=` oder `#` und Zahlen ab vier Stellen werden `<n>`. Vom Stack bleiben nur Zeilen in den eigenen Dateien (`_app/…js:Zeile:Spalte`) desselben Hosts, höchstens zehn. Nichts aus Artikeln, Feeds, Labels oder Regeln.
- **Wohin.** Nur an den eigenen Server, `POST /api/reports`: gesammelt, höchstens 20, alle 30 Sekunden und beim Verlassen der Seite, dann als `fetch` mit `keepalive`. Ein Fehler, der zweimal wartet, wird einmal geschickt. Der Browser spricht weiter mit nichts als rueder.
- **Auf dem Server.** Er vertraut keinem Browser und bereinigt Text und Stack ein zweites Mal mit denselben Regeln; Version und Browser behalten nur Buchstaben, Ziffern, Punkt, Strich und Leerzeichen. Ohne Zustimmung 403, mehr als 20 Berichte 400, ein Körper über 32 KB 413, und je Konto 100 Berichte auf einmal und etwa 100 am Tag (429).
- **Speicher.** Tabelle `client_reports` mit einem Fingerabdruck (SHA-256 über Art, Meldung und erste eigene Stack-Zeile), Zähler, erstem und letztem Auftreten, Version und Browser, je Konto in einer Zeile gezählt. Ein Bericht ist an das Konto gebunden, damit Ausschalten und das Löschen eines Kontos ihn mitnehmen. Stündlich geht, was vor mehr als 30 Tagen zuletzt auftrat.
- **Anzeige.** Der Abschnitt „Fehler in Browsern“ der Statusseite fasst je Fingerabdruck zusammen: Art, Meldung, Stack, wie oft, bei wie vielen Konten, Versionen, Browser, zuletzt; die 20 zuletzt aufgetretenen. Namen von Konten zeigt er nicht. Dazu, wie viele Konten Berichte schicken, und ein Hinweis, wenn am letzten Tag Fehler gemeldet wurden.
- **Verworfen:** ein selbst gehostetes Sentry oder GlitchTip mit deren SDK. Das wäre ein weiterer Dienst neben PostgreSQL, das SDK ist groß, und die Content-Security-Policy müsste dem Browser einen zweiten Host erlauben. Ebenso `navigator.sendBeacon`: `fetch` mit `keepalive` leistet dasselbe und geht durch denselben Client wie jede Anfrage.

## Android-App

Gebaut in Paket 3.11 (`app/`, beschrieben in [app/README.md](../app/README.md)). Capacitor um eine kleine Startseite, die nach der Adresse des eigenen rueder fragt und ihn dann öffnet. Die Seite von rueder kommt vom Server wie im Browser: gleicher Ursprung, das Sitzungs-Cookie, die Content-Security-Policy und die Prüfung der Herkunft bleiben, wie sie sind, und auch die PWA mit ihrem Hinweis auf neue Versionen läuft in der App.

- **Startseite** (`app/www/`): Adresse (nur https, nur der Ursprung, weil rueder ganz oben auf seinem Host läuft) und die Zustimmung zur Update-Suche, aus als Standard. Bekannte Server öffnet sie sofort. Zurück kommt man über eine Verknüpfung am App-Icon („Server und Updates“) und über das Kontomenü der Seite, das in der App einen Eintrag dafür hat (erkannt an `rueder-android` im User-Agent, `web/src/lib/app.ts`). Antwortet der Server nicht, sagt es die Startseite.
- **Wohin die WebView darf** (`ServerClient`): Seiten des eingestellten Servers bleiben in der App, die Startseite auch, alles andere öffnet der Browser. Login über OpenID Connect geht deshalb in der App nicht, nur mit Passwort.
- **Das Plugin der App** (`RuederPlugin`) antwortet nur der Startseite. Die Seite des Servers läuft in derselben WebView. Capacitor gibt seine Brücke nur der Startseite (`https://localhost`), wenn die WebView Web-Message-Listener kennt (ab Version 86 von Android System WebView); auf älteren gibt es sie jeder Seite, und das Plugin prüft deshalb selbst, welche Seite gerade gezeigt wird. Ein Skript der Seite des Servers darf so weder den Server ändern noch die Update-Suche. Die Adresse prüft auch die native Seite (`Address.java`).
- **Updates** nur im Build `github`: Mit Zustimmung liest die App bei jedem Start `version.json` aus dem Release `android-latest` (höchstens 4 KB, mit Build-Nummer, Commit und Größe), vergleicht die Build-Nummer mit ihrer eigenen und bietet einen neueren Build in einem Dialog an, mit Größe und einem Hinweis bei Netzen, die Geld kosten können. Den Download macht Androids Download-Manager, die Größe wird geprüft. Weil der Download-Manager in den Ordner der App auf dem geteilten Speicher schreibt, in den bis Android 9 jede App mit Speicher-Berechtigung schreiben darf, kopiert die App das APK in ihren eigenen Cache und gibt es nur weiter, wenn es diese App in dem Build ist, den `version.json` nennt (seit dem [Nachtrag zum Security-Review](#nachtrag-app-service-worker-ci-und-deploy)). Die Installation übernimmt Androids Installer, der fragt und ein Update nur mit demselben Schlüssel annimmt; ein APK eines anderen Pakets prüft er nicht gegen den Schlüssel, sondern installiert es als eigene App. Sonst geht nichts nach außen.
- **Zwei Builds**: `github` mit Update-Suche und der Berechtigung, Apps zu installieren; `play` für den Play Store ohne beides (`src/play/`), weil der Store Updates selbst bringt.
- **CI** (`app` und `app-release` in `ci.yml`): Tests der Startseite und der nativen Teile, beide Builds. Auf `main` ersetzt `app-release` das Release `android-latest` durch das signierte APK und `version.json`, wenn die Prüfungen bestanden sind und der Schlüssel in den Secrets liegt; sonst baut die CI ein Debug-APK und veröffentlicht nichts. Den Schlüssel bekommt nur ein Push auf `main`, nie ein Pull Request: Der führt seine eigenen, noch nicht gemergten Build-Skripte aus. Der Schlüssel liegt nie im Repository (anders als im Vorbild, dessen öffentlicher Schlüssel nur beweist, dass ein APK aus einem Build des Projekts stammt).

Abweichungen vom Vorbild: eigene Update-Logik statt `@fionapreroll/app-updates`, das nicht aus der npm-Registry kommt; kein Fortsetzen eines abgebrochenen Downloads über einen Neustart der App hinaus, das übernimmt der Download-Manager, soweit er es kann. Die Texte der App stehen nicht in `web/src/lib/i18n/`, weil sie nicht Teil der Seite sind: die der Startseite in `app/www/texts.js`, die nativen in `res/values*/strings.xml`, jeweils Englisch und Deutsch.

