-- Session locks do not fence the control role's direct projection writes.
CREATE FUNCTION game_exiva_lock_staff_access(p_source uuid, p_target uuid)
RETURNS TABLE(character_id uuid, staff_access boolean)
LANGUAGE plpgsql VOLATILE STRICT SECURITY DEFINER AS $$
BEGIN
    -- SHARE locks coexist across casts and block projection writes, including
    -- insertion of a missing role that would otherwise fall back to admission.
    -- A cast already holds admission locks. Refuse if a writer holds the
    -- projection instead of waiting on a writer that might need admission.
    LOCK TABLE game_character_spell_access IN SHARE MODE NOWAIT;
    RETURN QUERY
        SELECT a.character_id, a.staff_access
        FROM game_character_spell_access AS a
        WHERE a.character_id IN (p_source, p_target);
END;
$$;
DO $$ BEGIN
    EXECUTE format(
        'ALTER FUNCTION game_exiva_lock_staff_access(uuid,uuid) SET search_path=pg_catalog,%I,pg_temp',
        current_schema()
    );
END $$;
ALTER FUNCTION game_exiva_lock_staff_access(uuid,uuid) OWNER TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_exiva_lock_staff_access(uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_exiva_lock_staff_access(uuid,uuid) TO oteryn_game_runtime;
