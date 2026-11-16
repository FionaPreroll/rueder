-- rueder: Artikel unter neuer Kennung (Paket 5.42)
--
-- Google News gibt demselben Artikel bei jedem Abruf eine neue Kennung. Der
-- Abruf erkennt ihn deshalb auch an Titel und Datum in seinem Feed
-- (server/src/feeds.rs, same_in_feed) und fragt dafuer nach den Artikeln des
-- Feeds mit dem Datum eines Eintrags. Ohne diesen Index las er die neuesten
-- Artikel des Feeds, je einer auf einer eigenen Seite (docs/implementation.md,
-- Duplikate vom 2026-11-16).
CREATE INDEX articles_published ON articles (feed_id, published_at);
