-- Konten, die sich selbst anlegen: offene Registrierung und Einladungen.
--
-- Ob sich jemand registrieren darf und wie viele, legt der Betreiber auf der
-- Kommandozeile fest (rueder registration set); es steht in der einen Zeile
-- von registration. Ohne sein Zutun bleibt alles zu, wie bisher.
--
-- Zwei Kontingente, die sich nicht beruehren:
--   - das allgemeine: Konten vom Betreiber, aus offener Registrierung und aus
--     Einladungen von Usern zaehlen gegen max_accounts, die beiden letzten
--     ausserdem gegen per_day;
--   - das der Admins: Konten aus Einladungen von Admins zaehlen nur gegen
--     admin_invite_accounts.

-- Woher ein Konto stammt. Daran haengt, gegen welches Kontingent es zaehlt.
ALTER TABLE accounts
    ADD COLUMN origin text NOT NULL DEFAULT 'operator'
        CHECK (origin IN ('operator', 'registration', 'invite', 'admin_invite')),
    -- Wie viele Konten dieses Konto insgesamt einladen darf; leer heisst: so
    -- viele wie registration.invites_per_account.
    ADD COLUMN invite_quota integer CHECK (invite_quota >= 0);

CREATE TABLE registration (
    -- Es gibt genau eine Zeile.
    single                boolean PRIMARY KEY DEFAULT true CHECK (single),
    -- Ob sich jeder ohne Einladung registrieren kann.
    open                  boolean NOT NULL DEFAULT false,
    -- Obergrenze des allgemeinen Kontingents; leer heisst unbegrenzt.
    max_accounts          integer CHECK (max_accounts >= 0),
    -- Neue Konten in 24 Stunden aus Registrierung und Einladungen von Usern.
    per_day               integer CHECK (per_day >= 0),
    -- Einladungen, die jedes Konto aussprechen darf, wenn es kein eigenes
    -- Kontingent hat.
    invites_per_account   integer NOT NULL DEFAULT 0 CHECK (invites_per_account >= 0),
    -- Wie viele Tage ein Einladungslink gilt.
    invite_days           integer NOT NULL DEFAULT 7 CHECK (invite_days BETWEEN 1 AND 3650),
    -- Obergrenze der Konten aus Einladungen von Admins; leer heisst unbegrenzt.
    admin_invite_accounts integer CHECK (admin_invite_accounts >= 0)
);

INSERT INTO registration DEFAULT VALUES;

-- Ein Einladungslink. Wie eine Sitzung ist er ein zufaelliges Token, von dem
-- nur der SHA-256 gespeichert ist; den Link selbst kennt nur, wer ihn bekam.
CREATE TABLE invites (
    id         integer     GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    token_hash bytea       NOT NULL UNIQUE,
    -- Wer einlaedt. Mit dem Konto gehen seine Links.
    account_id integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    -- Von einem Admin ausgestellt: zaehlt gegen das Kontingent der Admins und
    -- bleibt es, auch wenn das Konto spaeter kein Admin mehr ist.
    by_admin   boolean     NOT NULL,
    -- Fuer wie viele Konten der Link gilt, und wie viele er schon ergab.
    max_uses   integer     NOT NULL CHECK (max_uses >= 1),
    uses       integer     NOT NULL DEFAULT 0 CHECK (uses >= 0 AND uses <= max_uses),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    -- Zurueckgezogen von dem, der einlud.
    revoked    boolean     NOT NULL DEFAULT false
);

CREATE INDEX invites_account ON invites (account_id);

-- Ueber welchen Link ein Konto kam. Der Link kann spaeter geloescht werden,
-- origin bleibt.
ALTER TABLE accounts ADD COLUMN invite_id integer REFERENCES invites ON DELETE SET NULL;
