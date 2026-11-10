-- rueder: Schema v1 (PostgreSQL 16)
--
-- Grundideen (ausfuehrlich in docs/architecture.md):
--   * Artikel liegen in drei Tabellen: articles (heiss, schmal, fuer Listen),
--     article_content (warm, der aktuelle Text) und article_versions (kalt, alle
--     Fassungen im Original, komprimiert, nach Monaten geteilt). Listen lesen nie
--     Text, alte Fassungen beruehren nie die Tabellen fuer neue Artikel.
--   * Jeder Artikel hat eine feste, lueckenlose Nummer in seinem Feed (seq). Der
--     Feed kennt seine hoechste Nummer (last_seq). Blaettern und Zaehlen kommen
--     ohne Fensterfunktion und ohne COUNT(*) ueber Artikel aus.
--   * Der Lesestand ist eine Lesemarke je Abo ("alles bis Nummer N ist gelesen")
--     plus Ausnahmen je Artikel. Die Ungelesen-Zahl wird daraus berechnet, es gibt
--     keinen gespeicherten Zaehler.
--   * Schluessel sind fortlaufende Zahlen. Was aus dem alten rueder kommt, behaelt
--     seine UUID in legacy_id.

-- ---------------------------------------------------------------- Konten

CREATE TABLE accounts (
    id            integer     GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username      text        NOT NULL CHECK (username <> ''),
    -- argon2id. Leer, solange kein Passwort gesetzt ist (importiertes Konto) oder
    -- das Konto sich nur ueber OpenID Connect anmeldet.
    password_hash text,
    is_admin      boolean     NOT NULL DEFAULT false,
    -- Einstellungen der Oberflaeche (Thema, Dichte, Tastatur ...).
    settings      jsonb       NOT NULL DEFAULT '{}',
    legacy_id     uuid        UNIQUE,              -- users.id im alten rueder
    created_at    timestamptz NOT NULL DEFAULT now()
);

-- Login-Namen unterscheiden keine Gross- und Kleinschreibung.
CREATE UNIQUE INDEX accounts_username ON accounts (lower(username));

-- Anmeldung ueber einen fremden Aussteller: OpenID Connect (issuer = Adresse des
-- Anbieters) oder der alte Login (issuer = auth_origin, subject = auth_subject).
CREATE TABLE account_identities (
    issuer     text    NOT NULL,
    subject    text    NOT NULL,
    account_id integer NOT NULL REFERENCES accounts ON DELETE CASCADE,
    PRIMARY KEY (issuer, subject)
);

CREATE INDEX account_identities_account ON account_identities (account_id);

-- Browser-Sitzungen. Im Cookie steht ein zufaelliges Token, hier nur sein SHA-256.
CREATE TABLE sessions (
    token_hash   bytea       PRIMARY KEY,
    account_id   integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    created_at   timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    expires_at   timestamptz NOT NULL
);

CREATE INDEX sessions_account ON sessions (account_id);

-- ---------------------------------------------------------------- Feeds

