-- Exact live capture credit: never a client UUID or a command-only occurrence.
CREATE TABLE game_quest_creature_captures (
    target_identity BYTEA PRIMARY KEY CHECK(octet_length(target_identity)=16),
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
    transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
    creature_key TEXT NOT NULL CHECK(creature_key ~ '^oteryn:creature\.[a-z0-9_]+$'),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_creature_capture_complete() RETURNS trigger LANGUAGE plpgsql AS $$
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
    EXECUTE format('ALTER FUNCTION game_quest_creature_capture_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
CREATE CONSTRAINT TRIGGER game_quest_creature_capture_complete AFTER INSERT ON game_quest_creature_captures
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_creature_capture_complete();
CREATE TRIGGER game_quest_creature_capture_immutable BEFORE UPDATE OR DELETE ON game_quest_creature_captures
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_creature_capture_no_truncate BEFORE TRUNCATE ON game_quest_creature_captures
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_quest_creature_captures FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_creature_capture_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_creature_captures TO oteryn_game_runtime;
GRANT SELECT ON game_quest_creature_captures TO oteryn_game_control;
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
