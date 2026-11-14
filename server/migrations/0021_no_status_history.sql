-- Der Status fuer Admins zaehlt wieder nur die letzte Stunde, im Speicher.
--
-- Paket 5.14 schrieb die Zahlen des Status in die Datenbank, damit sie einen
-- Neustart ueberdauern und die Statusseite ueber Tage zaehlen konnte. Fiona
-- hat das am 2026-11-14 gestrichen: Fuer laengere Zeit gibt es die Metriken
-- fuer Prometheus (Paket 5.25). Die Aufbewahrung der Artikel in `retention`
-- bleibt.

DROP TABLE status_slots;
DROP TABLE status_problems;
ALTER TABLE retention DROP COLUMN status_days;
