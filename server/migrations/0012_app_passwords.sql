-- App-Passwoerter fuer Apps anderer Anbieter (Paket 2.15, A24,
-- server/src/app_passwords.rs).
--
-- Apps wie Reeder, NetNewsWire oder Read You melden sich ueber die
-- Google-Reader-API mit einem App-Passwort an, nie mit dem Passwort des
-- Kontos. Jedes entsteht in den Einstellungen, wird einmal gezeigt und steht
-- hier nur als SHA-256; es ist zufaellig und lang genug, dass ein langsamer
-- Hash nichts gewinnt. Ein neues Passwort des Kontos und das Sperren beenden
-- alle App-Passwoerter, das Loeschen des Kontos nimmt sie mit.
CREATE TABLE app_passwords (
    id          integer     GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id  integer     NOT NULL REFERENCES accounts ON DELETE CASCADE,
    -- Der Name, den der Leser ihm gab, meist der der App.
    name        text        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),
    secret_hash bytea       NOT NULL UNIQUE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    -- Wann eine App es zuletzt benutzt hat; leer, bis sie sich das erste Mal
    -- anmeldet. Geschrieben wird hoechstens einmal je Stunde.
    used_at     timestamptz
);

CREATE INDEX app_passwords_account ON app_passwords (account_id);
