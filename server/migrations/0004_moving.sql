-- Umzug eines Kontos von einem anderen rueder (Paket 3.12, server/src/transfer.rs).
--
-- Ein Abo, das ein Import auf einem Feed anlegt, der hier noch nie abgerufen
-- wurde, kennt dessen Artikel noch nicht und kann seine Lesemarke nicht auf
-- eine Nummer setzen. Es merkt sich stattdessen die Zeit des Artikels, bis zu
-- dem auf dem anderen rueder gelesen war. Der erste Abruf, der Artikel bringt,
-- markiert alles, was vor dieser Zeit erschien, als gelesen und leert die
-- Spalte wieder. Ohne sie waeren nach einem Umzug alle neuen Feeds voller
-- ungelesener Artikel, die auf dem anderen rueder laengst gelesen waren.
ALTER TABLE subscriptions ADD COLUMN read_before timestamptz;
