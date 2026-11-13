-- Ankunftsmarken fuer Apps anderer Anbieter (Paket 2.15, A24,
-- server/src/greader/).
--
-- Die Google-Reader-API fragt nach Artikeln, die seit einer Zeit eingetroffen
-- sind (ot, nt). rueder kennt die Reihenfolge des Eintreffens (articles.id),
-- aber keine Zeit dazu. Statt jedem Artikel eine Zeit und einen Index zu
-- geben, merkt sich diese kleine Tabelle je Minute, welche Nummer als erste
-- eintraf: hoechstens eine Zeile je Minute, in der etwas eintraf. Aus einer
-- Zeit wird so eine Grenze fuer articles.id, und die Listen bleiben so billig
-- wie die der Seite. Was vor dieser Migration eintraf, hat keine Marke; eine
-- Zeit davor schliesst nichts aus.
CREATE TABLE arrivals (
    at       timestamptz PRIMARY KEY,
    -- Die kleinste Nummer, die in dieser Minute eintraf. Ein Import kann
    -- kleinere Nummern bringen als zuvor eingetroffene, deshalb fragt die API
    -- das Minimum ueber alle Marken ab einer Zeit.
    first_id bigint      NOT NULL
);
