-- rueder: Volltextsuche (Paket 2.13)
--
-- Gesucht wird ueber einen GIN-Index auf die Woerter der aktuellen Fassung eines
-- Artikels: Titel, Autor, Schlagworte und Text. Der Index ist ein Ausdruck ueber
-- article_content; die Woerter selbst werden nicht gespeichert, sie waeren
-- groesser als der Text (gemessen, siehe docs/implementation.md, "Messung").
--
-- Damit der Ausdruck alles in einer Zeile findet, bekommt article_content eine
-- Kopie von Titel, Autor und Reihenfolge des Eintreffens (id) aus articles. Die id
-- sortiert die Treffer wie die Listen, ohne fuer jeden Treffer in articles
-- nachzusehen. Die Kopie pflegen Trigger, wer auch immer schreibt: Abruf,
-- Import und erneutes Bereinigen.
--
-- Die Woerter kommen ohne Stammformen ('simple'): Feeds sind deutsch, englisch
-- und gemischt, eine falsche Sprache waere schlimmer als keine. Die Suche fragt
-- dafuer nach Wortanfaengen ("wahl" findet "wahlen"). Es gibt keine Suche nach
-- Wortfolgen und keine Rangfolge, die Treffer kommen nach Alter.

-- Die Woerter eines Artikels. Gelesen werden nur die ersten 100.000 Zeichen des
-- Textes: Ein tsvector darf hoechstens 1 MB gross werden, und ein Artikel, an dem
-- das scheitert, liesse den Abruf seines Feeds scheitern. HTML-Tags und Entities
-- erkennt der Parser von PostgreSQL selbst und laesst sie aus. IMMUTABLE, weil
-- ein Index darauf liegt: das Ergebnis haengt nur an den Argumenten. COST sagt
-- dem Planer, wie teuer es ist, die Woerter eines Artikels neu zu bestimmen
-- (rund 45 Mikrosekunden fuer einen Text von 600 Zeichen); ohne das liest er
-- lieber tausende Artikel als den Index.
CREATE FUNCTION article_search_doc(title text, author text, tags text[], html text)
RETURNS tsvector
LANGUAGE sql IMMUTABLE PARALLEL SAFE COST 2000
RETURN to_tsvector('simple'::regconfig,
    title || ' ' || author || ' ' || array_to_string(tags, ' ') || ' ' || left(html, 100000));

ALTER TABLE article_content
    ADD COLUMN id     bigint,
    ADD COLUMN title  text NOT NULL DEFAULT '',
    ADD COLUMN author text NOT NULL DEFAULT '';

UPDATE article_content c SET id = a.id, title = a.title, author = a.author
FROM articles a WHERE a.feed_id = c.feed_id AND a.seq = c.seq;

ALTER TABLE article_content ALTER COLUMN id SET NOT NULL;

-- Ein neuer Artikel bekommt die Kopie aus articles, das vorher geschrieben wird.
-- Wer beides in einer Anweisung schreibt (die Messdaten), setzt sie selbst.
CREATE FUNCTION article_content_copy() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.id IS NULL THEN
        SELECT a.id, a.title, a.author INTO NEW.id, NEW.title, NEW.author
        FROM articles a WHERE a.feed_id = NEW.feed_id AND a.seq = NEW.seq;
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER article_content_copy
    BEFORE INSERT ON article_content
    FOR EACH ROW EXECUTE FUNCTION article_content_copy();

-- Eine neue Fassung aendert Titel und Autor in articles; die Kopie folgt.
CREATE FUNCTION articles_copy() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    UPDATE article_content c SET id = n.id, title = n.title, author = n.author
    FROM changed n
    WHERE c.feed_id = n.feed_id AND c.seq = n.seq
      AND (c.id, c.title, c.author) IS DISTINCT FROM (n.id, n.title, n.author);
    RETURN NULL;
END $$;

CREATE TRIGGER articles_copy
    AFTER UPDATE ON articles
    REFERENCING NEW TABLE AS changed
    FOR EACH STATEMENT EXECUTE FUNCTION articles_copy();

CREATE INDEX article_content_search
    ON article_content USING gin (article_search_doc(title, author, tags, html));
-- Wie haeufig ein Wort ist, entscheidet ueber den Plan. Die Statistik soll auch
-- die Woerter mittlerer Haeufigkeit kennen, nicht nur die haeufigsten.
ALTER INDEX article_content_search ALTER COLUMN 1 SET STATISTICS 1000;
-- Fuer haeufige Woerter: rueckwaerts durch die Artikel, bis die Seite voll ist.
CREATE INDEX article_content_id ON article_content (id);
