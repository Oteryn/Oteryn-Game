-- M55: retain the child completion owner and extend its exact helper/target pairs.
-- Zero-ordinal server causes retain their original Quest receipt cause guard.
ALTER TABLE game_quest_world_completions
    DROP CONSTRAINT game_quest_world_completions_cause_ordinal_check;
ALTER TABLE game_quest_world_completions
    ADD CONSTRAINT game_quest_world_completions_cause_ordinal_check CHECK(cause_ordinal>=0);

DO $migration$
DECLARE body text; anchor text := '    IF NOT FOUND OR r.cause_kind';
BEGIN
    body := pg_get_functiondef('game_quest_world_completion_guard()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'M55 requires the existing world child completion guard';
    END IF;
    EXECUTE replace(body,anchor,$guard$
    IF FOUND AND r.completes AND r.quest_key IN
        ('oteryn:quest.hive_native_shared','oteryn:quest.deeplings_native_shared') THEN
        IF NEW.quest_key IS DISTINCT FROM (CASE r.quest_key
              WHEN 'oteryn:quest.hive_native_shared' THEN 'oteryn:quest.authored.hive_born_world_change'
              WHEN 'oteryn:quest.deeplings_native_shared' THEN 'oteryn:quest.deeplings_worldchange' END)
            OR r.committed_character_revision<>NEW.committed_character_revision
            OR r.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
            OR NOT EXISTS(SELECT 1 FROM game_character_quest_states s
                WHERE s.character_id=NEW.character_id AND s.quest_key=NEW.quest_key
                  AND s.pinned_content_revision=NEW.pinned_content_revision AND s.definition_hash=NEW.definition_hash
                  AND s.completed_character_revision=NEW.committed_character_revision
                  AND s.committed_character_revision=NEW.committed_character_revision) THEN
            RAISE EXCEPTION 'counter alias requires exact canonical state and fresh helper completion' USING ERRCODE='23514';
        END IF;
        -- Persisted helper provenance is necessary but insufficient: independently
        -- resolve and lock the currently active World and source cycle as well.
        PERFORM 1 FROM game_character_counter_quest_scopes s
            JOIN game_world_reset_active a ON a.world_id=s.world_id
            JOIN game_world_change_counters c ON c.world_id=a.world_id AND c.reset_epoch=a.reset_epoch
                AND c.change_key=s.change_key
            WHERE s.character_id=r.character_id AND s.committed_character_revision=r.committed_character_revision
              AND s.change_key=CASE r.quest_key WHEN 'oteryn:quest.hive_native_shared' THEN 'hive_born' ELSE 'deeplings' END
              AND s.occurrence_generation=coalesce((c.state->>'occurrence_generation')::numeric,1)
            FOR SHARE OF a,c;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'counter alias requires the current locked source cycle' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NOT FOUND OR r.cause_kind$guard$);
END $migration$;
