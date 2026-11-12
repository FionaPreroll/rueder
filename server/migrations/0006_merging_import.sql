-- Einmaliger Import der alten Datenbank in den laufenden Bestand (Paket 4.3,
-- server/src/import.rs).
--
-- Der neue rueder laeuft schon, wenn die alten Daten dazukommen. Hat ein alter
-- Feed hier ein Gegenstueck mit derselben Adresse, kommen seine alten Artikel
-- vor die, die der neue rueder selbst abgerufen hat: Sie bekommen die Nummern
-- 1 bis n, und die vorhandenen Artikel ruecken um n nach hinten. Das ist die
-- einzige Stelle, an der sich vergebene Nummern aendern; sie ist vertretbar,
-- weil der Server waehrenddessen steht und es einmal geschieht.

-- Damit das Verschieben einer Nummer in articles alles mitnimmt, was an ihr
-- haengt (Text, Fassungen, Labels, alte UUIDs), folgen die Fremdschluessel der
-- Aenderung. Die Ausnahmen vom Lesestand (article_states) haengen am Abo, nicht
-- am Artikel; die verschiebt der Import selbst.
ALTER TABLE article_content
    DROP CONSTRAINT article_content_feed_id_seq_fkey,
    ADD CONSTRAINT article_content_feed_id_seq_fkey FOREIGN KEY (feed_id, seq)
        REFERENCES articles ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE article_versions
    DROP CONSTRAINT article_versions_feed_id_seq_fkey,
    ADD CONSTRAINT article_versions_feed_id_seq_fkey FOREIGN KEY (feed_id, seq)
        REFERENCES articles ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE legacy_articles
    DROP CONSTRAINT legacy_articles_feed_id_seq_fkey,
    ADD CONSTRAINT legacy_articles_feed_id_seq_fkey FOREIGN KEY (feed_id, seq)
        REFERENCES articles ON UPDATE CASCADE ON DELETE CASCADE;

ALTER TABLE article_labels
    DROP CONSTRAINT article_labels_feed_id_seq_fkey,
    ADD CONSTRAINT article_labels_feed_id_seq_fkey FOREIGN KEY (feed_id, seq)
        REFERENCES articles ON UPDATE CASCADE ON DELETE CASCADE;

-- Der Browser-Speicher des alten Frontends nennt Artikel mit ihrer alten Nummer
-- im Feed (Paket 4.6). Wo ein alter Artikel hier schon da war, weicht die neue
-- davon ab. Zeilen aus einem Import vor dieser Migration hatten die alte Nummer
-- als seq, wenn der Import sie nicht nennen musste.
ALTER TABLE legacy_articles
    ADD COLUMN legacy_seq integer,
    -- Den Artikel hatte der neue rueder schon selbst abgerufen, der Import hat
    -- ihn nicht angelegt; die alte UUID zeigt auf ihn.
    ADD COLUMN matched boolean NOT NULL DEFAULT false;

UPDATE legacy_articles SET legacy_seq = seq;

ALTER TABLE legacy_articles ALTER COLUMN legacy_seq SET NOT NULL;

CREATE UNIQUE INDEX legacy_articles_legacy_seq ON legacy_articles (feed_id, legacy_seq);
