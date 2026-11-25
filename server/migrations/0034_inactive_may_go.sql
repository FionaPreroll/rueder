-- rueder: welches Konto als unbenutzt gehen darf (E13), an einer Stelle
--
-- Ein Konto ohne bestaetigte Adresse erreicht keine Mail, ein Admin bleibt,
-- damit eine Instanz ihren nicht verliert, und ueber ein gesperrtes hat der
-- Betreiber schon entschieden. Die Abfragen in server/src/inactive.rs fragen
-- nur hier, so dass die Regel nicht in jeder von ihnen steht.

CREATE FUNCTION inactive_may_go(account accounts) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    RETURN account.email IS NOT NULL AND NOT account.is_admin
        AND account.disabled_at IS NULL;
