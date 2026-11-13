-- rueder: Aufbewahrung der Artikel (Paket 5.3, A21, E4)
--
-- Was alt ist, darf gehen, wenn der Betreiber es einstellt (rueder retention
-- set); als Standard bleibt alles, wie bisher. Einstellbar sind:
--   - Artikel nach so vielen Monaten seit ihrer Ankunft, wahlweise ganz oder
--     nur ihr Text (entschieden am 2026-11-13: beides je Instanz waehlbar);
--   - Versionen nach so vielen Monaten, indem ganze Monatspartitionen von
--     article_versions fallen;
--   - Feeds ohne Abonnenten nach so vielen Tagen, samt ihren Artikeln.
-- Artikel mit Stern oder Label eines Kontos bleiben. Aufgeraeumt wird
-- stuendlich in kleinen Portionen (server/src/retention.rs).

-- In der einen Zeile, die schon sagt, wie lange der Status bleibt (0015).
ALTER TABLE retention
    -- Nach wie vielen Monaten seit der Ankunft Artikel gehen; leer heisst nie.
    ADD COLUMN articles_months integer CHECK (articles_months BETWEEN 1 AND 1200),
    -- Was dann geht: 'article' der ganze Artikel, 'text' nur sein Text, die
    -- Zeile in der Liste bleibt. Der Betreiber nennt es ausdruecklich.
    ADD COLUMN articles_what   text    CHECK (articles_what IN ('article', 'text')),
    -- Nach wie vielen Monaten Versionen gehen; leer heisst nie.
    ADD COLUMN versions_months integer CHECK (versions_months BETWEEN 1 AND 1200),
    -- Nach wie vielen Tagen ohne Abonnenten ein Feed geht; leer heisst nie.
    ADD COLUMN orphans_days    integer CHECK (orphans_days BETWEEN 1 AND 36500),
    ADD CONSTRAINT retention_articles_what_with_months
        CHECK ((articles_months IS NULL) = (articles_what IS NULL));

-- Wann rueder einen Artikel zuerst abrief. Danach zaehlt sein Alter, nicht nach
-- dem Datum, das der Feed nennt: ein Feed kann einen Artikel rueckdatieren, und
-- der Import des alten rueder brachte Artikel mit altem Datum. Der Abruf setzt
-- die Zeit des Abrufs, der Import die Zeit, zu der der alte rueder den Artikel
-- zuerst sah; innerhalb eines Feeds steigt sie mit seq.
--
-- Artikel von vor dieser Migration bekommen ihre Zeit: PostgreSQL schreibt die
-- Tabelle dafuer nicht neu, sondern merkt sich den Wert. Fuer die Produktion,
-- die erst seit dem 2026-11-12 abruft, ist das bis auf Tage genau; der Import
-- der alten Daten kommt erst danach.
ALTER TABLE articles ADD COLUMN arrived_at timestamptz NOT NULL DEFAULT now();

-- Der Text ist gegangen, die Zeile in der Liste ist geblieben (Aufbewahrung
-- "nur der Text"). Der Abruf laesst den Artikel dann in Ruhe.
ALTER TABLE article_content ADD COLUMN text_dropped boolean NOT NULL DEFAULT false;

