-- rueder: Wie oft ein Feed hoechstens abgerufen wird, wie ein Admin es sagt
--
-- Manche Seiten finden auch das laengere Intervall nach einem 429 noch zu
-- oft. Fuer sie setzt ein Admin, der den Feed liest, einen kleinsten Abstand
-- zwischen zwei Abrufen, unter den das Intervall nie geht (src/fetch.rs).
-- NULL, wo es keinen gibt.

ALTER TABLE feeds ADD COLUMN min_interval_s integer CHECK (min_interval_s > 0);
