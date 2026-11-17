-- rueder: Filterregeln fuer den Anfang oder das Ende eines Textes und fuer alle Artikel
--
-- Eine Regel kann ihr Muster auch am Anfang oder am Ende eines Textes suchen
-- (src/filters.rs, Match). Eine Regel fuer alle Artikel hat kein Muster, und
-- nur sie.

ALTER TABLE filter_rules
    DROP CONSTRAINT filter_rules_match_check,
    ADD CONSTRAINT filter_rules_match_check
        CHECK (match IN ('contains', 'word', 'starts', 'ends', 'regex', 'all')),
    DROP CONSTRAINT filter_rules_pattern_check,
    ADD CONSTRAINT filter_rules_pattern_check
        CHECK ((pattern = '') = (match = 'all') AND char_length(pattern) <= 200);