ALTER TABLE feeds
    -- Bis hierher sind die Artikel des Feeds aufgeraeumt: An oder unter cut_seq
    -- gibt es nur noch Artikel mit Stern oder Label, und jede Lesemarke steht
    -- mindestens hier. So zaehlen die Ungelesen-Zahlen weiter aus Lesemarke und
    -- Ausnahmen, ohne Artikel zu zaehlen, die es nicht mehr gibt.
    ADD COLUMN cut_seq      integer NOT NULL DEFAULT 0 CHECK (cut_seq >= 0),
    -- Bis hierher sind die Texte des Feeds aufgeraeumt (Aufbewahrung "nur der
    -- Text"): an oder unter text_cut_seq hat nur noch einen Text, was einen
    -- Stern oder ein Label hat.
    ADD COLUMN text_cut_seq integer NOT NULL DEFAULT 0 CHECK (text_cut_seq >= 0),
    -- Seit wann der Feed keine Abonnenten hat, wie es die Aufbewahrung zuerst
    -- bemerkte; leer, solange ihn jemand liest.
    ADD COLUMN orphaned_at  timestamptz,
    -- Der Artikel an cut_seq, wie der Umzug eines Kontos ihn nennt, wenn eine
    -- Lesemarke dort steht: Er selbst ist meist gegangen, und ohne ihn wuesste
    -- die andere Instanz nicht, bis wohin gelesen ist.
    ADD COLUMN cut_guid         text,
    ADD COLUMN cut_url          text,
    ADD COLUMN cut_published_at timestamptz;

-- Ein neues Abo beginnt nicht unter dem, was aufgeraeumt ist, wer es auch
-- anlegt: Abonnieren, der Umzug eines Kontos, der Import. Sonst zaehlten
-- geloeschte Artikel als ungelesen. Die Sperre wartet, falls die Aufbewahrung
-- den Feed gerade aufraeumt, und liest danach die neue Grenze.
CREATE FUNCTION subscription_above_cut() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    -- Eine Marke, die es nicht geben kann, bleibt, wie sie ist, und die Pruefung
    -- der Tabelle lehnt sie ab.
    IF NEW.read_upto >= 0 THEN
        NEW.read_upto := greatest(NEW.read_upto,
                                  (SELECT cut_seq FROM feeds WHERE id = NEW.feed_id FOR KEY SHARE));
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER subscriptions_above_cut
    BEFORE INSERT ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION subscription_above_cut();

-- Die Kennungen ganz geloeschter Artikel. Steht ein Artikel noch im Feed, wenn
-- die Aufbewahrung ihn loescht, wie bei Podcasts mit allen Folgen, erkennt der
-- Abruf ihn hieran und legt ihn nicht als neu an. Eine Zeile ist rund 60 Bytes,
-- ein Bruchteil des Textes, der ging; sie geht mit ihrem Feed.
CREATE TABLE article_tombstones (
    feed_id   integer NOT NULL REFERENCES feeds ON DELETE CASCADE,
    guid_hash bytea   NOT NULL CHECK (octet_length(guid_hash) = 16),
    PRIMARY KEY (feed_id, guid_hash)
);

-- Ob ein Artikel ein Label hat, fragt die Aufbewahrung nach Feed und Nummer,
-- und so loescht PostgreSQL die Labels eines geloeschten Artikels mit. Der
-- Primaerschluessel beginnt mit dem Label und hilft dabei nicht.
CREATE INDEX article_labels_article ON article_labels (feed_id, seq);

-- Loescht die Partitionen von article_versions, deren Monat ganz vor `before`
-- liegt, und nennt sie. Wartet hoechstens fuenf Sekunden auf die Tabelle, damit
-- der Abruf nicht hinter dem Loeschen wartet; dann eben beim naechsten Lauf.
CREATE FUNCTION drop_versions_partitions(before timestamptz) RETURNS SETOF text
LANGUAGE plpgsql AS $$
DECLARE
    part  text;
    month date;
BEGIN
    PERFORM set_config('lock_timeout', '5s', true);
    -- Wie beim Anlegen: zwei zugleich sollen sich nicht in die Quere kommen.
    PERFORM pg_advisory_xact_lock(hashtext('rueder.article_versions.partition'));
    FOR part IN
        SELECT c.relname FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid
        WHERE i.inhparent = 'article_versions'::regclass
          AND c.relname ~ '^article_versions_[0-9]{4}_[0-9]{2}$'
        ORDER BY c.relname
    LOOP
        month := to_date(substr(part, 18), 'YYYY_MM');
        IF ((month + interval '1 month')::timestamp AT TIME ZONE 'UTC') <= before THEN
            EXECUTE format('DROP TABLE %I', part);
            RETURN NEXT part;
        END IF;
    END LOOP;
END $$;
