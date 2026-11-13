-- Was an Artikel beim letzten Mal geaendert hat (auf Fionas Wunsch vom
-- 2026-11-13).
--
-- Eine neue Version entsteht, sobald sich Titel oder Text aendern; die
-- Oberflaeche sagte bisher nur "aktualisiert". Oft ist nur der Titel anders,
-- und dann sieht man im Text nichts. Der Abruf haelt deshalb fest, was anders
-- war, und die Liste kann es sagen.
--
-- Leer heisst: unbekannt. So bleiben die Artikel, die vor dieser Migration
-- eine Version bekamen, und die, die noch nie eine bekamen.

ALTER TABLE articles
    ADD COLUMN updated_what text
        CHECK (updated_what IN ('title', 'text', 'both'));
