-- Fehlerberichte aus den Browsern der User (Paket 3.9).
--
-- Nur wer zustimmt, schickt Berichte; die Zustimmung steht beim Konto und ist
-- aus, bis der User sie einschaltet. Ein Bericht ist an sein Konto gebunden,
-- damit Ausschalten und das Loeschen des Kontos ihn mitnehmen. Admins sehen auf
-- der Statusseite, wie viele Konten ein Fehler trifft, nie welche. Berichte
-- bleiben 30 Tage nach ihrem letzten Auftreten.

ALTER TABLE accounts ADD COLUMN sends_reports boolean NOT NULL DEFAULT false;

-- Gleiche Fehler zaehlen in einer Zeile: je Konto, Fingerabdruck, Version des
-- Frontends und Browser.
CREATE TABLE client_reports (
    id          bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id  integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    -- SHA-256 ueber Art, Meldung und ersten eigenen Eintrag des Stacks.
    fingerprint bytea       NOT NULL,
    kind        text        NOT NULL
                            CHECK (kind IN ('error', 'rejection', 'component', 'request')),
    message     text        NOT NULL CHECK (length(message) <= 500),
    -- Nur Dateien der eigenen Bundles mit Zeile und Spalte, je Zeile eine.
    stack       text        NOT NULL DEFAULT '' CHECK (length(stack) <= 2000),
    version     text        NOT NULL CHECK (length(version) <= 40),
    browser     text        NOT NULL CHECK (length(browser) <= 40),
    count       integer     NOT NULL DEFAULT 1 CHECK (count > 0),
    first_at    timestamptz NOT NULL DEFAULT now(),
    last_at     timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, fingerprint, version, browser)
);

-- Fuer das Aufraeumen nach 30 Tagen und die neuesten auf der Statusseite.
CREATE INDEX client_reports_last ON client_reports (last_at);