-- Ein Feed gehoert allen seinen Abonnenten gemeinsam.
CREATE TABLE feeds (
    id               integer     GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- Nicht UNIQUE: das alte Schema hat es nicht erzwungen, der Import soll an
    -- doppelten Adressen nicht scheitern.
    feed_url         text        NOT NULL CHECK (feed_url <> ''),
    site_url         text,
    title            text        NOT NULL DEFAULT '',
    icon_url         text,
    -- Hoechste vergebene Artikelnummer. Wird in derselben Transaktion erhoeht, in
    -- der die Artikel eingefuegt werden; die Sperre auf dieser Zeile sorgt dafuer,
    -- dass je Feed nur einer Nummern vergibt.
    last_seq         integer     NOT NULL DEFAULT 0 CHECK (last_seq >= 0),
    -- Womit der Feed abgerufen wird: ehrlich als rueder oder wie ein Browser.
    fetch_agent      text        NOT NULL DEFAULT 'rueder'
                                 CHECK (fetch_agent IN ('rueder', 'browser')),
    fetch_interval_s integer     NOT NULL DEFAULT 3600 CHECK (fetch_interval_s > 0),
    next_fetch_at    timestamptz NOT NULL DEFAULT now(),
    fetched_at       timestamptz,
    etag             text,
    last_modified    text,
    last_success_at  timestamptz,
    last_error_at    timestamptz,
    error_message    text,
    legacy_id        uuid        UNIQUE,           -- feeds.id im alten rueder
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX feeds_feed_url ON feeds (feed_url);
CREATE INDEX feeds_next_fetch ON feeds (next_fetch_at);

-- ---------------------------------------------------------------- Ordner und Abos

-- Ordner und Abos einer Ebene teilen sich eine Reihenfolge (position). Die
-- Position ist ein Sortierschluessel zwischen den Nachbarn, damit ein Zug nur die
-- bewegte Zeile aendert.
CREATE TABLE folders (
    id         integer          GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id integer          NOT NULL REFERENCES accounts ON DELETE CASCADE,
    parent_id  integer,                            -- leer: oberste Ebene
    title      text             NOT NULL CHECK (title <> ''),
    position   double precision NOT NULL,
    -- Ziel fuer Fremdschluessel, die das Konto mitpruefen: ein Abo oder Unterordner
    -- kann nur in einem Ordner desselben Kontos liegen.
    UNIQUE (account_id, id),
    CHECK (parent_id <> id),
    -- Faellt ein Ordner an der Anwendung vorbei weg, rutscht sein Inhalt auf die
    -- oberste Ebene. Die Anwendung haengt ihn vorher an die Stelle des Ordners.
    FOREIGN KEY (account_id, parent_id) REFERENCES folders (account_id, id)
        ON DELETE SET NULL (parent_id)
);

CREATE INDEX folders_tree ON folders (account_id, parent_id, position);

-- Hoechstens fuenf Ebenen, keine Kreise. Geprueft wird der Ordner mit allem, was
-- in ihm liegt. Wer den Baum aendert, sperrt vorher die Zeile des Kontos, sonst
-- koennten zwei gleichzeitige Zuege zusammen die Grenze reissen.
CREATE FUNCTION folders_check_depth() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    above  integer := 0;      -- Ebenen ueber dem Ordner
    below  integer := 1;      -- Ebenen im Ordner, ihn selbst mitgezaehlt
    circle boolean := false;
BEGIN
    IF NEW.parent_id IS NOT NULL THEN
        WITH RECURSIVE up AS (
            SELECT f.id, f.parent_id, 1 AS n FROM folders f WHERE f.id = NEW.parent_id
            UNION ALL
            SELECT f.id, f.parent_id, up.n + 1
            FROM folders f JOIN up ON f.id = up.parent_id
            WHERE up.n < 16
        )
        SELECT count(*), coalesce(bool_or(up.id = NEW.id), false)
        INTO above, circle FROM up;
    END IF;
    IF circle THEN
        RAISE EXCEPTION 'a folder cannot move into itself'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'folders_no_circle';
    END IF;

    WITH RECURSIVE down AS (
        SELECT f.id, 2 AS n FROM folders f WHERE f.parent_id = NEW.id
        UNION ALL
        SELECT f.id, down.n + 1
        FROM folders f JOIN down ON f.parent_id = down.id
        WHERE down.n < 16
    )
    SELECT coalesce(max(down.n), 1) INTO below FROM down;

    IF above + below > 5 THEN
        RAISE EXCEPTION 'folders nest at most 5 levels deep'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'folders_max_depth';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER folders_check_depth
    BEFORE INSERT OR UPDATE OF parent_id ON folders
    FOR EACH ROW EXECUTE FUNCTION folders_check_depth();

CREATE TABLE subscriptions (
    account_id integer          NOT NULL REFERENCES accounts ON DELETE CASCADE,
    feed_id    integer          NOT NULL REFERENCES feeds ON DELETE CASCADE,
    folder_id  integer,                            -- leer: oberste Ebene
    position   double precision NOT NULL,
    title      text,                               -- eigener Name statt des Feed-Titels
    -- Die Lesemarke: alles bis zu dieser Nummer ist gelesen, bis auf die
    -- Ausnahmen in article_states. Sie bewegt sich nie zurueck.
    read_upto  integer          NOT NULL DEFAULT 0 CHECK (read_upto >= 0),
    created_at timestamptz      NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, feed_id),
    FOREIGN KEY (account_id, folder_id) REFERENCES folders (account_id, id)
        ON DELETE SET NULL (folder_id)
);

CREATE INDEX subscriptions_feed ON subscriptions (feed_id);
CREATE INDEX subscriptions_tree ON subscriptions (account_id, folder_id, position);

-- ---------------------------------------------------------------- Artikel

