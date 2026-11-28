-- Character Info reads the committed Inbox count by Character.
GRANT SELECT (character_id, committed) ON game_character_inbox_counters
    TO oteryn_game_runtime;

-- Find Person already has SELECT. PostgreSQL also requires UPDATE on one
-- column for FOR SHARE. Keep the access projection writable only by control
-- and the table owner, including when runtime privileges are inherited.
CREATE OR REPLACE FUNCTION game_character_spell_access_guard_identity() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT pg_has_role(current_user, 'oteryn_game_control', 'USAGE')
       AND NOT pg_has_role(current_user, (SELECT relowner FROM pg_class WHERE oid = TG_RELID), 'USAGE') THEN
        RAISE EXCEPTION 'runtime cannot update spell access identity' USING ERRCODE = '42501';
    END IF;
    RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_character_spell_access_guard_identity() FROM PUBLIC;

DROP TRIGGER IF EXISTS game_character_spell_access_guard_identity ON game_character_spell_access;
CREATE TRIGGER game_character_spell_access_guard_identity
    BEFORE UPDATE OF character_id ON game_character_spell_access
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_spell_access_guard_identity();

GRANT UPDATE (character_id) ON game_character_spell_access
    TO oteryn_game_runtime;
