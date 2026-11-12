-- Meldungen von Inhalten und Sperre von Feeds (Paket 5.9).
--
-- Ein User meldet einen Feed oder einen Artikel daraus, mit einem Grund aus
-- einer festen Liste und einem kurzen Text. Admins sehen auf der Statusseite
-- die Adresse gemeldeter Feeds und wie oft sie gemeldet wurden, nie wer
-- gemeldet hat. Gesperrt wird auf der Kommandozeile.
--
-- Die Nummer 0009 laesst 0008 fuer das Loeschen von Konten (Paket 5.6) frei,
-- das in einem eigenen Pull Request liegt.

-- Wann der Feed gesperrt wurde; leer, solange er es nicht ist. Ein gesperrter
-- Feed wird nicht abgerufen, seine Artikel sieht niemand, und niemand kann ihn
-- neu abonnieren. Die Abos bleiben, damit eine Aufhebung alles zurueckbringt.
ALTER TABLE feeds ADD COLUMN blocked_at timestamptz;

-- Eine Meldung je Konto und Feed oder Artikel: wer noch einmal meldet, ersetzt
-- Grund und Text seiner Meldung. seq ist leer, wenn der ganze Feed gemeint ist.
-- Die Meldung haengt am Konto, damit das Loeschen des Kontos sie mitnimmt.
CREATE TABLE feed_reports (
    id         bigint      GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    feed_id    integer     NOT NULL REFERENCES feeds ON DELETE CASCADE,
    seq        integer     CHECK (seq > 0),
    reason     text        NOT NULL
                           CHECK (reason IN ('spam', 'illegal', 'hate', 'copyright',
                                             'malware', 'other')),
    text       text        NOT NULL DEFAULT '' CHECK (length(text) <= 500),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (account_id, feed_id, seq)
);

CREATE INDEX feed_reports_feed ON feed_reports (feed_id);

-- Die Bilder gesperrter Feeds, als SHA-256 ihrer Adresse. Die Sperre loescht
-- sie aus dem Cache des Bild-Proxy; diese Liste haelt ihn davon ab, sie fuer
-- eine Adresse, die ein Browser noch kennt, wieder zu holen.
CREATE TABLE blocked_images (
    url_hash bytea   NOT NULL,
    feed_id  integer NOT NULL REFERENCES feeds ON DELETE CASCADE,
    PRIMARY KEY (url_hash, feed_id)
);

CREATE INDEX blocked_images_feed ON blocked_images (feed_id);
