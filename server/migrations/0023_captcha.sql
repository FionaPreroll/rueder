-- rueder: Captcha bei der offenen Registrierung (Paket 5.28)
--
-- Wer sich ohne Einladung registriert, loest vorher eine Rechenaufgabe
-- (src/captcha.rs). Der Betreiber schaltet sie mit
-- `rueder registration set --captcha false` ab. Sie ist an als Standard,
-- anders als sonst fuer die oeffentliche Instanz (A16), weil rueder als
-- oeffentliche Instanz von Haus aus sicher sein soll (Fiona, 2026-11-15).

ALTER TABLE registration ADD COLUMN captcha boolean NOT NULL DEFAULT true;
