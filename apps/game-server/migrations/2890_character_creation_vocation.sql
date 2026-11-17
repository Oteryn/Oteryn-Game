-- Immutable creation choice. Historical roots retain NULL, preserving their `none` seed.
ALTER TABLE game_character_roots ADD COLUMN creation_vocation TEXT
    CHECK (creation_vocation IN ('knight','paladin','sorcerer','druid','monk'));
CREATE TRIGGER game_character_creation_vocation_immutable
    BEFORE UPDATE OF creation_vocation ON game_character_roots
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();

-- A creation seed is independently proven from the immutable root, without inventing a
-- gameplay mutation or advancing the initial CharacterRevision.
ALTER TABLE game_character_build_state DROP CONSTRAINT game_character_build_state_committed_character_revision_check;
ALTER TABLE game_character_build_state ADD CONSTRAINT game_character_build_state_committed_character_revision_check
    CHECK (committed_character_revision BETWEEN 1 AND 18446744073709551615);
CREATE FUNCTION game_character_creation_build_seed(c UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_character_build_state b JOIN game_character_roots r USING(character_id)
        WHERE b.character_id=c AND r.creation_vocation IS NOT NULL
          AND b.vocation=r.creation_vocation AND b.committed_character_revision=1
          AND b.last_build_occurrence_id=c
          AND (b.magic_level,b.mana_spent,b.fist_level,b.fist_tries,b.club_level,b.club_tries,
               b.sword_level,b.sword_tries,b.axe_level,b.axe_tries,b.distance_level,b.distance_tries,
               b.shielding_level,b.shielding_tries,b.fishing_level,b.fishing_tries)
              = (0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0)
          AND b.offline_pool_s=43200 AND b.offline_skill=-1)
$$;
CREATE FUNCTION game_character_initialize_creation_build() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.creation_vocation IS NOT NULL THEN
        INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,
            fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,
            distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,
            committed_character_revision,last_build_occurrence_id)
        VALUES(NEW.character_id,NEW.creation_vocation,0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,1,NEW.character_id);
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER game_character_initialize_creation_build AFTER INSERT ON game_character_roots
    FOR EACH ROW EXECUTE FUNCTION game_character_initialize_creation_build();

-- Extend the current guard chain, retaining every intervening migration's arms.
DO $$
DECLARE body TEXT; changed TEXT;
BEGIN
    body := pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure);
    IF position('EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character)' IN body)=0
       OR position('IS DISTINCT FROM (''none'', 0, 0, 10' IN body)=0
       OR position('AND NOT EXISTS (SELECT 1 FROM build_chain))' IN body)=0 THEN
        RAISE EXCEPTION 'expected all three Character build seed guard arms';
    END IF;
    changed := replace(body,
        'EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character)',
        'EXISTS (SELECT 1 FROM game_character_build_state WHERE character_id = v_character AND NOT game_character_creation_build_seed(v_character))');
    changed := replace(changed, 'IS DISTINCT FROM (''none'', 0, 0, 10',
        'IS DISTINCT FROM ((SELECT coalesce(creation_vocation,''none'') FROM game_character_roots WHERE character_id=v_character), 0, 0, 10');
    changed := replace(changed, 'AND NOT EXISTS (SELECT 1 FROM build_chain))',
        'AND NOT EXISTS (SELECT 1 FROM build_chain) AND NOT game_character_creation_build_seed(v_character))');
    IF changed = body OR position('IS DISTINCT FROM (''none'', 0, 0, 10' IN changed)>0
       OR position('AND NOT EXISTS (SELECT 1 FROM build_chain))' IN changed)>0 THEN
        RAISE EXCEPTION 'expected Character build seed guard arms';
    END IF;
    EXECUTE changed;
END;
$$;
-- Training's empty-chain projection is the immutable creation seed too.
DO $$
DECLARE body TEXT;
BEGIN
    body := pg_get_functiondef('game_verify_training_integrity(uuid)'::regprocedure);
    IF position('IF NOT FOUND THEN RETURN; END IF;' IN body)=0 THEN
        RAISE EXCEPTION 'expected training empty-state guard';
    END IF;
    EXECUTE replace(body, 'IF NOT FOUND THEN RETURN; END IF;',
        'IF NOT FOUND THEN RETURN; END IF; IF game_character_creation_build_seed(p_character) AND NOT EXISTS (SELECT 1 FROM game_character_build_receipts WHERE character_id=p_character UNION ALL SELECT 1 FROM game_character_death_receipts WHERE character_id=p_character AND vocation IS NOT NULL) THEN RETURN; END IF;');
END;
$$;
REVOKE ALL ON FUNCTION game_character_creation_build_seed(UUID), game_character_initialize_creation_build() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_creation_build_seed(UUID), game_character_initialize_creation_build() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_character_creation_build_seed(UUID), game_character_initialize_creation_build() TO oteryn_game_control;
