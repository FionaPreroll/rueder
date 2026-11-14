-- rueder: Passwort vergessen (Paket 5.5, A17)
--
-- Mit bestaetigter Adresse schickt „Passwort vergessen“ einen Link: 32
-- zufaellige Byte, gespeichert nur ihr SHA-256, 30 Minuten gueltig, einmal
-- nutzbar. Je Konto gilt nur der neueste. Er gilt nur, solange das Konto noch
-- die Adresse hat, an die er ging; ein neues Passwort auf anderem Weg nimmt
-- ihn weg.

CREATE TABLE password_resets (
    token_hash bytea       PRIMARY KEY,
    account_id integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    -- Die Adresse, an die der Link ging.
    email      text        NOT NULL CHECK (length(email) <= 254),
    created_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL
);

CREATE INDEX password_resets_account ON password_resets (account_id);
