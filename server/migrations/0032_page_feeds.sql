-- rueder: Webseiten ohne Feed (A25)
--
-- Eine Seite, die keinen Feed nennt, liest rueder ueber RSSifier: feed_url ist
-- dann die Adresse der Seite, wie RSSifier sie schreibt, und der Abruf fragt
-- RSSifier statt der Seite selbst.

ALTER TABLE feeds
    -- 'feed' fuer einen Feed, den die Seite selbst hat; 'page' fuer eine Seite,
    -- aus der RSSifier einen macht.
    ADD COLUMN source text NOT NULL DEFAULT 'feed' CHECK (source IN ('feed', 'page'));
