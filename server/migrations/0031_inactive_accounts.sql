-- rueder: Konten, die niemand mehr benutzt (E13)
--
-- Stellt der Betreiber es ein (rueder retention set --inactive), geht ein
-- Konto nach so vielen Monaten ohne Nutzung, nach zwei Mails an seine
-- bestaetigte Adresse. Ohne Adresse, als Admin und gesperrt geht keines.

ALTER TABLE accounts
    -- Wann das Konto zuletzt benutzt wurde: ein Login, eine Anfrage mit einer
    -- Sitzung oder eine App, hoechstens einmal am Tag geschrieben. Bestehende
    -- Konten zaehlen ab dieser Migration, damit keines frueher geht, als die
    -- Frist nach dem Update erlaubt.
    ADD COLUMN active_at timestamptz NOT NULL DEFAULT now(),
    -- Wann die erste und die zweite Mail vor dem Loeschen hinausgingen; leer,
    -- sobald das Konto wieder benutzt wird.
    ADD COLUMN inactive_warned_at   timestamptz,
    ADD COLUMN inactive_reminded_at timestamptz;

ALTER TABLE retention
    -- Nach wie vielen Monaten ohne Nutzung ein Konto geht; leer behaelt es.
    ADD COLUMN inactive_months integer CHECK (inactive_months > 0);
