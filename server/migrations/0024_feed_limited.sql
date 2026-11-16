-- rueder: Wann die Seite eines Feeds zuletzt sagte, sie werde zu oft gefragt
--
-- Antwortet eine Seite mit 429, bleibt ihr Feed eine Weile beim laengeren
-- Intervall (src/fetch.rs, LIMITED_FOR), statt mit dem ersten Erfolg wieder
-- so oft zu fragen, wie es die Seite eben zu oft fand.

ALTER TABLE feeds ADD COLUMN limited_at timestamptz;
