-- HUNT-1 bounded storage extension of PARTY-XP-1's existing owner (0141).
-- PREY-CONTENT-1 is not admitted: no initialization, draws, unlock claims or
-- player writes are activated and no receipt/revision arm is invented here.
-- free_reroll_at is trusted Unix seconds; zero preserves legacy rows as ready.
ALTER TABLE game_character_prey_slots
    ADD COLUMN creature_list TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    ADD COLUMN option TEXT NOT NULL DEFAULT 'NONE'
        CHECK (option IN ('NONE', 'AUTO_REROLL', 'LOCK')),
    ADD COLUMN free_reroll_at BIGINT NOT NULL DEFAULT 0 CHECK (free_reroll_at >= 0);

CREATE FUNCTION game_prey_creature_list_valid(keys TEXT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT SET search_path = pg_catalog, pg_temp AS $$
    -- CASE bounds both unnest scans before evaluation, including rejected inputs.
    SELECT CASE WHEN cardinality(keys) > 9 THEN FALSE ELSE
       (cardinality(keys) = 0 OR (array_ndims(keys) = 1 AND array_lower(keys, 1) = 1))
       AND NOT EXISTS (SELECT 1 FROM unnest(keys) AS k
           WHERE k IS NULL OR octet_length(k) NOT BETWEEN 1 AND 256)
       AND cardinality(keys) = (SELECT count(DISTINCT k) FROM unnest(keys) AS k)
    END
$$;
ALTER TABLE game_character_prey_slots
    ADD CONSTRAINT game_prey_creature_list_bound CHECK (game_prey_creature_list_valid(creature_list)),
    -- Canary ioprey.cpp reloadBonusValue at 52550d5e3c0ffe565f4bd1014f4e5575abf1059b.
    -- Grade 1 may be retained even though fresh draws start at grade 2 (PREY0 §6.2).
    ADD CONSTRAINT game_prey_bonus_grade_value CHECK (bonus_type IS NULL OR bonus_percent = CASE bonus_type
        WHEN 'DAMAGE' THEN 2 * grade + 5
        WHEN 'REDUCTION' THEN 2 * grade + 10
        WHEN 'XP' THEN 3 * grade + 10
        WHEN 'LOOT' THEN 3 * grade + 10
    END);

-- Read-only until the content+fenced writer+receipt bundle qualifies. In
-- particular there is no direct runtime INSERT/UPDATE grant, including on the
-- newly added columns. Existing table SELECT grants cover the added columns.
REVOKE ALL ON FUNCTION game_prey_creature_list_valid(TEXT[]) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_prey_creature_list_valid(TEXT[])
    TO oteryn_game_runtime, oteryn_game_control;
