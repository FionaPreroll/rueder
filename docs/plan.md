# rueder: Plan für den Rewrite in Rust

Stand 2026-11-11. Dieses Dokument sagt, was gebaut wird, warum, in welcher Reihenfolge und was entschieden ist. Wie das neue System aufgebaut ist, steht in [architecture.md](architecture.md). Wie gearbeitet wird, was fertig ist und welche Arbeitspakete offen sind, steht in [implementation.md](implementation.md).

- [Ziel und Anlass](#ziel-und-anlass)
- [Anforderungen](#anforderungen)
- [Ausgangslage](#ausgangslage)
- [Was den Plan trägt](#was-den-plan-trägt)
- [Entscheidungen](#entscheidungen)
- [Plan in vier Schritten](#plan-in-vier-schritten)
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
| A4 | Schnelle, bequeme, intuitive UX im Stil des Google Reader, modernisiert | Dichte Listenansicht, Volltextansicht, Tastaturbedienung, Dark Mode, Ansicht fürs Handy. |
| A5 | Anpassbar | Thema, Akzentfarbe, Dichte, Schrift, Ansicht und Sortierung, Tastaturbelegung; gespeichert auf dem Server. |
| A6 | Live-Updates von Feeds | Eine SSE-Verbindung meldet neue Artikel, Lesestand anderer Geräte, Ordneränderungen und Abruffehler. |
| A7 | Verlässliche Ungelesen-Zahlen | Lesestand auf dem Server als Lesemarke je Feed plus Ausnahmen; die Zahl wird immer berechnet, nie mitgezählt. |
| A8 | Ordnerstruktur, per Drag & Drop umsortierbar | Ordner und Abos in eigenen Tabellen mit Position, bis zu fünf Ebenen tief, auch per Tastatur. |
| A9 | Filterregeln pro User (Stichwörter, Autoren) | Regeln mit Bereich, Feld, Vergleich und Aktion; sie greifen beim Eintreffen eines Artikels. |
| A10 | Werbung und Tracker aus Artikeln entfernen | Die vorhandenen Regeln ziehen auf den Server, die Listen werden Daten statt Code. |
| A11 | Bilder über den Server leiten (Privacy) | Eigener Bild-Proxy in Rust mit signierten Adressen. |
| A12 | Vorhandenes bleibt: OPML-Import und -Export, Feed-Suche auf Webseiten, Labels mit Farben | Zieht mit um. |
| A13 | Den Betrieb überblicken | Eine Statusseite für Admins: Tempo und Engpässe, Fehler des Servers, Zustand von Abruf, Feed-Suche und Bild-Proxy. Admins werden auf der Kommandozeile bestimmt. Später, für User, die zustimmen: Fehler aus ihren Browsern. |

## Ausgangslage

Der bestehende rueder hat rund 6.300 Zeilen Go (`backend/`) und 7.400 Zeilen Svelte und TypeScript (`frontend/`). Vieles Gewünschte ist im Ansatz da, aber der Aufwand der Artikelliste wächst mit der Größe des Feeds. Das ist aus dem Code abgeleitet und nicht an Produktionsdaten gemessen; gemessen wurde in Schritt 1 das neue Schema an erzeugten Daten.

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
| Gleiches Repository oder ein neues? | Gleiches Repository, neue Verzeichnisse `server/` und `web/`. Der alte Code bleibt bis zum Umstieg lauffähig. | Arbeitsweise ab Schritt 1. |
| Login | Heute loginsrv mit htpasswd. Künftig lokale Konten mit Argon2id, OpenID Connect muss möglich sein. | Bestimmt, wie bestehende Konten umziehen. |
| Ordner | Verschachtelt, höchstens 5 Ebenen. | Datenmodell und Drag & Drop. |
| Versionen | Höchstens 20 Versionen je Artikel. Ein aktualisierter Artikel bleibt gelesen und bekommt den Hinweis „aktualisiert“. | Speicherbedarf und Ungelesen-Zahlen. |
| Bild-Proxy | Selbst in Rust: prüfen, cachen, verkleinern mit dem `image`-Crate. Kein imgproxy mehr. Die Adresse des Proxy kommt erst beim Ausliefern in den Text, nicht schon beim Bereinigen. | Ein Dienst weniger, dafür eigener Code an einer Angriffsfläche. |
| User-Agent beim Abruf | Ehrlich als „rueder“, je Feed umstellbar. | Manche Seiten sperren unbekannte Clients. |
| Oberfläche | Der klickbare Entwurf in `web/` ist abgenommen („sieht perfekt aus“). | Schritt 3 baut ihn aus, statt neu zu entwerfen. |
| Umstieg und alte Daten | Kein Parallelbetrieb von altem und neuem rueder. Die alte Produktion ist abgeschaltet, ihre Daten kommen aus einem Backup von vor der Abschaltung. Der neue rueder geht ohne alte Daten in Betrieb, mit neuen Usern und Feeds. Später werden die alten Daten einmalig importiert und mit den Konten zusammengeführt, die es dann gibt, bei Bedarf unter anderem Namen. Mehrfache Importe sind nicht nötig. | Schritt 4 und der Import: Er muss in einen Bestand schreiben können, der schon Konten, Feeds und Artikel hat. |
| Statusseite (2026-11-11) | Eine Seite für Admins mit Tempo und Engpässen, den Warnungen und Fehlern des Servers und dem Zustand von Feed-Suche, Bild-Proxy und Abruf. Admins bestimmt die Kommandozeile (`rueder admin add`, `remove`, `list`). Laufzeitfehler aus den Browsern der User kommen später, nur von denen, die zustimmen, und mit einer eigenen, offenen Lösung statt eines fremden Dienstes; bis dahin ist das nur geplant. | Paket 3.8 baut die Seite, Paket 3.9 plant die Fehlerberichte. |
| Fuzzing | Ja, gezielt dort, wo eigener Code über Sicherheit entscheidet: Bereinigung, Feed-Verarbeitung und Bild-Proxy bekommen Fuzz-Targets, die Adressprüfung des HTTP-Clients einen Property-Test. Kein Fuzz-Target für jeden Parser. | Arbeitspaket in Schritt 2, läuft vor beiden Security-Reviews. |

Technische Festlegungen (Stack, Werkzeuge, CI) stehen in [implementation.md](implementation.md#stack-und-konventionen).

## Plan in vier Schritten

Jeder Schritt endet mit etwas, das sich prüfen lässt. Der neue Code entsteht im selben Repository in `server/` und `web/`, der alte bleibt bis zum Umstieg lauffähig. Den Stand je Schritt führt [implementation.md](implementation.md#stand).

**Vorarbeit: klickbarer Entwurf der Oberfläche** (fertig). Die Oberfläche wurde vor dem Server entworfen, damit Aussehen und Bedienung früh feststehen. Der Entwurf liegt in `web/`, läuft mit erfundenen Feeds ganz im Browser und ist abgenommen.

1. **Fundament und Daten** (fertig). Rust-Grundgerüst, neues Schema, Importprogramm für die alte Datenbank. Die Abfragen für Liste, Artikel und Ungelesen-Zahlen werden an erzeugten Daten in Produktionsgröße gemessen.

   Ergebnis: ein Importprogramm, das an einer nachgebauten alten Datenbank geprüft ist, und Messwerte, die die Speicherstrategie bestätigen oder korrigieren.

2. **Backend.** Abruf mit Versionen, Bereinigung mit Filterlisten, Bild-Proxy, Login und Sitzungen, API für Lesen, Lesestand, Ordner und Filterregeln, Live-Updates. Am Ende Fuzzing und das Security-Review.

   Ergebnis: Der neue Server läuft gegen echte Feeds. Die API-Tests und die übernommenen Cleaner-Tests sind grün.

3. **Frontend.** Der Entwurf wird an die API angeschlossen und ausgebaut: Listen- und Volltextansicht, Tastatur, Ordnerbaum mit Drag & Drop, Live-Updates, Einstellungen, Filterregeln, Versionsvergleich, Labels, OPML.

   Ergebnis: rueder ist täglich benutzbar.

4. **Umstieg.** Der neue rueder geht ohne alte Daten in Betrieb. Danach werden die alten Daten einmalig aus dem Backup importiert und mit den neuen Konten und Feeds zusammengeführt, erst zur Probe auf einer Kopie, dann in der Produktion. Lesestand und Labels kommen aus dem Browser nach, am Ende wird das alte System abgebaut.

   Ergebnis: Die Produktion läuft auf dem neuen rueder mit den alten Daten, der alte Code ist entfernt.

Ein Security-Review gegen die Tabelle in [architecture.md](architecture.md#security) steht am Ende von Schritt 2 und noch einmal vor der Inbetriebnahme. Vor beiden wird [gefuzzt](architecture.md#fuzzing).

## Messziel

Liste, Artikel und Ungelesen-Zahlen brauchen an der Datenbank jeweils unter 50 ms. Gemessen wird an erzeugten Daten in der Größe der Produktion (10 User, 300 Feeds, 10 Millionen Artikel) und noch einmal mit zehnfacher Artikelzahl, um zu sehen, ob die Zeiten von der Gesamtgröße unabhängig bleiben. Verfehlt eine Abfrage das Ziel, wird das Schema in Schritt 1 angepasst, bevor darauf gebaut wird. Methode und Ergebnisse: [implementation.md](implementation.md#messung).

Ergebnis vom 2026-11-10: Das Ziel ist erreicht. Mit 10 Millionen Artikeln brauchen die Abfragen höchstens 11 ms im Median und 18 ms im 99. Perzentil, direkt nach einem Neustart höchstens 34 ms. Dafür wurde die Listenabfrage umgebaut, das Schema blieb. Abweichung vom Plan: 100 Millionen Artikel passten nicht auf die Platte der Messmaschine. Verglichen wurde stattdessen mit 1 Million Artikeln, mit gleichen Zeiten; die Messung mit mehr Daten folgt beim Probelauf auf einer Kopie der Produktion.

## Nach dem Umstieg

Bewusst zurückgestellt, damit der Umstieg nicht daran hängt:

- „Hot“: Themen, auf die mehrere Feeds verlinken (Vorbild Fever).
- Magazin-Ansicht mit großem Aufmacherbild und Kachelraster.
- Aufteilung in drei Bereiche (Baum, Liste, Artikel) als weitere Ansicht.

## Risiken

| Risiko | Gegenmaßnahme |
|---|---|
| Die Messung an erzeugten Daten trifft die Produktion nicht (andere Verteilung der Feedgrößen und Textlängen). | Der Generator bildet schiefe Verteilungen nach (wenige sehr große Feeds, viele kleine). Bevor die alten Daten in die Produktion kommen, läuft der Import einmal zur Probe auf einer Kopie, mit denselben Messungen. |
| Der Lesestand liegt nur in den Browsern der User und geht beim Umstieg verloren. | Das neue Frontend liest nach dem Import den alten `localStorage` und schickt ihn an den Server (Schritt 4). Dafür muss das neue Frontend unter derselben Adresse (Origin) laufen wie das alte, und der Browser muss den Speicher noch haben. Je später der Import, desto eher fehlt er; dann gilt alles Alte als gelesen. |
| Eigener Bild-Proxy und eigener HTTP-Client sind Angriffsflächen. | Je eine zentrale, getestete Komponente, feste Grenzen, Fuzzing, zwei Security-Reviews. |
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
| Werden Artikel irgendwann gelöscht? | Nein. Alte Artikel kosten nur Plattenplatz. Eine Aufbewahrungsgrenze ließe sich später am unteren Ende eines Feeds ergänzen. |
| Volltextsuche über alle Artikel? | Der Entwurf durchsucht die gezeigte Liste. Eine Suche auf dem Server über die eigenen Abos (PostgreSQL-Volltextsuche) kommt in Schritt 2 dazu, wenn die Messung zeigt, dass der Index tragbar ist. Gemessen und gebaut am 2026-11-11 mit Paket 2.13, siehe [Suche](architecture.md#suche) und [Messung](implementation.md#suche). Ohne Stammformen, dafür mit Wortanfängen, und ohne Rangfolge; die Listen der markierten und ausgeblendeten Artikel und der Labels durchsucht weiter das Frontend. |
