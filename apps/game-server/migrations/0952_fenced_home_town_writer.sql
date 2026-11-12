-- #2046 slice 1: the table boundary independently re-proves the session fence.
-- Other gameplay/revision/receipt checks remain in the existing transaction.
CREATE FUNCTION game_character_home_town_write(
    p_character uuid, p_session uuid, p_generation numeric,
    p_lease numeric, p_scope_generation numeric, p_town integer
) RETURNS integer LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    -- Keep reconnect/replacement from superseding the proof until commit.
    PERFORM 1 FROM game_durability_reconnect_sessions
      WHERE game_session_id = p_session AND character_id = p_character
        AND current_generation = p_generation
        AND character_lease_generation = p_lease
        AND scope_ownership_generation = p_scope_generation
        AND session_state IN (1,2)
      FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'home town session fence rejected' USING ERRCODE = '42501';
    END IF;
    INSERT INTO game_character_home_towns(character_id, town_id)
      VALUES (p_character, p_town)
      ON CONFLICT (character_id) DO UPDATE SET town_id = EXCLUDED.town_id;
    RETURN p_town;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_character_home_town_write(uuid,uuid,numeric,numeric,numeric,integer) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_home_town_write(uuid,uuid,numeric,numeric,numeric,integer) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_home_town_write(uuid,uuid,numeric,numeric,numeric,integer) TO oteryn_game_runtime;
REVOKE INSERT, UPDATE, DELETE ON game_character_home_towns FROM oteryn_game_runtime;
