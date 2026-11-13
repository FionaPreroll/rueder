-- Rechtliches (Paket 5.8, server/src/legal.rs): welcher Fassung der
-- Nutzungsbedingungen ein Konto zugestimmt hat, und wann. Die Fassung nennt der
-- Betreiber mit TERMS_VERSION; ist sie eine andere als die hier, fragt die
-- Seite beim naechsten Besuch neu. Ohne TERMS_VERSION bleibt beides leer.
ALTER TABLE accounts
    ADD COLUMN terms_version     text,
    ADD COLUMN terms_accepted_at timestamptz;
