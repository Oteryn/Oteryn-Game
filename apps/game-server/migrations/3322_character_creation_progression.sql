-- Birth progression is committed with the build at revision one. Its level and XP
-- match FIRST-ENTRY (Tibia Global: level 1, no XP). Gameplay still binds its full
-- progression context to the activated policy through the fenced initializer.
LOCK TABLE game_character_roots IN SHARE ROW EXCLUSIVE MODE;

CREATE TABLE IF NOT EXISTS game_character_creation_progression (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    level BIGINT NOT NULL DEFAULT 1 CHECK (level = 1),
    total_experience BIGINT NOT NULL DEFAULT 0 CHECK (total_experience = 0)
);

REVOKE ALL ON game_character_creation_progression FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_creation_progression TO oteryn_game_runtime;
GRANT SELECT ON game_character_creation_progression TO oteryn_game_control, oteryn_game_highscore_job;

CREATE OR REPLACE FUNCTION game_character_initialize_creation_build() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    IF NEW.creation_vocation IS DISTINCT FROM NULL THEN
        INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,
            fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,
            distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,
            committed_character_revision,last_build_occurrence_id)
        VALUES(NEW.character_id,NEW.creation_vocation,0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,1,NEW.character_id);

        INSERT INTO game_character_creation_progression(character_id) VALUES (NEW.character_id);
    END IF;
    RETURN NEW;
END;
$$;

REVOKE ALL ON FUNCTION game_character_initialize_creation_build() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_initialize_creation_build() TO oteryn_game_runtime, oteryn_game_control;

-- A root beyond revision one must already have policy-bound progression.
-- Preserve that invariant and keep existing saved progression authoritative.
INSERT INTO game_character_creation_progression(character_id)
SELECT r.character_id
FROM game_character_roots r
WHERE r.creation_vocation IS DISTINCT FROM NULL
  AND r.character_revision = 1
  AND NOT EXISTS (
      SELECT 1 FROM game_character_progression_state p WHERE p.character_id = r.character_id
  )
ON CONFLICT (character_id) DO NOTHING;
