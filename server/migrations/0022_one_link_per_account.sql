-- rueder: Ein Link je Konto (Code-Review vom 2026-11-14, Punkt S5)
--
-- Je Konto gilt nur der neueste Link, der eine Adresse bestaetigt, und nur
-- der neueste, der ein neues Passwort setzt. Das sagt jetzt ein eindeutiger
-- Schluessel auf account_id: Ein neuer Link ersetzt den alten in einer
-- Anweisung (INSERT ... ON CONFLICT), ohne dass die Zeile des Kontos gesperrt
-- wird. Von mehreren Links eines Kontos bleibt vorher der neueste.

DELETE FROM email_links l
 USING email_links newer
 WHERE newer.account_id = l.account_id
   AND (newer.created_at, newer.token_hash) > (l.created_at, l.token_hash);
DROP INDEX email_links_account;
ALTER TABLE email_links ADD CONSTRAINT email_links_account UNIQUE (account_id);

DELETE FROM password_resets r
 USING password_resets newer
 WHERE newer.account_id = r.account_id
   AND (newer.created_at, newer.token_hash) > (r.created_at, r.token_hash);
DROP INDEX password_resets_account;
ALTER TABLE password_resets ADD CONSTRAINT password_resets_account UNIQUE (account_id);
