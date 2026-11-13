-- Der Status fuer Admins ueberdauert einen Neustart (server/src/history.rs).
--
-- Bisher hielt der Server, was er ueber sich zaehlt, nur im Speicher und nur
-- fuer die letzte Stunde. Jetzt schreibt er seine Abschnitte zu zehn Minuten
-- jede Minute und beim Beenden hierher, liest die letzte Stunde beim Start
-- wieder ein und kann auf der Statusseite auch ueber laengere Zeit zaehlen.
-- Wie lange das bleibt, sagt retention.status_days.

-- Was wie lange aufbewahrt wird, gesetzt mit `rueder retention set`. Es gibt
-- genau eine Zeile; Paket 5.3 fuegt die Aufbewahrung der Artikel hinzu.
CREATE TABLE retention (
    single      boolean PRIMARY KEY DEFAULT true CHECK (single),
    -- Wie viele Tage die Zahlen des Status und die Probleme aus dem Log
    -- bleiben.
    status_days integer NOT NULL DEFAULT 7 CHECK (status_days BETWEEN 1 AND 3650)
);

INSERT INTO retention DEFAULT VALUES;

-- Ein Abschnitt von zehn Minuten einer Art von Zahlen: Anfragen je Route,
-- Abrufe, Feed-Suchen, Bilder des Proxy, Warnungen und Fehler des Logs. Die
-- Zahlen stehen als JSON, wie sie der Server zaehlt (monitor.rs); ein Name
-- eines Kontos, eine Adresse oder ein Feed steht nie darin, die Route ist
-- Methode und Muster, etwa "GET /api/feeds/{id}/articles".
CREATE TABLE status_slots (
    starts_at timestamptz NOT NULL,
    kind      text        NOT NULL
        CHECK (kind IN ('requests', 'fetches', 'lookups', 'images', 'problems')),
    -- Die Route bei kind = 'requests', sonst leer.
    route     text        NOT NULL DEFAULT '' CHECK (length(route) <= 300),
    counts    jsonb       NOT NULL,
    PRIMARY KEY (starts_at, kind, route)
);

-- Die Warnungen und Fehler des Logs, wie sie die Statusseite listet: hoechstens
-- 100, die neueste mit position 0. Der Server schreibt die Liste ganz neu,
-- wenn sie sich geaendert hat. Die Texte schrieb der Server selbst; wer ihn
-- betreibt, liest sie ohnehin in seinem Log.
CREATE TABLE status_problems (
    position integer     PRIMARY KEY CHECK (position >= 0),
    error    boolean     NOT NULL,
    target   text        NOT NULL,
    text     text        NOT NULL,
    count    bigint      NOT NULL CHECK (count > 0),
    first_at timestamptz NOT NULL,
    last_at  timestamptz NOT NULL
);
