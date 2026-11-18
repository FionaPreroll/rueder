-- rueder: Die Sprache eines Kontos ist eine der Seite
--
-- Mails gehen in der Sprache des Kontos (src/mail.rs, Language), und die Seite
-- spricht mehr als Englisch und Deutsch. Eine Sprache mit Region wie "pt-BR"
-- steht hier mit ihr.

ALTER TABLE accounts
    DROP CONSTRAINT accounts_language_check,
    ADD CONSTRAINT accounts_language_check
        CHECK (language IN ('en', 'de', 'es', 'fr', 'pt-BR', 'it', 'ja'));