-- Heiss: eine schmale Zeile je Artikel, alles was eine Liste zeigt.
CREATE TABLE articles (
    feed_id       integer     NOT NULL REFERENCES feeds ON DELETE CASCADE,
    -- Laufende Nummer im Feed, ab 1, lueckenlos, nie neu vergeben.
    seq           integer     NOT NULL CHECK (seq > 0),
    -- Reihenfolge des Eintreffens ueber alle Feeds, fuer gemischte Listen
    -- (Ordner, "Alle Artikel"). Innerhalb eines Feeds steigt sie mit seq.
    id            bigint      GENERATED BY DEFAULT AS IDENTITY,
    published_at  timestamptz NOT NULL,
    -- Wann die aktuelle Fassung die vorige ersetzt hat; leer, solange es nur die
    -- erste gibt. Daraus wird der Hinweis "aktualisiert".
    updated_at    timestamptz,
    version       smallint    NOT NULL DEFAULT 1 CHECK (version >= 1),
    title         text        NOT NULL,
    teaser        text        NOT NULL DEFAULT '',
    author        text        NOT NULL DEFAULT '',
    url           text,
    thumbnail_url text,
    PRIMARY KEY (feed_id, seq)
);

CREATE UNIQUE INDEX articles_arrival ON articles (feed_id, id);

-- Warm: genau eine Zeile je Artikel mit dem bereinigten Text der aktuellen
-- Fassung und dem, was nur die Artikelansicht und der Abruf brauchen.
CREATE TABLE article_content (
    feed_id     integer NOT NULL,
    seq         integer NOT NULL,
    -- Woran der Abruf einen Artikel wiedererkennt: SHA-256 seiner Kennung im Feed
    -- (GUID, ersatzweise der Link), die ersten 16 Bytes.
    guid_hash   bytea   NOT NULL CHECK (octet_length(guid_hash) = 16),
    -- Pruefsumme ueber den normalisierten Inhalt der aktuellen Fassung. Weicht sie
    -- beim naechsten Abruf ab, entsteht eine neue Fassung.
    checksum    bytea   NOT NULL,
    guid        text    NOT NULL,
    html        text    NOT NULL,
    tags        text[]  NOT NULL DEFAULT '{}',
    enclosures  jsonb   NOT NULL DEFAULT '[]',
    image_url   text,
    image_title text,
    PRIMARY KEY (feed_id, seq),
    FOREIGN KEY (feed_id, seq) REFERENCES articles ON DELETE CASCADE
);

-- Der Abruf fragt je Artikel des Feeds "kenne ich den, und mit welcher
-- Pruefsumme?". Der Index beantwortet das ohne die Tabelle.
CREATE UNIQUE INDEX article_content_guid
    ON article_content (feed_id, guid_hash) INCLUDE (seq, checksum);

ALTER TABLE article_content ALTER COLUMN html SET COMPRESSION lz4;

-- Kalt: jede Fassung im Original (Titel, Text, Metadaten als JSON, mit zstd
-- komprimiert). Es wird nur angehaengt. Nach Monat des Abrufs geteilt, damit alte
-- Monate als Ganzes auf langsameren Speicher oder weg koennen.
CREATE TABLE article_versions (
    feed_id    integer     NOT NULL,
    seq        integer     NOT NULL,
    version    smallint    NOT NULL CHECK (version >= 1),
    fetched_at timestamptz NOT NULL,
    checksum   bytea       NOT NULL,
    raw        bytea       NOT NULL,
    PRIMARY KEY (feed_id, seq, version, fetched_at),
    FOREIGN KEY (feed_id, seq) REFERENCES articles ON DELETE CASCADE
) PARTITION BY RANGE (fetched_at);

-- Schon komprimiert, PostgreSQL soll es nicht noch einmal versuchen.
ALTER TABLE article_versions ALTER COLUMN raw SET STORAGE EXTERNAL;

-- Legt die Partition fuer den Monat an, in dem der Tag liegt (Monate in UTC).
-- Der Server ruft das beim Start und danach regelmaessig fuer den laufenden und
-- den naechsten Monat auf, der Import fuer jeden Monat, den er braucht.
CREATE FUNCTION ensure_versions_partition(day date) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
    first date := date_trunc('month', day)::date;
    next  date := (date_trunc('month', day) + interval '1 month')::date;
BEGIN
    -- Zwei gleichzeitige Aufrufe sollen sich nicht in die Quere kommen.
    PERFORM pg_advisory_xact_lock(hashtext('rueder.article_versions.partition'));
    EXECUTE format(
        'CREATE TABLE IF NOT EXISTS %I PARTITION OF article_versions
             FOR VALUES FROM (%L) TO (%L)',
        'article_versions_' || to_char(first, 'YYYY_MM'),
        first::timestamp AT TIME ZONE 'UTC',
        next::timestamp AT TIME ZONE 'UTC'
    );
END $$;

