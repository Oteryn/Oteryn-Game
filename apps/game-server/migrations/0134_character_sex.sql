-- APP-3: creation authority, never a client appearance selection or guessed backfill.
-- Historical roots without a creation choice retain NULL and cannot authorize appearance.
ALTER TABLE game_character_roots ADD COLUMN sex SMALLINT CHECK (sex IN (1, 2));

-- Reuse the creation owner's immutable-field guard. No operation changes creation sex.
CREATE TRIGGER game_character_sex_immutable
    BEFORE UPDATE OF sex ON game_character_roots
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
