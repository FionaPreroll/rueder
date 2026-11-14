-- rueder: Namen der Autoren ohne Klammern (Design-Review F1, entschieden von
-- Fiona am 2026-11-14)
--
-- Aus "desk@example.org (Jonas Erbe)" in einem RSS-Feed machte der Abruf bis
-- zum 2026-11-14 den Autor "(Jonas Erbe)", aus "Jonas Erbe <desk@...>" ganz
-- selten "<...>". Neu abgerufene Artikel bekommen den Namen ohne Klammern
-- (server/src/feeds.rs, author); hier bekommen ihn auch die gespeicherten,
-- nach derselben Regel: Steht der ganze Name in runden oder spitzen Klammern,
-- fallen sie weg und mit ihnen Leerzeichen am Rand.
--
-- Der Trigger articles_copy (0005) bringt die Namen in die Kopie, die die
-- Suche liest. Fassungen in article_versions bleiben, wie sie waren: Sie
-- zeigen, was der Feed damals geschickt hat. Regeln auf den Autor wirken nur
-- bei der Ankunft und werden hier nicht neu ausgewertet.
UPDATE articles
SET author = btrim(substr(author, 2, length(author) - 2))
WHERE length(author) >= 2
  AND (author LIKE '(%)' OR author LIKE '<%>');
