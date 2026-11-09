-- Extend the earlier capture custody owner; both lanes use the same receipt.
-- Placement identity includes the complete runtime occurrence, so a second
-- character or command cannot credit the same captured target.
CREATE UNIQUE INDEX game_quest_creature_capture_occurrence
    ON game_quest_creature_captures(target_identity);

CREATE FUNCTION game_quest_creature_occurrence_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id <> pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_quest_reward_sources s
        JOIN game_character_quest_receipts r USING(character_id,cause_id,cause_ordinal,transition_key)
        WHERE s.character_id=NEW.character_id AND s.cause_id=NEW.cause_id
            AND s.cause_ordinal=NEW.cause_ordinal AND s.transition_key=NEW.transition_key
            AND s.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
            AND r.cause_kind='command' AND s.plan->'capture_creature'='true'::jsonb
            AND COALESCE(s.plan->'fresh_corpse','false'::jsonb)='false'::jsonb
            AND s.plan->>'creature_target'=NEW.creature_key
    ) THEN
        RAISE EXCEPTION 'capture lacks same-transaction live quest receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_creature_occurrence_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
CREATE CONSTRAINT TRIGGER game_quest_creature_occurrence_complete AFTER INSERT ON game_quest_creature_captures
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_creature_occurrence_complete();
REVOKE ALL ON FUNCTION game_quest_creature_occurrence_complete() FROM PUBLIC;
-- Conversely, a declared capture cannot credit progress without its occurrence.
CREATE FUNCTION game_quest_capture_source_complete() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.plan->'capture_creature'='true'::jsonb AND NOT EXISTS (
        SELECT 1 FROM game_quest_creature_captures c
        WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND c.creature_key=NEW.plan->>'creature_target'
            AND c.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'capture reward lacks exact live occurrence' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_capture_source_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
CREATE CONSTRAINT TRIGGER game_quest_capture_source_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_capture_source_complete();
REVOKE ALL ON FUNCTION game_quest_capture_source_complete() FROM PUBLIC;
