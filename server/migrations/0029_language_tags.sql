-- rueder: Die Datenbank prueft nur die Form der Sprache eines Kontos
--
-- Welche Sprachen es gibt, weiss der Code (src/mail.rs, Language), und es
-- kommen weitere dazu. Hier steht nur, dass es ein Sprach-Tag ist wie "de",
-- "pt-BR" oder "zh-Hant", damit nicht jede neue Sprache eine Migration
-- braucht.

ALTER TABLE accounts
    DROP CONSTRAINT accounts_language_check,
    ADD CONSTRAINT accounts_language_check
        CHECK (language ~ '^[a-z]{2,3}(-[A-Z][a-z]{3})?(-[A-Z]{2})?$');
