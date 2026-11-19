-- rueder: Platz auf jeder Seite der Abos fuer die naechste Lesemarke (Paket 5.35)
--
-- Jedes Lesen kann die Lesemarke eines Abos weiterruecken (read_upto). Die
-- Spalte steht in keinem Index, und findet die neue Fassung der Zeile Platz
-- auf derselben Seite, braucht PostgreSQL keinen neuen Eintrag in den Indizes
-- (HOT). In einer dicht gepackten Tabelle findet sie ihn selten: Gemessen mit
-- 100.000 Lesern kam nur ein Drittel der Updates so aus, der Rest liess tote
-- Zeilen und Indexeintraege zurueck. Mit einem Zehntel jeder Seite frei waren
-- es alle (siehe docs/implementation.md, "Messung").
--
-- Das gilt fuer Seiten, die von jetzt an geschrieben werden. Eine Tabelle,
-- die schon dicht gepackt ist, bekommt den Platz mit der Zeit, wenn Zeilen
-- wandern und VACUUM aufraeumt, oder sofort mit VACUUM FULL subscriptions.

ALTER TABLE subscriptions SET (fillfactor = 90);
