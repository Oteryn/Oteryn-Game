-- PostgreSQL requires UPDATE privilege for FOR SHARE. Keep the staff
-- projection read-only for runtime while retaining the cast's row locks.
CREATE FUNCTION game_spell_access_read_locked(p_caster UUID, p_target UUID)
RETURNS TABLE(character_id UUID, staff_access BOOLEAN)
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
    SELECT a.character_id, a.staff_access
    FROM game_character_spell_access AS a
    WHERE a.character_id IN (p_caster, p_target)
    ORDER BY a.character_id
    FOR SHARE OF a
$$;

-- Catalogue metadata and scoped publications are insert-once for runtime.
CREATE FUNCTION game_boss_room_catalogue_read_locked(p_boss TEXT, p_unpublished_only BOOLEAN)
RETURNS SETOF game_boss_room_catalogue
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
    SELECT c.*
    FROM game_boss_room_catalogue AS c
    WHERE c.boss_key = p_boss AND c.enabled
      AND (NOT p_unpublished_only OR NOT EXISTS (
          SELECT 1 FROM game_boss_room_publications AS p WHERE p.boss_key = c.boss_key
      ))
    FOR SHARE OF c
$$;

CREATE FUNCTION game_boss_room_publication_read_locked(
    p_world UUID, p_boss TEXT, p_revision TEXT, p_frame BYTEA, p_map BYTEA, p_artifact BYTEA
)
RETURNS SETOF game_boss_room_publications
LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
    SELECT p.*
    FROM game_boss_room_publications AS p
    WHERE p.world_id = p_world AND p.boss_key = p_boss AND p.content_revision = p_revision
      AND p.origin_frame_digest = p_frame AND p.origin_map_digest = p_map
      AND p.server_artifact_digest = p_artifact AND p.enabled
    FOR SHARE OF p
$$;

-- The sleep writer owns the receipt table. The gameplay caller only needs
-- to distinguish a replay while holding the existing session lock.
CREATE FUNCTION game_house_bed_sleep_receipted(p_session UUID, p_command NUMERIC)
RETURNS BOOLEAN
LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
        SELECT 1 FROM game_house_bed_sleep_receipts AS r
        WHERE r.game_session_id = p_session AND r.command_id = p_command
    )
$$;

-- Preserve the house owner's table-level DELETE boundary. The death writer
-- can remove only the exact backpack slot covered by its current receipt.
CREATE FUNCTION game_player_death_remove_backpack_slot(p_item UUID, p_transaction UUID)
RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    DELETE FROM game_item_container_slots AS s
    USING game_player_death_item_records AS r
    WHERE s.item_instance_id = p_item
      AND r.transaction_id = p_transaction
      AND r.item_instance_id = s.item_instance_id
      AND r.character_id = s.character_id
      AND r.world_id = s.world_id
      AND r.source_placement = s.placed_transaction_id
      AND r.source_slot = 9
      AND r.created_xact_id = pg_current_xact_id();
    IF NOT FOUND THEN
        RAISE EXCEPTION 'backpack slot removal requires the exact death receipt'
            USING ERRCODE = '23514';
    END IF;
END $$;

DO $$
DECLARE
    signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY[
        'game_spell_access_read_locked(UUID, UUID)',
        'game_boss_room_catalogue_read_locked(TEXT, BOOLEAN)',
        'game_boss_room_publication_read_locked(UUID, TEXT, TEXT, BYTEA, BYTEA, BYTEA)',
        'game_house_bed_sleep_receipted(UUID, NUMERIC)',
        'game_player_death_remove_backpack_slot(UUID, UUID)'
    ] LOOP
        EXECUTE format(
            'ALTER FUNCTION %s SET search_path = pg_catalog, %I, pg_temp',
            signature, current_schema()
        );
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC', signature);
        EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime', signature);
    END LOOP;
END $$;

-- These identity columns already reject UPDATE via their installed row guards.
-- The column grants let the existing row locks protect the guarded evidence.
GRANT UPDATE (handoff_id) ON game_activity_handoff_participants TO oteryn_game_runtime;
GRANT UPDATE (handoff_id) ON game_activity_runtime_entries TO oteryn_game_runtime;
GRANT UPDATE (handoff_id) ON game_activity_runtime_placements TO oteryn_game_runtime;
GRANT UPDATE (recovery_grant_nonce) ON game_activity_transfer_recoveries TO oteryn_game_runtime;
GRANT UPDATE (character_id) ON game_character_blessings TO oteryn_game_runtime;
GRANT UPDATE (death_occurrence_id) ON game_character_death_receipts TO oteryn_game_runtime;
GRANT UPDATE (character_id) ON game_character_pending_respawns TO oteryn_game_runtime;
GRANT UPDATE (character_id) ON game_character_viola_book_cycles TO oteryn_game_runtime;
GRANT UPDATE (character_id) ON game_item_container_slots TO oteryn_game_runtime;
GRANT UPDATE (handoff_id) ON game_rotten_bak_phase_origins TO oteryn_game_runtime;
