-- A separately owned child quest completes in the admitted native reward transaction.
-- This is not a second Character revision or a second item grant.
CREATE TABLE game_quest_world_completions (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal>0),
    transition_key TEXT NOT NULL,
    quest_key TEXT NOT NULL CHECK(quest_key ~ '^oteryn:quest[.][A-Za-z0-9._:/-]+$' AND octet_length(quest_key)<=128),
    pinned_content_revision TEXT NOT NULL,
    definition_hash BYTEA NOT NULL CHECK(octet_length(definition_hash)=32),
    committed_character_revision NUMERIC(20,0) NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    UNIQUE(character_id,quest_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_world_completion_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_character_quest_receipts%ROWTYPE;
BEGIN
    SELECT * INTO r FROM game_character_quest_receipts WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR r.cause_kind<>'use' OR r.completes OR r.quest_key=NEW.quest_key
        OR r.committed_character_revision<>NEW.committed_character_revision
        OR r.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_states s WHERE s.character_id=NEW.character_id
            AND s.quest_key=NEW.quest_key AND s.pinned_content_revision=NEW.pinned_content_revision
            AND s.definition_hash=NEW.definition_hash
            AND s.completed_character_revision=NEW.committed_character_revision
            AND s.committed_character_revision=NEW.committed_character_revision) THEN
        RAISE EXCEPTION 'world child completion lacks exact same-transaction native receipt/state' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_world_completion_guard_trigger
    AFTER INSERT ON game_quest_world_completions DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_world_completion_guard();
-- Preserve every primary and foreign-storage rule; only the separately proved child
-- can be written and carry its original completion into later ordinary transitions.
CREATE OR REPLACE FUNCTION game_character_quest_state_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r WHERE r.character_id=NEW.character_id
            AND r.committed_character_revision=NEW.committed_character_revision AND r.quest_key=NEW.quest_key
            AND r.pinned_content_revision=NEW.pinned_content_revision AND r.definition_hash=NEW.definition_hash)
        AND NOT EXISTS(SELECT 1 FROM game_quest_reward_participants p WHERE p.character_id=NEW.character_id
            AND p.committed_character_revision=NEW.committed_character_revision AND p.quest_key=NEW.quest_key
            AND p.pinned_content_revision=NEW.pinned_content_revision AND p.definition_hash=NEW.definition_hash
            AND p.created_xact_id=pg_current_xact_id())
        AND NOT EXISTS(SELECT 1 FROM game_quest_world_completions p WHERE p.character_id=NEW.character_id
            AND p.committed_character_revision=NEW.committed_character_revision AND p.quest_key=NEW.quest_key
            AND p.pinned_content_revision=NEW.pinned_content_revision AND p.definition_hash=NEW.definition_hash
            AND p.created_xact_id=pg_current_xact_id()))
        OR (TG_OP='INSERT' AND NEW.started_character_revision<>NEW.committed_character_revision)
        OR (NEW.completed_character_revision IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.committed_character_revision=NEW.completed_character_revision
                AND r.quest_key=NEW.quest_key AND r.completes)
            AND NOT EXISTS(SELECT 1 FROM game_quest_world_completions p WHERE p.character_id=NEW.character_id
                AND p.quest_key=NEW.quest_key AND p.committed_character_revision=NEW.completed_character_revision
                AND p.pinned_content_revision=NEW.pinned_content_revision AND p.definition_hash=NEW.definition_hash)) THEN
        RAISE EXCEPTION 'quest state is not written by a receipt of its quest' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_world_completion_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_character_quest_state_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_world_completion_guard() FROM PUBLIC;
REVOKE ALL ON FUNCTION game_character_quest_state_guard() FROM PUBLIC;
REVOKE ALL ON game_quest_world_completions FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_world_completions TO oteryn_game_runtime;
GRANT SELECT ON game_quest_world_completions TO oteryn_game_control;
