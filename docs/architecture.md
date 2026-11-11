# rueder: Architektur des neuen Systems

Stand 2026-11-11. Dieses Dokument beschreibt, wie der neue rueder aufgebaut ist und warum. Den Plan mit Anforderungen und Entscheidungen führt [plan.md](plan.md), die Umsetzung mit Stand und Arbeitspaketen [implementation.md](implementation.md).

Vom Server stehen seit Schritt 1 das Schema, der Artikelspeicher, der Lesestand und der Import, aus Schritt 2 der gehärtete HTTP-Client, der Abruf, die Bereinigung, der Bild-Proxy und die Live-Updates. Alles andere hier ist der Entwurf, nach dem gebaut wird. Sobald ein Teil gebaut ist, gilt der Code: für das Schema die Migrationen in `server/migrations/`, für Artikelspeicher und Lesestand `server/src/articles.rs` und `server/src/reading.rs`, für den HTTP-Client `server/src/http_client.rs`, für die Bereinigung `server/src/cleaning.rs`, für den Bild-Proxy `server/src/images.rs`, für Filterregeln und Ordnerbaum die Modelle in `web/src/lib/`. Wer von diesem Dokument abweicht, ändert es im selben Pull Request.

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
- [Security](#security)
- [API](#api)
- [Frontend](#frontend)
- [Betrieb](#betrieb)
- [Status für Admins](#status-für-admins)

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

Die Tabellen im Überblick. Die genauen Spalten stehen in der ersten Migration, `server/migrations/0001_initial.sql`.

| Bereich | Tabellen | Idee |
|---|---|---|
| Konten | `accounts`, `account_identities`, `sessions` | Ein Konto hat einen Login-Namen, optional einen Argon2id-Hash und Einstellungen als JSON. `account_identities` verknüpft Konten mit OpenID-Connect-Anbietern (Aussteller und Subject) und mit dem alten Login (`auth_origin`, `auth_subject`). Eine Sitzung ist der SHA-256 des Cookie-Tokens mit Ablaufzeit. |
| Feeds | `feeds` | Ein Feed gehört allen seinen Abonnenten gemeinsam. Er kennt seine höchste Artikelnummer (`last_seq`), den Abrufstand (nächster Abruf, Intervall, ETag, Last-Modified, letzter Erfolg, letzter Fehler) und den User-Agent, mit dem er abgerufen wird. |
| Baum | `folders`, `subscriptions` | Ordner mit Elternordner und Position. Ein Abo verbindet Konto und Feed, liegt in einem Ordner oder auf oberster Ebene, hat eine Position und trägt die Lesemarke. |
| Artikel | `articles`, `article_content`, `article_versions` | Heiß, warm und kalt getrennt, siehe [Artikelspeicher und Versionen](#artikelspeicher-und-versionen). |
| Lesestand | `subscriptions.read_upto`, `article_states` | Lesemarke je Abo plus Ausnahmen je Artikel, siehe [Lesestand und Ungelesen-Zahlen](#lesestand-und-ungelesen-zahlen). |
| Regeln | `filter_rules` | Filterregeln je Konto, siehe [Filterregeln](#filterregeln). |
| Labels | `labels`, `article_labels` | Labels mit Farbe je Konto und ihre Zuordnung zu Artikeln. Der Stern ist kein Label, sondern eine Ausnahme in `article_states`. |

Schlüssel sind fortlaufende Zahlen (`bigint`), keine UUIDs: Sie halten Zeilen und Indizes schmal, und neue Artikel liegen im Index beieinander. Feeds und Konten behalten ihre alte UUID in einer Spalte `legacy_id`, Artikel in der Tabelle `legacy_articles`, damit der Import weiß, was er schon übernommen hat, und Lesestand und Labels aus dem Browser zugeordnet werden können. Die Tabelle statt einer Spalte hält die heiße Artikeltabelle schmal und kann nach dem Umstieg weg.

## Artikelspeicher und Versionen

Listen lesen nie Artikeltext, und alte Versionen berühren nie die Tabellen, aus denen neue Artikel gelesen werden. Dafür wird die eine Tabelle von heute in drei geteilt:

| Tabelle | Inhalt | Wird gelesen | Größe und Pflege |
|---|---|---|---|
| `articles` (heiß) | Eine schmale Zeile je Artikel: Feed, laufende Nummer im Feed, Titel, Anriss, Autor, Datum, Vorschaubild, Nummer der aktuellen Version, Zeitpunkt der letzten Änderung | Bei jeder Liste und jeder Zählung | Klein genug für den Arbeitsspeicher. Index auf Feed und Nummer. |
| `article_content` (warm) | Genau eine Zeile je Artikel: der bereinigte Text der aktuellen Version, dazu was nur die Artikelansicht braucht (Schlagwörter, Anhänge, Kennung des Feeds für den Artikel) | Beim Öffnen eines Artikels, per Primärschlüssel | Wächst mit der Zahl der Artikel, nicht mit der Zahl der Versionen. |
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
- Grenzen je Konto für Ordner und Abos, wie heute (100 Ordner, 1.000 Feeds je Ordner).

Das Modell mit Tests ist `web/src/lib/tree.ts` (`canMove`, `moveNode`, `nudge` für die Tastatur).

Gebaut in Paket 2.6 (`server/src/tree.rs`, `server/src/subscribe.rs`):

- **Züge.** `POST /api/tree/move` nimmt Knoten, Ziel und `before`, `after` oder `inside`, wie das Modell im Frontend. Ein Test spielt zufällige Züge gegen ein Modell, das `tree.ts` nachgebaut ist. Jede Änderung am Baum sperrt die Zeile des Kontos, es läuft also je Konto nur eine.
- **Grenzen.** 100 Ordner je Konto, 1.000 Feeds je Ordner und ebenso auf der obersten Ebene, Namen bis 100 Zeichen.
- **Feed-Suche.** `POST /api/feeds/find` sieht sich eine Adresse an. Ist sie ein Feed, ist das der eine Treffer. Ist sie eine Webseite, nennt die Antwort die Feeds, auf die die Seite mit `<link rel="alternate">` zeigt, höchstens 20. Gelesen werden die ersten 512 kB der Seite, mit dem Parser der [Bereinigung](#bereinigung) und in dessen Grenzen. Die Suche legt nichts an. **YouTube:** Die Seite eines Kanals nennt seinen Feed zuerst, `https://www.youtube.com/feeds/videos.xml?channel_id=UC…`, mit dem Namen des Kanals. Bei Adressen wie `youtube.com/@name` steht die Kennung des Kanals im `<meta property="og:url">` der Seite (`…/channel/UC…`); eine Adresse `youtube.com/channel/UC…` führt ohne Umweg zum Feed, die Seite wird gar nicht erst geholt. Gilt nur für `youtube.com`, `www.youtube.com` und `m.youtube.com` und nur für eine Kennung aus „UC“ und 22 Buchstaben, Ziffern, `-` oder `_`. Schickt YouTube statt der Seite seine Frage nach Cookies (`consent.youtube.com`, wie es in der EU vorkommt), findet die Suche bei `@name` nichts.
- **Abonnieren.** `POST /api/feeds` abonniert die Adresse eines Feeds. Einen Feed, den noch niemand liest, ruft der Server dabei ab und behält ihn nur, wenn das gelingt: Aus einer beliebigen Adresse wird so kein Feed. Eine Adresse mit Name und Passwort wird abgelehnt, denn ein Feed gehört allen, die ihn lesen. Ohne Schema gilt eine Adresse als https.
- **Lesemarke eines neuen Abos.** Wer einen Feed abonniert, den andere schon lesen, findet die 20 neuesten Artikel ungelesen und alles davor gelesen. Ein neuer Feed bringt mit, was sein erster Abruf liefert. Die [Filterregeln](#filterregeln) des Kontos sehen sich den Feed sofort an.
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

## Abruf

Ein Planer im Server ruft jeden Feed mit mindestens einem Abonnenten in seinem Intervall ab.

1. Fällige Feeds kommen aus der Datenbank (`next_fetch_at`), begrenzt viele laufen gleichzeitig, je Host nur einer.
2. Der gehärtete HTTP-Client holt den Feed, bedingt mit ETag und Last-Modified. „Nicht geändert“ beendet den Abruf.
3. `feed-rs` liest RSS, Atom oder JSON Feed, mit Grenzen für Größe und Zahl der Artikel. Atom 0.3, das manche Websites noch senden (die Tagesschau etwa), schreibt der Server vorher als Atom 1.0 um, weil `feed-rs` nur dessen Namensraum liest und die Elemente der älteren Fassung übergeht: Namensraum, die umbenannten Elemente (`issued`, `modified`, `tagline`, `copyright`), die Art der Texte (`mode="escaped"` wird `type="html"`) und `dc:date` als `updated`. Escapter HTML-Text, der zusätzlich in einer CDATA-Markierung als Text steckt, verliert diese Hülle. Davor bekommt `feed-rs` jeden Feed in XML als UTF-8 und ohne Deklarationen, die eine andere Kodierung nennen: Die Bibliothek wechselt sonst mitten im Lesen die Kodierung und bricht mit einer Panic ab, wenn die Deklaration nach den ersten 64 Bytes steht, bei einer langen Deklaration oder einer zweiten weiter unten (gefunden beim [Fuzzing](#fuzzing)). Die Kodierung nennt das Byte Order Mark, sonst die Deklaration. Adressen des Feeds (Seite, Icon, Links der Artikel, Vorschaubilder, Anhänge) bleiben nur, wenn sie ganze `http`- oder `https`-Adressen mit Host sind; die Kennung eines Artikels nimmt den Link weiter so, wie der Feed ihn nennt.
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

Abweichungen vom Entwurf oben: Schritt 7 meldet noch nichts, das kommt mit Paket 2.7. Die Filterregeln aus Schritt 6 gibt es seit Paket 2.6. Anders als der alte rueder probiert der Abruf bei einer http-Adresse nicht zuerst https, und eine dauerhafte Weiterleitung ändert die gespeicherte Adresse des Feeds nicht.

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
- Der alte Cleaner merkte sich, ob ein Text ein Bild enthält, um das Aufmacherbild nicht doppelt zu zeigen. Das entscheidet künftig das Frontend (Schritt 3).

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
| `labels` | Label angelegt, geändert, gelöscht, zugeordnet | `by` | Labels und die sichtbare Liste neu laden. |
| `settings` | Einstellungen geändert | `by` | Einstellungen neu laden. |
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

- **Lokale Konten** mit Passwort-Hash nach Argon2id (19 MiB, 2 Durchläufe, ein Salt je Hash). Es gibt keine offene Registrierung: Konten legt die Kommandozeile an (`rueder account add`, `password`, `link`, `list`). Ein Name ist höchstens 64 Zeichen lang und unterscheidet keine Groß- und Kleinschreibung, ein Passwort hat mindestens 8 Zeichen und höchstens 1024 Byte. Höchstens vier Hashes werden gleichzeitig gerechnet, damit Login-Versuche den Speicher nicht füllen.
- **Sitzung** als zufälliges Token (32 Byte) im Cookie `__Host-rueder` mit `HttpOnly`, `Secure`, `SameSite=Lax` und `Path=/`. In der Datenbank liegt nur der SHA-256 des Tokens. Eine Sitzung endet nach 30 Tagen ohne Nutzung, beim Abmelden und sobald das Konto ein neues Passwort bekommt; ein neuer Login ersetzt die Sitzung, die der Browser noch hatte. Dass eine Sitzung genutzt wurde, schreibt der Server höchstens einmal je Stunde auf. Abgelaufene Sitzungen räumt er stündlich weg.
- **Wer eingeloggt ist**, sagt der Extraktor `Session` mit der Nummer des Kontos. Jeder Handler, der ihn nimmt, antwortet ohne Sitzung mit 401; an seine Kontonummer binden die Endpunkte aus Paket 2.6 ihre Abfragen. Der Bild-Proxy unter `/img/` nimmt ihn schon.
- **Login** über `POST /api/login` mit Name und Passwort als JSON, `POST /api/logout`, `GET /api/me`. `GET /api/login/ways` sagt der Login-Seite ohne Sitzung, welche Wege es außer Name und Passwort gibt (`{"oidc": true}`), und sonst nichts. Ein falscher Name, ein falsches Passwort und ein Konto ohne Passwort bekommen dieselbe Antwort nach derselben Arbeit: Für einen Namen, den es nicht gibt, wird gegen einen Hash geprüft, dessen Passwort niemand kennt.
- **Rate-Limit** auf falsche Passwörter: 10 je Name und 30 je Adresse in 15 Minuten, danach 429 mit `Retry-After`, auch für das richtige Passwort. Bei IPv6 zählt ein /64 als eine Adresse. Ein Login, der klappt, setzt den Zähler des Namens zurück. Gezählt wird im Speicher des Servers, für höchstens 10.000 Namen und Adressen. Hinter einem Reverse-Proxy kommt jede Verbindung vom Proxy; `CLIENT_IP_HEADER` nennt den Header, in den der Proxy die Adresse des Absenders schreibt (etwa `X-Forwarded-For`), und es zählt dessen letzter Eintrag, also der, für den der Proxy selbst bürgt.
- **OpenID Connect** als Option, eingestellt über `OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET` und `OIDC_REDIRECT_URL`. Authorization Code Flow mit PKCE: `GET /api/oidc/start` schickt den Browser zum Anbieter, `GET /api/oidc/callback` tauscht den Code gegen das ID-Token. State, Nonce und Code-Verifier eines Logins liegen zehn Minuten lang in einem eigenen Cookie (`__Host-rueder-oidc`, dieselben Attribute). Geprüft werden State, Aussteller, Empfänger (`aud`, `azp`), Ablauf und Nonce. Der Weg endet mit einer Weiterleitung auf `/`, bei einem Fehler auf `/?oidc=failed`, `denied` oder `unknown`.
- **Niemand bekommt durch OpenID Connect ein Konto.** `account_identities` sagt, welches Konto zu Aussteller und Subject gehört. Die Verknüpfung entsteht auf der Kommandozeile (`rueder account link <name> <subject>`) oder dadurch, dass jemand eingeloggt ist und den Weg mit `/api/oidc/start?link` beginnt. Ein unbekanntes Subject wird abgewiesen.
- Der Anbieter wird wie alles andere über den [gehärteten HTTP-Client](#gehärteter-http-client) erreicht. Der kann dafür Formulare senden; ein solcher Abruf folgt keiner Weiterleitung, damit Code und Secret bei dem bleiben, für den sie bestimmt sind.
- Das Frontend sieht nie ein Token. JWTs als Sitzung entfallen.

Abweichungen vom Entwurf, und was offen ist:

- **Die Signatur des ID-Tokens wird nicht eigens geprüft** (von Fiona am 2026-11-10 bestätigt, ebenso dass durch OpenID Connect nie ein Konto entsteht). Das Token kommt beim Code Flow direkt vom Token-Endpunkt des Anbieters über TLS und nie durch den Browser; OpenID Connect Core 3.1.3.7 lässt dafür die Prüfung des TLS-Zertifikats an Stelle der Signatur zu. Das erspart die Abhängigkeiten für JWT-Signaturen und Schlüsselabruf. Dafür müssen alle Adressen des Anbieters https sein, sonst startet der Server nicht. Das Security-Review (Paket 2.12) soll diese Entscheidung noch einmal ansehen.
- Der Cookie verlangt HTTPS, weil `Secure` und das Präfix `__Host-` fest sind. Ohne TLS lässt sich nur über `localhost` einloggen, wo Browser eine Ausnahme machen.
- Das Rate-Limit je Name kann ein Konto 15 Minuten lang aussperren, wenn jemand absichtlich falsche Passwörter schickt. Wer schon eine Sitzung hat, merkt davon nichts.
- **Eigene Sitzungen** (seit Paket 2.6, `server/src/profile.rs`). `GET /api/sessions` zeigt, wo ein Konto eingeloggt ist, mit Beginn und letzter Nutzung; `DELETE /api/sessions/{id}` beendet eine davon, `DELETE /api/sessions` alle anderen. Der Name einer Sitzung in dieser Liste sind die ersten acht Byte des SHA-256 ihres Tokens; aus ihm lässt sich kein Token bilden. Gerät oder Browser merkt sich der Server nicht.
- **Neues Passwort.** `PUT /api/password` verlangt das bisherige Passwort, zählt Fehlversuche wie der Login (eine gestohlene Sitzung kann also nicht endlos raten), beendet alle Sitzungen des Kontos und gibt dem fragenden Browser eine neue. Ein Konto ohne Passwort, etwa nach einem Login über OpenID Connect, setzt sein erstes ohne das alte.
- **Konten anlegen.** `GET` und `POST /api/accounts` sind einem Admin vorbehalten: die Liste der Konten und ein neues Konto mit einem ersten Passwort, das der Admin weitergibt. Der Einmal-Link aus den [Detailfragen](plan.md#detailfragen) ist für importierte Konten ohne Passwort gedacht und kommt mit dem Import (Paket 4.3); bis dahin setzt `rueder account password` ein Passwort.
- Die Prüfung des `Origin`-Headers gibt es seit Paket 2.9, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). Seither hat ein Konto auch höchstens 100 Sitzungen; mit der nächsten endet die, die am längsten nicht genutzt wurde.

Wie die bestehenden Konten umziehen, steht bei den [Detailfragen](plan.md#detailfragen): Der Import führt sie mit den Konten zusammen, die es im neuen rueder schon gibt, und legt nur die übrigen ohne Passwort an.

## Security

Der Server ruft Adressen ab, die User eingeben, und zeigt HTML an, das Fremde geschrieben haben. Diese beiden Stellen sind die größten Angriffsflächen und bekommen je eine zentrale, getestete Komponente. Gegen diese Tabelle laufen die Security-Reviews am Ende von Schritt 2 und vor der Inbetriebnahme.

| Angriffsfläche | Maßnahme |
|---|---|
| Login und Sitzung | Sitzung als Cookie mit `HttpOnly`, `Secure` und `SameSite=Lax`. Das Frontend sieht kein Token mehr. Passwörter mit Argon2id, Rate-Limit auf dem Login, OpenID Connect als Option. |
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
| Missbrauch | Limits je Konto für Feeds, Ordner, Regeln, Labels und Sitzungen. Rate-Limits je Adresse auf allem und je Konto auf Abrufen fremder Adressen. Gebaut, siehe [Header, Herkunft und Limits](#header-herkunft-und-limits). |
| Lieferkette und Betrieb | `cargo audit` und `cargo deny` in der CI, Lockfiles, Container ohne Root mit schreibgeschütztem Dateisystem. Geheimnisse kommen nie in den Frontend-Build. |

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
- **Styles** nur aus den eigenen Dateien, auch keine `style`-Attribute. Svelte setzt berechnete Styles über das DOM, das ist davon nicht betroffen. SvelteKit versteckt sein Element für Screenreader (`#svelte-announcer`) mit einem `style`-Attribut; das lehnt der Browser ab und meldet es einmal in der Konsole. Dieselben Regeln stehen deshalb in `web/src/routes/layout.css`.
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

Der Server braucht dafür keine Einstellung mit seiner eigenen Adresse. Der Reverse-Proxy muss für Punkt 2 den `Host`-Header durchreichen; Caddy tut das von selbst, das nginx-Beispiel setzt ihn. Lesende Anfragen von fremden Seiten bleiben erlaubt, sonst ließe sich rueder nicht über einen Link öffnen; dort schützt `SameSite=Lax` des Cookies. Nicht abgedeckt ist `GET /api/oidc/start?link`, das einem eingeloggten Konto die eigene Identität beim Anbieter zuordnet; das sieht sich das Security-Review (Paket 2.12) an.

**Rate-Limits**, gezählt im Speicher des Servers für höchstens 10.000 Adressen und Konten. Wer darüber liegt, bekommt 429 mit `Retry-After`.

| Was | Grenze | Gezählt je |
|---|---|---|
| Jede Anfrage, auch an Frontend und Bild-Proxy | 1200 auf einmal, danach 50 je Sekunde | Adresse (bei IPv6 das /64) |
| Abrufe fremder Adressen: Feed-Suche und das Abonnieren eines Feeds, den noch niemand liest | 30 auf einmal, danach einer alle zwei Sekunden | Konto |
| Falsche Passwörter | 10 je Name und 30 je Adresse in 15 Minuten, siehe [Login und Sitzungen](#login-und-sitzungen) | Name und Adresse |
| Offene Live-Verbindungen | 20, siehe [Live-Updates](#live-updates) | Konto |

Die Grenze je Adresse liegt weit über dem, was Lesen braucht: Eine Liste lädt bis zu 200 Vorschaubilder auf einmal, und mehrere Leser können eine Adresse teilen. Sie soll verhindern, dass einer den Server für alle belegt. Hinter einem Reverse-Proxy ohne `CLIENT_IP_HEADER` zählen alle Leser als eine Adresse. Die Bilder des Proxy brauchen keine eigene Grenze je Konto, weil nur der Server ihre Adressen signiert: Niemand kann ihn darüber zu beliebigen Abrufen bringen. Ein OPML-Import ruft nichts ab, seine Feeds holt die nächste Runde des Abrufs.

**Limits je Konto.** Was darüber hinausgeht, lehnt die API mit 400 ab.

| Was | Grenze | Wo |
|---|---|---|
| Abonnierte Feeds | 2000, davon höchstens 1000 in einem Ordner | `tree::MAX_FEEDS`, `tree::MAX_FEEDS_IN_FOLDER` |
| Ordner | 100, höchstens fünf Ebenen | `tree::MAX_FOLDERS` |
| Filterregeln | 200, ein Muster höchstens 200 Zeichen | `filters::MAX_RULES`, `filters::MAX_PATTERN` |
| Labels | 100 | `labels::MAX_LABELS` |
| Sitzungen | 100, die am längsten ungenutzte endet | `sessions::MAX_PER_ACCOUNT` |
| Einstellungen | 16 KB | `tree::MAX_SETTINGS` |
| Eine Anfrage an die API | 256 KB, eine OPML-Datei 1 MB mit höchstens 2000 Feeds | `MAX_BODY`, `subscribe::MAX_OPML` |

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
| Konto | `GET /api/sessions`, `DELETE /api/sessions`, `DELETE /api/sessions/{id}`, `PUT /api/password`; für Admins `GET` und `POST /api/accounts` |
| Baum | `GET /api/tree` (Ordner, Abos, Feeds mit `last_seq`, Lesemarken, Ungelesen-Zahlen und dem Fehler des letzten Abrufs, dazu die Zahl der ausgeblendeten Artikel), `POST /api/tree/move`, `POST /api/folders`, `PATCH` und `DELETE /api/folders/{id}` |
| Abos | `POST /api/feeds/find`, `POST /api/feeds`, `PATCH` und `DELETE /api/feeds/{id}`, `PUT /api/feeds/{id}/agent`, `GET` und `POST /api/opml` |
| Listen | `GET /api/feeds/{id}/articles`, `GET /api/folders/{id}/articles`, `GET /api/articles` (alle), `GET /api/starred`, `GET /api/hidden`, `GET /api/labels/{id}/articles`, jeweils mit `before` oder `after`, `limit` und `unread` |
| Artikel | `GET /api/feeds/{id}/articles/{seq}` (bereinigter Text, Anhänge, Aufmacherbild, die ausblendende Regel), `GET /api/feeds/{id}/articles/{seq}/versions` |
| Lesestand | `PUT /api/feeds/{id}/articles/{seq}/state` mit `read` und `starred`; „alle als gelesen“ mit `POST /api/feeds/{id}/read`, `POST /api/folders/{id}/read` und `POST /api/read` |
| Regeln | `GET` und `POST /api/rules`, `PUT` und `DELETE /api/rules/{id}`, `POST /api/rules/order`, `POST /api/rules/preview`, `GET /api/rules/hits` |
| Labels | `GET` und `POST /api/labels`, `PATCH` und `DELETE /api/labels/{id}`, `POST /api/labels/order`, `PUT` und `DELETE /api/feeds/{id}/articles/{seq}/labels/{label}` |
| Einstellungen | `GET` und `PUT /api/settings` |
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
- Die Volltextsuche auf dem Server (Detailfrage in [plan.md](plan.md#detailfragen)) ist nicht Teil dieses Pakets. Sie hängt an der Messung ihres Index und ist als eigenes Paket 2.13 geführt.

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
- **Listen** kommen seitenweise (40 Artikel, Anker `before`, bei „Älteste zuerst“ `after`) und ohne Texte. Eine Liste, die beim ältesten beginnt, fragt zugleich nach dem neuesten Artikel, damit „alle als gelesen“ weiß, wo es aufhört. Der Text eines Artikels wird geholt, wenn er gezeigt wird, höchstens vier zugleich, und mit einem geöffneten Artikel der, den der nächste Schritt öffnen würde (Paket 3.6). Die Suche durchsucht, was geladen ist.
- **Lange Listen** (Paket 3.6) zeichnen nur, was in der Nähe des Sichtbaren liegt: ab 61 Zeilen in der Listenansicht, ab 13 Karten in der erweiterten, die mit ihren Texten schwer sind. Was darüber und darunter liegt, ist leerer Raum in der Höhe, die die Zeilen beim Zeichnen hatten; für Zeilen, die nie gezeichnet wurden, gilt der Durchschnitt der gezeichneten. Gezeichnet wird bis 1200 Pixel über und unter dem Sichtbaren, damit schnelles Scrollen keine Lücke zeigt. Die Rechnung steht in `web/src/lib/virtual.ts`, ein Zufallstest hält sie gegen das Modell, jede Zeile einzeln zu fragen. Ein Schritt mit der Tastatur zu einem Artikel, der nicht gezeichnet ist, scrollt erst an seine Stelle. Dass in der erweiterten Ansicht gelesen ist, was oben hinausscrollt, ergibt sich aus denselben Höhen. Die erweiterte Ansicht holt so auch nur die Texte der Karten in der Nähe, statt einer ganzen Seite auf einmal.
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

Abweichungen vom Entwurf:

- **Die Ungelesen-Zahlen rechnet der `ServerReader` nicht selbst.** Der Entwurf sah vor, dass das Modell aus `unread.ts` (Lesemarke und Ausnahmen) im Frontend bleibt. Die API liefert aber je Feed die fertige Zahl und je Artikel „gelesen“ und „Stern“, nicht die Ausnahmen. Der `ServerReader` übernimmt die Zahl des Servers und bewegt sie bei einer eigenen Änderung um eins; nach „alle als gelesen“ und nach jeder Ablehnung fragt er den Server neu. So gibt es weiterhin keine zweite Stelle, die zählt. Das Modell `unread.ts` bleibt das des `DemoReader` und der Maßstab für den Server.
- **Ein neuer Ordner erscheint erst, wenn der Server ihn hat,** weil der Server seine Nummer vergibt. Alle anderen Änderungen am Baum wirken sofort.
- **Welche Ordner zugeklappt sind,** merkt sich der Browser (`localStorage`), nicht der Server.
- Was die Oberfläche mit Server noch weglässt, steht bei [Paket 3.1](implementation.md#schritt-3-frontend).

Was noch fehlt und Schritt 3 ergänzt: eine virtualisierte Liste für sehr lange Feeds, Vorladen des nächsten Artikels.

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
