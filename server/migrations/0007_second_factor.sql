-- Zweiter Faktor (Paket 5.7, server/src/second_factor.rs): TOTP nach RFC 6238
-- und zehn Wiederherstellungscodes.

-- Das Geheimnis fuer TOTP. Es steht so in der Datenbank, wie es ist, weil der
-- Server mit ihm jeden Code nachrechnet. totp_pending ist das Geheimnis einer
-- Einrichtung, die noch kein Code bestaetigt hat; erst dann wird es
-- totp_secret. totp_last_step ist der Zeitschritt des zuletzt angenommenen
-- Codes, damit derselbe Code nicht zweimal gilt.
ALTER TABLE accounts
    ADD COLUMN totp_secret    bytea,
    ADD COLUMN totp_pending   bytea,
    ADD COLUMN totp_last_step bigint;

-- Codes fuer den Fall, dass das Geraet mit dem Geheimnis fehlt. Gespeichert ist
-- nur ihr SHA-256: Sie sind zufaellig und lang genug, dass ein langsamer Hash
-- nichts dazugewinnt. Ein benutzter Code wird geloescht.
CREATE TABLE recovery_codes (
    account_id integer NOT NULL REFERENCES accounts ON DELETE CASCADE,
    code_hash  bytea   NOT NULL,
    PRIMARY KEY (account_id, code_hash)
);

-- Ob eine Sitzung mit dem zweiten Faktor begonnen hat. Verlangt der Betreiber
-- ihn fuer Admins, wirkt deren Recht nur in solchen Sitzungen.
ALTER TABLE sessions ADD COLUMN second_factor boolean NOT NULL DEFAULT false;
