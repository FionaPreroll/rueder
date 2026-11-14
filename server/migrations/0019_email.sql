-- rueder: E-Mail am Konto (Paket 5.4, A16, A17, E6)
--
-- Ob die Registrierung nach einer Adresse fragt, sagt der Betreiber mit
-- `rueder registration set --email off|optional|required`; als Standard nicht.
-- Jede Adresse wird mit einem Link bestaetigt, bevor sie am Konto steht: 32
-- zufaellige Byte wie bei Einladungen, gespeichert nur ihr SHA-256, 24 Stunden
-- gueltig, einmal nutzbar. Verschickt wird ueber das Relais, das der Betreiber
-- mit SMTP_URL nennt (E6).

ALTER TABLE registration
    ADD COLUMN email text NOT NULL DEFAULT 'off'
        CHECK (email IN ('off', 'optional', 'required'));

ALTER TABLE accounts
    -- Die bestaetigte Adresse. Eine, die erst bestaetigt werden soll, steht in
    -- email_links.
    ADD COLUMN email          text CHECK (length(email) <= 254),
    -- Die Sprache, in der das Konto angelegt wurde: in ihr gehen seine Mails.
    -- Leer bei Konten von vor dieser Migration, von der Kommandozeile und von
    -- OpenID Connect, die keine Seite angelegt hat; bei ihnen gilt die Sprache
    -- der Seite, in der sie ihre erste Adresse angeben.
    ADD COLUMN language       text CHECK (language IN ('en', 'de')),
    -- Mit `--email required` registriert und noch nicht bestaetigt: Das Konto
    -- meldet sich nicht an, bis der Link geklickt ist.
    ADD COLUMN awaiting_email boolean NOT NULL DEFAULT false;

-- Eine Adresse gehoert hoechstens einem Konto; gross oder klein ist eins.
CREATE UNIQUE INDEX accounts_email ON accounts (lower(email));

-- Links, die eine Adresse bestaetigen. Je Konto gilt nur der neueste, ein
-- benutzter geht.
CREATE TABLE email_links (
    token_hash bytea       PRIMARY KEY,
    account_id integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    email      text        NOT NULL CHECK (length(email) <= 254),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX email_links_account ON email_links (account_id);
