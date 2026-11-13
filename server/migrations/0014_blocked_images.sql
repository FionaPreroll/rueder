-- Die Bilder gesperrter Feeds (Nachtrag zu Paket 5.9).
--
-- Die Sperre loescht die Bilder eines Feeds aus dem Cache des Bild-Proxy. Ein
-- Browser, der ihre signierten Adressen noch kennt, liesse den Proxy sie sonst
-- beim naechsten Abruf wieder holen und ablegen. Hier steht je Bild der
-- SHA-256 seiner Adresse; der Proxy holt nichts, was hier steht. Die Aufhebung
-- der Sperre loescht die Zeilen ihres Feeds.
CREATE TABLE blocked_images (
    url_hash bytea   NOT NULL,
    feed_id  integer NOT NULL REFERENCES feeds ON DELETE CASCADE,
    PRIMARY KEY (url_hash, feed_id)
);

CREATE INDEX blocked_images_feed ON blocked_images (feed_id);