-- Die UUIDs der Artikel aus dem alten rueder. Gebraucht wird das nur, um beim
-- Umstieg die Labels aus dem Browser zuzuordnen; danach kann die Tabelle weg.
CREATE TABLE legacy_articles (
    legacy_id uuid    PRIMARY KEY,
    feed_id   integer NOT NULL,
    seq       integer NOT NULL,
    FOREIGN KEY (feed_id, seq) REFERENCES articles ON DELETE CASCADE
);

-- ---------------------------------------------------------------- Lesestand

-- Die Ausnahmen von der Lesemarke und alles Weitere, was ein Konto an einem
-- Artikel festhaelt. Eine Zeile gibt es nur, solange mindestens eins davon gilt.
--   read:        gelesen, obwohl ueber der Lesemarke
--   kept_unread: ungelesen, obwohl an oder unter der Lesemarke
--   hidden:      von einer Filterregel ausgeblendet, zaehlt nie als ungelesen
--   starred:     mit Stern
-- Ungelesen in einem Feed sind damit
--     (last_seq - read_upto)
--   - Zeilen ueber der Marke mit read oder hidden
--   + Zeilen an oder unter der Marke mit kept_unread und ohne hidden.
CREATE TABLE article_states (
    account_id  integer NOT NULL,
    feed_id     integer NOT NULL,
    seq         integer NOT NULL CHECK (seq > 0),
    read        boolean NOT NULL DEFAULT false,
    kept_unread boolean NOT NULL DEFAULT false,
    hidden      boolean NOT NULL DEFAULT false,
    starred     boolean NOT NULL DEFAULT false,
    PRIMARY KEY (account_id, feed_id, seq),
    FOREIGN KEY (account_id, feed_id) REFERENCES subscriptions ON DELETE CASCADE,
    CONSTRAINT article_states_not_empty CHECK (read OR kept_unread OR hidden OR starred),
    CONSTRAINT article_states_read_or_kept CHECK (NOT (read AND kept_unread))
);

-- Fuer die Zaehlung unter der Marke und die Listen "Mit Stern" und "Ausgeblendet".
CREATE INDEX article_states_kept_unread
    ON article_states (account_id, feed_id, seq) WHERE kept_unread;
CREATE INDEX article_states_starred
    ON article_states (account_id, feed_id, seq) WHERE starred;
CREATE INDEX article_states_hidden
    ON article_states (account_id, feed_id, seq) WHERE hidden;

-- ---------------------------------------------------------------- Filterregeln

-- Eine Regel gilt fuer alle Feeds des Kontos (folder_id und feed_id leer), fuer
-- einen Ordner mit allem darunter oder fuer einen Feed.
CREATE TABLE filter_rules (
    id         integer          GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id integer          NOT NULL REFERENCES accounts ON DELETE CASCADE,
    position   double precision NOT NULL,
    enabled    boolean          NOT NULL DEFAULT true,
    folder_id  integer,
    feed_id    integer,
    field      text             NOT NULL
                                CHECK (field IN ('any', 'title', 'content', 'author', 'tag')),
    match      text             NOT NULL CHECK (match IN ('contains', 'word', 'regex')),
    pattern    text             NOT NULL
                                CHECK (pattern <> '' AND char_length(pattern) <= 200),
    action     text             NOT NULL CHECK (action IN ('hide', 'read')),
    created_at timestamptz      NOT NULL DEFAULT now(),
    CONSTRAINT filter_rules_one_scope CHECK (folder_id IS NULL OR feed_id IS NULL),
    FOREIGN KEY (account_id, folder_id) REFERENCES folders (account_id, id)
        ON DELETE CASCADE,
    FOREIGN KEY (account_id, feed_id) REFERENCES subscriptions (account_id, feed_id)
        ON DELETE CASCADE
);

CREATE INDEX filter_rules_account ON filter_rules (account_id, position);

-- ---------------------------------------------------------------- Labels

CREATE TABLE labels (
    id         integer          GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id integer          NOT NULL REFERENCES accounts ON DELETE CASCADE,
    title      text             NOT NULL CHECK (title <> ''),
    color      text             NOT NULL CHECK (color ~ '^#[0-9a-fA-F]{6}$'),
    position   double precision NOT NULL,
    UNIQUE (account_id, title)
);

CREATE TABLE article_labels (
    label_id   integer     NOT NULL REFERENCES labels ON DELETE CASCADE,
    feed_id    integer     NOT NULL,
    seq        integer     NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (label_id, feed_id, seq),
    FOREIGN KEY (feed_id, seq) REFERENCES articles ON DELETE CASCADE
);
