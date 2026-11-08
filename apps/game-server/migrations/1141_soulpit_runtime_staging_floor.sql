-- Soulpit donor source z=8 is runtime/LastPosition floor 8. Only the native
-- map TilePos projection negates it (ADR-0021). Crystal 201189f
-- data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77.
-- Fix forward: keep exact shared-entry membership and transaction provenance.
CREATE OR REPLACE FUNCTION game_soulpit_participant_admit() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM game_soulpit_encounters f
        JOIN game_activity_handoff_participants p ON p.handoff_id=f.handoff_id
        WHERE f.handoff_id=NEW.handoff_id AND p.character_id=NEW.character_id
            AND f.revision=0 AND f.created_xact_id=pg_current_xact_id()
            AND p.entry_x=32375 AND p.entry_y=31158+NEW.staging_slot AND p.entry_floor=8)
    THEN RAISE EXCEPTION 'Soulpit membership is the exact shared entry snapshot' USING ERRCODE='23514'; END IF;
    RETURN NEW;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_soulpit_participant_admit() SET search_path=%I, pg_temp', current_schema());
END $$;
REVOKE ALL ON FUNCTION game_soulpit_participant_admit() FROM PUBLIC;
