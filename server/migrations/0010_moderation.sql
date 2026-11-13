-- Meldeweg und Sperre von Feeds (Paket 5.9, server/src/moderation.rs).

-- Ein gesperrter Feed wird nicht mehr abgerufen, seine Artikel sieht niemand,
-- und niemand abonniert ihn neu. Gesetzt und aufgehoben wird das nur auf der
-- Kommandozeile (rueder feed block, unblock).
ALTER TABLE feeds ADD COLUMN blocked_at timestamptz;

-- Meldungen von Feeds und Artikeln durch ihre Leser. Ein Konto meldet einen
-- Feed oder einen Artikel einmal; eine zweite Meldung ersetzt die erste. Die
-- Statusseite zeigt die gemeldeten Feeds mit Adresse und Zahl der Meldungen,
-- nie, wer gemeldet hat; das Konto steht hier nur, damit es seine Meldungen mit
-- sich nimmt, wenn es geht, und damit niemand einen Feed mehrfach meldet.
CREATE TABLE content_reports (
    id         bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    feed_id    integer     NOT NULL REFERENCES feeds ON DELETE CASCADE,
    seq        integer,    -- der gemeldete Artikel; leer, wenn der Feed gemeldet ist
    reason     text        NOT NULL
        CHECK (reason IN ('illegal', 'copyright', 'malicious', 'spam', 'other')),
    note       text        NOT NULL DEFAULT '' CHECK (char_length(note) <= 500),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (account_id, feed_id, seq)
);

CREATE INDEX content_reports_feed ON content_reports (feed_id, created_at);
