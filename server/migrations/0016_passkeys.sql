-- Passkeys (server/src/passkeys.rs): ein Login ohne Passwort, der zugleich als
-- zweiter Faktor zaehlt, weil das Geraet PIN oder Biometrie verlangt.
--
-- Gespeichert ist je Passkey nur, was der Server zum Pruefen braucht: die
-- Nummer des Schluessels, die das Geraet nennt, und der oeffentliche
-- Schluessel mit Zaehler und Zustand, so wie webauthn-rs ihn als JSON
-- schreibt. Geheim ist davon nichts; den privaten Schluessel hat nur das
-- Geraet. Das Loeschen des Kontos nimmt seine Passkeys mit.

-- Wer das Konto fuer Passkeys ist: eine zufaellige UUID, die das Geraet beim
-- Login ohne Namen nennt. Sie verraet weder Namen noch Nummer des Kontos und
-- entsteht mit dem ersten Passkey.
ALTER TABLE accounts ADD COLUMN passkey_user uuid UNIQUE;

CREATE TABLE passkeys (
    id            integer     GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id    integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    -- Der Name, den der Leser ihm gab, etwa das Geraet.
    name          text        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    -- Die Nummer des Schluessels; dieselbe gehoert nie zu zwei Konten.
    credential_id bytea       NOT NULL UNIQUE
                              CHECK (octet_length(credential_id) BETWEEN 1 AND 1023),
    -- Der Schluessel als JSON von webauthn-rs (`Passkey`).
    credential    jsonb       NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    -- Wann er zuletzt einen Login oder eine Bestaetigung trug; leer, bis dahin.
    used_at       timestamptz
);

CREATE INDEX passkeys_account ON passkeys (account_id);
