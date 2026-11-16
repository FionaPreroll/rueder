-- rueder: Die Woerter eines Artikels werden gespeichert (Paket 5.38)
--
-- Die Suche sieht sich zuerst die neuesten Artikel selbst an (src/search.rs) und
-- bestimmte dafuer die Woerter jedes Textes neu. Bei langen Texten, wie sie Feeds
-- mit dem ganzen Artikel liefern, war das der groesste Teil ihrer Zeit (gemessen,
-- siehe docs/implementation.md, "Messung"). Jetzt werden die Woerter einmal beim
-- Speichern bestimmt, in eine Spalte, auf der auch der Index liegt.
--
-- Ohne Positionen (strip): Die Suche kennt weder Wortfolgen noch eine Rangfolge,
-- und ohne sie ist die Spalte deutlich kleiner.

ALTER TABLE article_content
    ADD COLUMN words tsvector
    GENERATED ALWAYS AS (strip(article_search_doc(title, author, tags, html))) STORED;

DROP INDEX article_content_search;
CREATE INDEX article_content_search ON article_content USING gin (words)
    WITH (fastupdate = off);
-- Wie in 0005: Die Statistik soll auch die Woerter mittlerer Haeufigkeit kennen.
ALTER TABLE article_content ALTER COLUMN words SET STATISTICS 1000;
ANALYZE article_content;
