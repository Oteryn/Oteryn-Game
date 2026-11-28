-- Retain the tagged Ground address that admitted the action. Existing receipts
-- continue to name their Channel. A private activity never borrows that identity.
ALTER TABLE game_corpse_skinning_records
    ALTER COLUMN channel_id DROP NOT NULL,
    ADD COLUMN instance_id UUID CHECK (game_character_is_uuid_v7(instance_id)),
    ADD CONSTRAINT game_corpse_skinning_scope CHECK (num_nonnulls(channel_id,instance_id)=1);

DO $migration$
DECLARE
    guard REGPROCEDURE;
    body TEXT;
    anchor CONSTANT TEXT := 'g.channel_id=NEW.channel_id';
BEGIN
    FOREACH guard IN ARRAY ARRAY['game_corpse_skinning_admit()'::regprocedure,
                                 'game_corpse_skinning_complete()'::regprocedure] LOOP
        body := pg_get_functiondef(guard);
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
            RAISE EXCEPTION 'Unexpected Ground scope proof in %',guard;
        END IF;
        body := replace(body,anchor,
            '(g.channel_id,g.instance_id) IS NOT DISTINCT FROM (NEW.channel_id,NEW.instance_id)');
        IF guard='game_corpse_skinning_complete()'::regprocedure THEN
            -- The deferred craft row supplies the session and character. Bind the
            -- new Instance address to that activity's committed participant.
            body := replace(body,'AND c.world_id=i.world_id',$session$
                AND (NEW.instance_id IS NULL OR EXISTS (
                    SELECT 1 FROM game_durability_reconnect_sessions session
                    WHERE session.game_session_id=c.game_session_id AND session.character_id=c.character_id
                      AND session.runtime_scope_world_id=c.world_id AND session.runtime_scope_kind=2
                      AND session.runtime_scope_instance_id=NEW.instance_id
                      AND session.scope_ownership_generation=NEW.scope_ownership_generation
                      AND game_activity_session_origin_valid(c.game_session_id,c.character_id)))
                AND c.world_id=i.world_id$session$);
        END IF;
        EXECUTE body;
    END LOOP;
END $migration$;
