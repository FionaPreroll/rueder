# rueder: Plan für den Rewrite in Rust

Stand 2026-11-14. Dieses Dokument sagt, was gebaut wird, warum, in welcher Reihenfolge und was entschieden ist. Wie das neue System aufgebaut ist, steht in [architecture.md](architecture.md). Wie gearbeitet wird, was fertig ist und welche Arbeitspakete offen sind, steht in [implementation.md](implementation.md).

- [Ziel und Anlass](#ziel-und-anlass)
- [Anforderungen](#anforderungen)
- [Ausgangslage](#ausgangslage)
- [Was den Plan trägt](#was-den-plan-trägt)
- [Entscheidungen](#entscheidungen)
- [Plan in fünf Schritten](#plan-in-fünf-schritten)
- [Messziel](#messziel)
- [Nach dem Umstieg](#nach-dem-umstieg)
- [Risiken](#risiken)
- [Detailfragen](#detailfragen)

## Ziel und Anlass

rueder ist ein selbst gehosteter Feedreader, entstanden nach dem Ende des Google Reader. Er lief über Jahre in Produktion und wurde mit steigender Artikelzahl langsam. Er wird deshalb neu gebaut: als ein einziges Rust-Programm mit PostgreSQL und einem Svelte-Frontend mit TailwindCSS im Stil des Google Reader. Das Rust-Programm ersetzt die fünf Go-Dienste (`api`, `authbackend`, `events`, `feedfinder`, `worker`) sowie RabbitMQ, loginsrv und imgproxy.

Die Daten der Produktion bleiben erhalten: Feeds, Artikel, User und Ordner kommen aus der alten Datenbank, Lesestand und Labels aus dem Browser der User.

## Anforderungen

Die Nummern werden in den Arbeitspaketen in [implementation.md](implementation.md) wieder aufgegriffen.

| Nr. | Anforderung | Umsetzung in Kürze |
|---|---|---|
| A1 | Schnell bleiben, auch wenn die Artikelzahl wächst | Listen lesen nur eine schmale Tabelle und blättern über eine feste Nummer je Feed. |
| A2 | Mehrere Versionen alter Artikel aufbewahren, ohne dass neuere, häufig gelesene Artikel darunter leiden | Aktueller Text und alte Versionen liegen in getrennten Tabellen; das Archiv ist komprimiert und nach Monaten geteilt. |
| A3 | Durchdachte Security | Sitzungs-Cookies, ein gehärteter HTTP-Client für alle fremden Adressen, Bereinigung und Bild-Proxy auf dem Server, Zugriff nur auf eigene Abos. |
| A4 | Schnelle, bequeme, intuitive UX im Stil des Google Reader, modernisiert | Dichte Listenansicht, Volltextansicht, seit 2026-11-12 auch drei Bereiche und ein Magazin, Tastaturbedienung, Dark Mode, Ansicht fürs Handy. |
| A5 | Anpassbar | Thema, Akzentfarbe, Dichte, Schrift, Ansicht und Sortierung, Tastaturbelegung; gespeichert auf dem Server. |
| A6 | Live-Updates von Feeds | Eine SSE-Verbindung meldet neue Artikel, Lesestand anderer Geräte, Ordneränderungen und Abruffehler. |
| A7 | Verlässliche Ungelesen-Zahlen | Lesestand auf dem Server als Lesemarke je Feed plus Ausnahmen; die Zahl wird immer berechnet, nie mitgezählt. |
| A8 | Ordnerstruktur, per Drag & Drop umsortierbar | Ordner und Abos in eigenen Tabellen mit Position, bis zu fünf Ebenen tief, auch per Tastatur. |
| A9 | Filterregeln pro User (Stichwörter, Autoren) | Regeln mit Bereich, Feld, Vergleich und Aktion; sie greifen beim Eintreffen eines Artikels. |
| A10 | Werbung und Tracker aus Artikeln entfernen | Die vorhandenen Regeln ziehen auf den Server, die Listen werden Daten statt Code. |
| A11 | Bilder über den Server leiten (Privacy) | Eigener Bild-Proxy in Rust mit signierten Adressen. |
| A12 | Vorhandenes bleibt: OPML-Import und -Export, Feed-Suche auf Webseiten, Labels mit Farben | Zieht mit um. |
| A13 | Den Betrieb überblicken | Eine Statusseite für Admins: Tempo und Engpässe, Fehler des Servers, Zustand von Abruf, Feed-Suche und Bild-Proxy. Admins werden auf der Kommandozeile bestimmt. Dazu, von Usern, die zustimmen, Fehler aus ihren Browsern. Seit 2026-11-13 überdauern die Zahlen einen Neustart und bleiben so lange, wie der Betreiber einstellt (Paket 5.14). Seit 2026-11-14 gibt es dieselben Zahlen für ein eigenes Prometheus, mit einem Token des Betreibers (Paket 5.25). |
| A14 | Als App nutzbar | rueder lässt sich als App installieren (PWA) und startet ohne Netz aus dem Cache; eine neue Version bietet sich zum Neuladen an. Dazu eine Android-App mit Capacitor, die nach Zustimmung nach Updates sucht. |
| A15 | Umzug zwischen rueder-Instanzen (2026-11-12) | Ein Konto geht als eine Datei hinaus und in einen anderen rueder hinein: Einstellungen, Ordner und Feeds, Lesestand, Sterne, Labels und Filterregeln. Feeds werden an ihrer Adresse und Artikel an ihrer Kennung wiedererkannt, nicht an ihren Nummern, denn die sind auf jeder Instanz andere. |
| A16 | Öffentliche Instanz neben selbst gehosteten (2026-11-12) | Alles, was eine öffentliche Instanz braucht, ist eine Einstellung und aus als Standard. Eine selbst gehostete Instanz bleibt, wie sie ist. |
| A17 | Konten mit E-Mail | Pflicht auf der öffentlichen Instanz, sonst wählbar oder aus. Dazu Bestätigung, Passwort vergessen und Wechsel der Adresse. |
| A18 | Konto löschen und sperren (2026-11-12) | Ein Konto löscht sich selbst, mit allem, was ihm gehört; Feeds bleiben für die anderen. Der Betreiber sperrt ein Konto bei Missbrauch. |
| A20 | Rechtliches und Meldeweg (2026-11-12) | Impressum, Datenschutz und Nutzungsbedingungen mit Zustimmung; Meldungen von Inhalten; Sperre von Feeds. |
| A21 | Wachstum im Griff (2026-11-12) | Aufbewahrung von Artikeln einstellbar, Abruf einstellbar und gemessen. |
| A22 | Zweiter Faktor (2026-11-12) | TOTP mit Wiederherstellungscodes, Pflicht für Admins der öffentlichen Instanz. Dazu Passkeys (2026-11-13): Login ohne Passwort, der zugleich als zweiter Faktor zählt. |
| A24 | Apps anderer Anbieter (2026-11-13) | Feedreader-Apps wie Reeder, NetNewsWire oder Read You lesen und bedienen ein Konto über die Google-Reader-API, angemeldet mit App-Passwörtern. |

Die Nummern ab A16 und die Kennungen E4, E6, E12, E13 und 5.x stammen aus Fionas eigener Planung für eine öffentliche Instanz; was hier fehlt, steht dort und gehört nicht zu diesem Repository. Die Lücken sind Absicht.

## Ausgangslage

Der alte rueder hatte (Stand Commit `0b36a96`) rund 6.300 Zeilen Go (`backend/`) und 7.400 Zeilen Svelte und TypeScript (`frontend/`). Vieles Gewünschte ist im Ansatz da, aber der Aufwand der Artikelliste wächst mit der Größe des Feeds. Das ist aus dem Code abgeleitet und nicht an Produktionsdaten gemessen; gemessen wurde in Schritt 1 das neue Schema an erzeugten Daten.

| Bereich | Stand heute | Folge |
|---|---|---|
| Artikel | Eine Tabelle `articles`, der Text liegt als JSON in derselben Zeile. Ein bereits bekannter Artikel wird beim Abruf übersprungen. | Es gibt keine Versionen. Geänderte Artikel bleiben auf dem alten Stand. |
| Artikelliste | Jede Listenabfrage nummeriert alle Artikel des Feeds per Fensterfunktion durch (`row_number() over (partition by feed_id order by seq)`) und lädt mit `SELECT *` den vollen Text mit. Dazu kommt ein `COUNT(*)` je Feed bei jedem Abruf. | Wahrscheinlichste Ursache der Langsamkeit. |
| Lesestand | Gelesen-Listen und Labels liegen nur im `localStorage` des Browsers ([user_data.md](user_data.md)). Die Tabelle `user_states` existiert, wird aber nicht benutzt. | Kein Abgleich zwischen Geräten, die Ungelesen-Zahlen driften. |
| Ordner | Ein JSON-Dokument je User (`users.folders`), das Frontend schickt immer die ganze Liste. Eine Ebene, Drag & Drop mit `svelte-dnd-action`. | Gleichzeitige Änderungen überschreiben sich. |
| Live-Updates | Ein SSE-Dienst mit RabbitMQ meldet nur Ordneränderungen. Beide fehlen im Produktions-Compose. | Neue Artikel kommen nicht live an. |
| Bereinigung | Das Backend entfernt Skripte und unsichere Tags. Werbe- und Tracker-Filter laufen im Browser bei jedem Anzeigen (`frontend/src/rueder/helpers/ArticleCleaner.ts`): Tracking-Parameter, 1×1-Pixel, Social-Links, gesperrte Domains. | Die Regeln stehen fest im Code. Die Tests dazu sind als Vorlage wertvoll. |
| Bilder | imgproxy mit Presets. Die URL-Signatur ist im Browser vorgesehen (`VITE_IMGPROXY_KEY`), die Beispielkonfiguration setzt keinen Schlüssel. | In beiden Fällen kann jeder beliebige Adressen durch den Proxy laden. |
| Login | loginsrv (mit htpasswd) stellt ein JWT aus, das Frontend hält es im `localStorage`. | Ein einziger XSS-Fehler reicht, um die Sitzung zu stehlen. |
| Zugriff | `GET /feeds` liefert alle Feeds aller User. Jeder eingeloggte User kann jeden Artikel per ID lesen. | Die Abos anderer User sind sichtbar. |
| Abruf | `gofeed` mit vorgetäuschtem Chrome-User-Agent, Intervall 15 Minuten bis 12 Stunden je nach Aktivität des Feeds. Geprüft wird nur, ob die Adresse mit http oder https beginnt. | Interne Adressen sind abrufbar (SSRF). |
| Filterregeln | Es gibt keine Regeln pro User für Stichwörter oder Autoren. | Neu zu bauen. |
| Sonstiges | OPML-Import und -Export, Feed-Suche auf Webseiten, Labels mit Farben. | Bleibt erhalten und zieht mit um. |

## Was den Plan trägt

- **Schnell trotz wachsender Artikelzahl:** Die Artikelliste liest nur eine schmale Tabelle. Der aktuelle Text liegt getrennt daneben. Alte Versionen liegen komprimiert in einem kalten Archiv, das nach Monaten geteilt ist.
- **Verlässliche Ungelesen-Zahlen:** Der Lesestand zieht vom Browser auf den Server um. Die Zahl wird aus einer Lesemarke je Feed und wenigen Ausnahmen berechnet; es gibt keinen Zähler, der auseinanderlaufen kann.
- **Security von Anfang an:** Sitzungs-Cookies statt Token im Browser-Speicher, ein gehärteter HTTP-Client für jeden Abruf fremder Adressen, Bereinigung und Bild-Proxy auf dem Server.
- **Daten bleiben erhalten:** Feeds, Artikel, User und Ordner werden einmalig aus einem Backup der alten Produktion übernommen, auch nachträglich in einen neuen rueder, der schon läuft. Der Lesestand kommt aus dem Browser.

## Entscheidungen

Von Fiona am 2026-11-10 entschieden.

| Frage | Entscheidung | Wofür es zählt |
|---|---|---|
| Größe der Produktion, Dump zum Messen? | Kein Dump. Ausgelegt und gemessen wird für 10 User, 300 Feeds und 10 Millionen Artikel, an erzeugten Daten. | Schritt 1 misst daran. |
| Gleiches Repository oder ein neues? | Gleiches Repository, neue Verzeichnisse `server/` und `web/`, seit Paket 2.10 dazu `deploy/` für den Betrieb, seit Paket 3.11 `app/` für die Android-App und seit Paket 5.22 `pages/` für die Seite auf GitHub Pages. Der alte Code bleibt bis zum Umstieg als Vorlage liegen. Seine Paketdateien (`backend/go.mod`, `go.sum`, `frontend/package.json`, `pnpm-lock.yaml`), seine Compose-Dateien in `/` und `backend/` und seine Workflows hat Fiona am 2026-11-11 vorzeitig entfernen lassen, damit Dependabot und die Security Advisories nicht mehr die Abhängigkeiten eines Systems melden, das nicht mehr läuft. Seitdem ist der alte Code nicht mehr lauffähig; wer ihn starten will, nimmt den Commit `0b36a96` davor. Am 2026-11-12 hat Fiona den Rest des alten Codes entfernen lassen (Paket 4.7), noch vor Probelauf, Import und Übernahme aus dem Browser (4.4 bis 4.6): Der neue rueder läuft in der Produktion, der Import liest nur die alte Datenbank, und was 4.6 vom alten Frontend wissen muss, steht in [user_data.md](user_data.md) und im Commit `0b36a96`. | Arbeitsweise ab Schritt 1. |
| Login | Heute loginsrv mit htpasswd. Künftig lokale Konten mit Argon2id, OpenID Connect muss möglich sein. | Bestimmt, wie bestehende Konten umziehen. |
| Ordner | Verschachtelt, höchstens 5 Ebenen. | Datenmodell und Drag & Drop. |
| Versionen | Höchstens 20 Versionen je Artikel. Ein aktualisierter Artikel bleibt gelesen und bekommt den Hinweis „aktualisiert“. | Speicherbedarf und Ungelesen-Zahlen. |
| Bild-Proxy | Selbst in Rust: prüfen, cachen, verkleinern mit dem `image`-Crate. Kein imgproxy mehr. Die Adresse des Proxy kommt erst beim Ausliefern in den Text, nicht schon beim Bereinigen. | Ein Dienst weniger, dafür eigener Code an einer Angriffsfläche. |
| User-Agent beim Abruf | Ehrlich als „rueder“, je Feed umstellbar. | Manche Seiten sperren unbekannte Clients. |
| Oberfläche | Der klickbare Entwurf in `web/` ist abgenommen („sieht perfekt aus“). | Schritt 3 baut ihn aus, statt neu zu entwerfen. |
| Umstieg und alte Daten | Kein Parallelbetrieb von altem und neuem rueder. Die alte Produktion ist abgeschaltet, ihre Daten kommen aus einem Backup von vor der Abschaltung. Der neue rueder geht ohne alte Daten in Betrieb, mit neuen Usern und Feeds. Später werden die alten Daten einmalig importiert und mit den Konten zusammengeführt, die es dann gibt, bei Bedarf unter anderem Namen. Mehrfache Importe sind nicht nötig. | Schritt 4 und der Import: Er muss in einen Bestand schreiben können, der schon Konten, Feeds und Artikel hat. |
| Statusseite (2026-11-11) | Eine Seite für Admins mit Tempo und Engpässen, den Warnungen und Fehlern des Servers und dem Zustand von Feed-Suche, Bild-Proxy und Abruf. Admins bestimmt die Kommandozeile (`rueder admin add`, `remove`, `list`). Laufzeitfehler aus den Browsern der User kommen dazu, nur von denen, die zustimmen, und mit einer eigenen, offenen Lösung statt eines fremden Dienstes. Am 2026-11-11 entschieden: aus als Standard, 30 Tage Aufbewahrung, Admins sehen keine Namen von Konten. | Paket 3.8 baut die Seite, Paket 3.9 die Fehlerberichte. |
| Apps (2026-11-12) | Nach dem Vorbild von vibe-wormholesurfing: zuerst eine PWA mit Service Worker und Hinweis auf neue Versionen, dann eine Android-App mit Capacitor, die nach Updates sucht, wenn der User zustimmt, und deren Update-Suche sich für Builds für den Play Store abschalten lässt. Je ein eigener Pull Request. Für die App am 2026-11-12 entschieden: Sie fragt nach der Adresse des eigenen rueder und lädt dann dessen Seite (Server und Härtung bleiben, wie sie sind); Updates kommen aus einem GitHub-Release, der Signaturschlüssel liegt nur in den Secrets der CI; die Update-Logik ist eigener Code statt der Bibliothek des Vorbilds, die nicht aus der npm-Registry kommt. | Pakete 3.10 und 3.11. |
| Fuzzing | Ja, gezielt dort, wo eigener Code über Sicherheit entscheidet: Bereinigung, Feed-Verarbeitung und Bild-Proxy bekommen Fuzz-Targets, die Adressprüfung des HTTP-Clients einen Property-Test; seit Paket 2.15 dazu das Lesen der Anfragen der Google-Reader-API (siehe A24). Kein Fuzz-Target für jeden Parser. | Paket 2.11, lief vor dem Security-Review 2.12; seit 2.15 laufen die Targets auf GitHub. |
| Registrierung (entschieden am 2026-11-12) | User können sich selbst registrieren, wenn der Betreiber es einschaltet; ohne sein Zutun bleibt alles zu. Er begrenzt die Gesamtzahl der Konten und die Registrierungen je Tag. Bei geschlossener Registrierung laden User mit einem Kontingent ein, das die Kommandozeile setzt; ihre Einladungen halten dieselben Grenzen ein. Admins können immer Einladungslinks erzeugen, deren Konten gegen ein eigenes Kontingent zählen. Links gelten eine einstellbare Zeit und für ein oder mehrere Konten. Alle Grenzen stehen in der Datenbank und werden per Kommandozeile gesetzt. Das ersetzt „Konten legt nur der Betreiber an“. | Login-Seite, Einstellungen, Kommandozeile; neue Angriffsfläche für das Security-Review. |
| Weitere Ansichten (2026-11-12) | Neben Liste und erweiterter Ansicht drei Bereiche (Baum, Liste, Artikel) und ein Magazin mit großem Aufmacherbild und Kachelraster, vorgezogen aus „Nach dem Umstieg“. Je ein eigener Pull Request. | Pakete 3.13 und 3.14. |
| Umzug eines Kontos (2026-11-12) | Import und Export der Daten eines Users mit Einstellungen und Lesestand, mit Feed- und Artikel-ID und -Adresse; weil die IDs auf einer anderen rueder-Instanz andere sind, erkennt der Import Feeds an der Adresse und Artikel an Kennung und Adresse. Der Plan dazu (ein Import, der nur hinzufügt; der Lesestand eines Feeds, den das Ziel noch nie abgerufen hat, wartet auf den ersten Abruf) ist freigegeben. Geändert am 2026-11-12 nach dem Security-Review: Texte von Artikeln kommen nicht mehr mit. Vorher kamen die mit Stern oder Label mit und standen dann im geteilten Feed, so dass jede selbst geschriebene Datei erfundene Artikel in Feeds anderer stellen konnte; Fiona hat das Weglassen den Kopien nur für das eigene Konto vorgezogen. Was ein Artikel, den das Ziel nicht hat, an Lesestand, Stern und Labels trug, geht verloren. | Paket 3.12, [Umzug eines Kontos](architecture.md#umzug-eines-kontos). |
| E4 Aufbewahrung (2026-11-12) | Einstellbar, Standard bleibt alles, wie bisher. Vorgesehen für eine öffentliche Instanz: Texte 24 Monate, Versionen 12 Monate, Artikel mit Stern oder Label bleiben, verwaiste Feeds gehen nach 90 Tagen. Das ersetzt die Detailfrage „Werden Artikel irgendwann gelöscht? Nein.“ Am 2026-11-13 genauer entschieden: Was nach den Monaten geht, wählt der Betreiber je Instanz, den ganzen Artikel oder nur seinen Text, und er nennt es ausdrücklich, weil das eine wie das andere unerwartet sein kann; das Alter zählt ab der Ankunft in rueder, nicht nach dem Datum, das der Feed nennt; gesperrte Feeds bekommen keine Sonderregel. | Paket 5.3, [Aufbewahrung](architecture.md#aufbewahrung). |
| E6 Versand von E-Mails (2026-11-14) | SMTP mit `lettre` (rustls) an ein Relais, das der Betreiber einstellt. Das ist eine begründete Ausnahme von „nur der gehärtete HTTP-Client geht ins Internet“, weil die Adresse nie ein User wählt. Die Alternative ist die HTTP-API eines Anbieters über den gehärteten Client. Die Texte der Mails liegen als Daten in `server/templates/mail/`, eine zweite Ausnahme von „alle Texte in `web/src/lib/i18n/`“, wie bei der App. | Paket 5.4, [E-Mail am Konto](architecture.md#e-mail-am-konto). |
| E12 Meldungen (2026-11-12) | Admins sehen bei gemeldeten Feeds die Adresse, nie, wer gemeldet hat. Das ist eine Ausnahme davon, dass die Statusseite keine Feeds nennt. | Paket 5.9. |
| E13 Inaktive Konten (2026-11-12) | Konten ohne Login seit 12 Monaten werden optional nach zwei Mails gelöscht. | Ein eigenes Paket E13 in Schritt 5. Bei Paket 5.3 zurückgestellt, weil rueder damals keine Mails schickte; es baut auf Paket 5.4 auf. |
| Passkeys (2026-11-13) | Ein Passkey ist beides: ein Login ohne Name und Passwort und ein zweiter Faktor, weil das Gerät PIN oder Biometrie verlangt. Er zählt auch für Admins mit `ADMIN_SECOND_FACTOR=required` und ersetzt Passwort und Code überall, wo eine Änderung beides verlangt. Hat ein Konto einen Passkey, ist der zweite Faktor an: Das Passwort allein öffnet es nicht mehr, nach ihm braucht es den Passkey, einen Code der App oder einen Wiederherstellungscode, und die zehn Wiederherstellungscodes kommen mit dem ersten Passkey, wenn es noch keine gibt. Geprüft wird mit `webauthn-rs` 0.6, das seit dieser Version reines Rust auf RustCrypto ist; dafür eine begründete Ausnahme für RUSTSEC-2023-0071 in `rsa`, weil rueder nur Signaturen mit öffentlichen Schlüsseln prüft. Verworfen ist eine eigene schmale Prüfung: weniger Abhängigkeiten, aber eigener Code an der empfindlichsten Stelle. Passkeys gibt es nur, wo der Betreiber `PUBLIC_URL` setzt. | Paket 5.15, [Passkeys](architecture.md#passkeys). |
| Passkeys in der Android-App (2026-11-13) | Die App bleibt eine WebView und schaltet dort WebAuthn für sich ein; der Server nennt die App in `/.well-known/assetlinks.json` und nimmt ihre Herkunft `android:apk-key-hash:…` an. Jeder rueder-Server erlaubt das von sich aus der App von GitHub; der Betreiber ändert die Liste mit `PASSKEY_ANDROID_APPS` oder leert sie mit `none`. Verworfen sind eine Trusted Web Activity (braucht genauso `assetlinks.json` je Server und ersetzt die ganze App) und ein Login über einen Custom Tab mit Übergabe der Sitzung (mehr eigener Code an der empfindlichsten Stelle, und Passkeys hinzufügen ginge in der App trotzdem nicht). | Paket 5.17, [Passkeys](architecture.md#passkeys). |
| A24 Apps anderer Anbieter (2026-11-13) | rueder spricht die Google-Reader-API, wie FreshRSS, Miniflux und Inoreader; Fever und eine eigene API für Apps sind verworfen. Apps melden sich nur mit App-Passwörtern an, nie mit dem Passwort des Kontos: Jedes entsteht in den Einstellungen mit Passwort und, wo eingerichtet, dem Code des zweiten Faktors, wird einmal gezeigt, nur als Hash gespeichert und lässt sich einzeln widerrufen. Bilder gehen auch an Apps über den Bild-Proxy, mit Adressen, die für das App-Passwort signiert sind und ohne Cookie gelten; mit dem Widerruf enden sie. Verschachtelte Ordner erscheinen als Pfad („News/Tech“), weil die Google-Reader-API nur eine Ebene kennt. Apps dürfen alles ändern, was die Seite ändert: Lesestand, Sterne, Labels, alles als gelesen, Abos. Der Reiter „Apps“ der Einstellungen zeigt die Apps, die mit rueder gehen, mit Links zu ihnen, und führt durch die Einrichtung, so dass sie ohne Anleitung von außen gelingt. Eingeschaltet als Standard, weil es auch für selbst gehostete Instanzen zählt; `APP_API=off` schaltet es ab. Das ist bewusst anders als bei A16, wo alles für die öffentliche Instanz aus als Standard ist. Das Lesen der Anfragen bekommt ein Fuzz-Target; die Fuzz-Targets laufen auf GitHub, nicht in der Entwicklungsumgebung. Was eine App an rueder vorbei selbst holt, wie NetNewsWire die Symbole der Feeds von ihren Websites, umgeht rueder nicht mit Umwegen; die Einrichtung sagt es kurz und sachlich, mit dem, was es für die Privatsphäre heißt (entschieden am 2026-11-13). | Paket 2.15, [Apps anderer Anbieter](architecture.md#apps-anderer-anbieter). |
| Seite auf GitHub Pages (2026-11-13) | Unter `fionapreroll.github.io/rueder/` steht eine Übersicht, unter `demo/` der Leser mit erfundenen Daten, unter `docs/rest-api/` die REST-API wie bei Swagger (die Google-Reader-API als eigene Gruppe), unter `docs/rust/` die Doku des Rust-Codes, unter `docs/android-app/` die der App und unter `docs/frontend/` die des Frontends. Alles nur auf Englisch. Dazu, von Fiona freigegeben: `docs/self-hosting/` aus `deploy/README.md`; `docs/` selbst führt zur Übersicht; jede Seite nennt Commit und Tag, aus denen sie gebaut ist; jeder Pull Request baut die Seite, Warnungen der Werkzeuge und Links ins Leere lassen ihn scheitern, veröffentlicht wird nur von `main`. Die REST-API wird mit Swagger UI gezeigt, ohne „Try it out“; die Beschreibung (OpenAPI) entsteht aus dem Code mit `utoipa`, nicht von Hand, und liegt als `openapi.json` im Repository und auf der Seite. Die Demo folgt wie das Produkt der Sprache des Browsers; ihre erfundenen Daten gibt es seit 2026-11-14 auch auf Englisch, und sie folgen derselben Sprache. Die Rust-Doku wird zuerst nur aufgesetzt, die Arbeit an der Doku aller Routen kommt in einem zweiten Pull Request. Seit 2026-11-14 zeigt die README einige Bildschirmfotos der besten Seiten von rueder; sie entstehen in der CI und dürfen die Commits nicht füllen, deshalb liegen sie auf der Seite unter `screenshots/` und nicht im Repository. | Pakete 5.22, 5.23 und 5.26. |

Technische Festlegungen (Stack, Werkzeuge, CI) stehen in [implementation.md](implementation.md#stack-und-konventionen).

## Plan in fünf Schritten

Jeder Schritt endet mit etwas, das sich prüfen lässt. Der neue Code entsteht im selben Repository in `server/` und `web/` (dazu `app/` für die Android-App, `deploy/` für den Betrieb und `pages/` für die Seite auf GitHub Pages), der alte lag bis zum Umstieg als Vorlage daneben und ist seit dem 2026-11-12 entfernt. Den Stand je Schritt führt [implementation.md](implementation.md#stand).

**Vorarbeit: klickbarer Entwurf der Oberfläche** (fertig). Die Oberfläche wurde vor dem Server entworfen, damit Aussehen und Bedienung früh feststehen. Der Entwurf liegt in `web/`, läuft mit erfundenen Feeds ganz im Browser und ist abgenommen.

1. **Fundament und Daten** (fertig). Rust-Grundgerüst, neues Schema, Importprogramm für die alte Datenbank. Die Abfragen für Liste, Artikel und Ungelesen-Zahlen werden an erzeugten Daten in Produktionsgröße gemessen.

   Ergebnis: ein Importprogramm, das an einer nachgebauten alten Datenbank geprüft ist, und Messwerte, die die Speicherstrategie bestätigen oder korrigieren.

2. **Backend** (fertig). Abruf mit Versionen, Bereinigung mit Filterlisten, Bild-Proxy, Login und Sitzungen, API für Lesen, Lesestand, Ordner und Filterregeln, Live-Updates. Am Ende Fuzzing und das Security-Review.

   Ergebnis: Der neue Server läuft gegen echte Feeds. Die API-Tests und die übernommenen Cleaner-Tests sind grün.

3. **Frontend** (fertig). Der Entwurf wird an die API angeschlossen und ausgebaut: Listen- und Volltextansicht, Tastatur, Ordnerbaum mit Drag & Drop, Live-Updates, Einstellungen, Filterregeln, Versionsvergleich, Labels, OPML. Dazugekommen sind die Statusseite für Admins mit Fehlerberichten aus den Browsern (A13), PWA und Android-App (A14) und der Umzug eines Kontos (A15).

   Ergebnis: rueder ist täglich benutzbar.

4. **Umstieg** (begonnen: in Betrieb, alter Code entfernt; Probelauf, Import und Übernahme aus dem Browser offen). Der neue rueder geht ohne alte Daten in Betrieb. Danach werden die alten Daten einmalig aus dem Backup importiert und mit den neuen Konten und Feeds zusammengeführt, erst zur Probe auf einer Kopie, dann in der Produktion. Lesestand und Labels kommen aus dem Browser nach. Das alte System sollte am Ende abgebaut werden; auf Fionas Wunsch geschah das schon am 2026-11-12, nach der Inbetriebnahme und vor dem Import (siehe [Entscheidungen](#entscheidungen)).

   Ergebnis: Die Produktion läuft auf dem neuen rueder mit den alten Daten, der alte Code ist entfernt.

5. **Öffentliche Instanz** (dazugekommen am 2026-11-12, zum größten Teil gebaut; offen sind 5.5, 5.11 und E13). Was ein rueder braucht, der für alle offen ist (A16): ein einstellbarer Abruf für viele Feeds, eine einstellbare Aufbewahrung, Konten, die sich löschen und sperren lassen, ein zweiter Faktor, die Rechtstexte mit Zustimmung und ein Meldeweg mit Sperre von Feeds. Jedes davon ist eine Einstellung und aus als Standard; eine selbst gehostete Instanz merkt nichts davon.

   Ergebnis: Eine öffentliche Instanz lässt sich mit rueder betreiben, ohne dass sich für eine selbst gehostete etwas ändert.

Ein Security-Review gegen die Tabelle in [architecture.md](architecture.md#security) stand am Ende von Schritt 2 (Paket 2.12), davor wurde [gefuzzt](architecture.md#fuzzing). Ein zweites war vor der Inbetriebnahme vorgesehen (Paket 4.1); Fiona hat es am 2026-11-12 übersprungen, weil 2.12 gerade erst fertig geworden war, mit einem [Nachtrag](architecture.md#nachtrag-app-service-worker-ci-und-deploy) für App, CI und Deploy. Seit Paket 2.15 laufen die Fuzz-Targets jede Woche auf GitHub.

## Messziel

Liste, Artikel und Ungelesen-Zahlen brauchen an der Datenbank jeweils unter 50 ms. Gemessen wird an erzeugten Daten in der Größe der Produktion (10 User, 300 Feeds, 10 Millionen Artikel) und noch einmal mit zehnfacher Artikelzahl, um zu sehen, ob die Zeiten von der Gesamtgröße unabhängig bleiben. Verfehlt eine Abfrage das Ziel, wird das Schema in Schritt 1 angepasst, bevor darauf gebaut wird. Methode und Ergebnisse: [implementation.md](implementation.md#messung).

Ergebnis vom 2026-11-10: Das Ziel ist erreicht. Mit 10 Millionen Artikeln brauchen die Abfragen höchstens 11 ms im Median und 18 ms im 99. Perzentil, direkt nach einem Neustart höchstens 34 ms. Dafür wurde die Listenabfrage umgebaut, das Schema blieb. Abweichung vom Plan: 100 Millionen Artikel passten nicht auf die Platte der Messmaschine. Verglichen wurde stattdessen mit 1 Million Artikeln, mit gleichen Zeiten; die Messung mit mehr Daten folgt beim Probelauf auf einer Kopie der Produktion.

## Nach dem Umstieg

Bewusst zurückgestellt, damit der Umstieg nicht daran hängt:

- „Hot“: Themen, auf die mehrere Feeds verlinken (Vorbild Fever).

Vorgezogen hat Fiona am 2026-11-12 die beiden Ansichten, die hier standen: die Aufteilung in drei Bereiche (Baum, Liste, Artikel) und eine Magazin-Ansicht mit großem Aufmacherbild und Kachelraster (Pakete 3.13 und 3.14).

## Risiken

| Risiko | Gegenmaßnahme |
|---|---|
| Die Messung an erzeugten Daten trifft die Produktion nicht (andere Verteilung der Feedgrößen und Textlängen). | Der Generator bildet schiefe Verteilungen nach (wenige sehr große Feeds, viele kleine). Bevor die alten Daten in die Produktion kommen, läuft der Import einmal zur Probe auf einer Kopie, mit denselben Messungen. |
| Der Lesestand liegt nur in den Browsern der User und geht beim Umstieg verloren. | Das neue Frontend liest nach dem Import den alten `localStorage` und schickt ihn an den Server (Schritt 4). Dafür muss das neue Frontend unter derselben Adresse (Origin) laufen wie das alte, und der Browser muss den Speicher noch haben. Je später der Import, desto eher fehlt er; dann gilt alles Alte als gelesen. |
| Eigener Bild-Proxy und eigener HTTP-Client sind Angriffsflächen. | Je eine zentrale, getestete Komponente, feste Grenzen, Fuzzing jede Woche, ein Security-Review mit Nachtrag. |
| Scheinversionen: Feeds ändern bei jedem Abruf Kleinigkeiten (Werbe-Token, Zeitstempel) und füllen das Archiv. | Prüfsumme über den normalisierten Inhalt, Obergrenze von 20 Versionen je Artikel. |
| Seiten sperren den ehrlichen User-Agent. | Je Feed umstellbar; Abruffehler sind in der Oberfläche sichtbar. |
| Der einmalige Import schreibt in eine Datenbank, die schon benutzt wird, und lässt sich nicht einfach wiederholen. | Der Server steht währenddessen. Davor ein Backup der neuen Datenbank, ein Lauf, der nur zeigt, was er täte, und ein Probelauf auf einer Kopie. Nach einem Abbruch lässt sich der Import fortsetzen. |

## Detailfragen

Fiona hat die Vorschläge am 2026-11-10 angenommen. Gebaut wird danach.

| Frage | Entscheidung |
|---|---|
| Was geschieht bei der 21. Version eines Artikels? | Die Erstfassung und die 19 neuesten bleiben, die zweitälteste fällt weg. |
| Wie viele Artikel sind bei einem neuen Abo ungelesen? | Die 10 neuesten, alles davor gilt als gelesen. Heute ist alles ungelesen. |
| Wer darf den User-Agent eines Feeds umstellen? Ein Feed gehört allen seinen Abonnenten gemeinsam. | Jeder Abonnent; die Einstellung gilt für den Feed. |
| Wie ziehen die htpasswd-Konten um? | Die Hashes aus htpasswd werden nicht übernommen. Geändert am 2026-11-10: Weil der neue rueder vor dem Import in Betrieb geht, gibt es die Konten dann meist schon. Der Import führt einen alten User mit dem Konto gleichen Namens zusammen oder mit dem Konto, das ihm beim Aufruf zugeordnet wird. Nur für alte User ohne Gegenstück legt er ein Konto ohne Passwort an; ein Kommandozeilenbefehl setzt ein Passwort oder erzeugt einen Einmal-Link. |
| Werden Artikel irgendwann gelöscht? | Nur, wenn der Betreiber es einstellt ([E4](#entscheidungen), gebaut mit Paket 5.3): `rueder retention set` lässt Artikel nach so vielen Monaten seit ihrer Ankunft gehen, ganz oder nur ihren Text, Versionen nach so vielen Monaten und Feeds ohne Abonnenten nach so vielen Tagen. Artikel mit Stern oder Label bleiben. Als Standard bleibt alles, wie bisher. Geräumt wird, wie hier zuerst vermutet, am unteren Ende eines Feeds. |
| Volltextsuche über alle Artikel? | Der Entwurf durchsucht die gezeigte Liste. Eine Suche auf dem Server über die eigenen Abos (PostgreSQL-Volltextsuche) kommt in Schritt 2 dazu, wenn die Messung zeigt, dass der Index tragbar ist. Gemessen und gebaut am 2026-11-11 mit Paket 2.13, siehe [Suche](architecture.md#suche) und [Messung](implementation.md#suche-vom-2026-11-11). Ohne Stammformen, dafür mit Wortanfängen, und ohne Rangfolge; die Listen der markierten und ausgeblendeten Artikel und der Labels durchsucht weiter das Frontend. |
