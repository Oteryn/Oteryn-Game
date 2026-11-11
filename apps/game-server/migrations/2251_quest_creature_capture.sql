-- M13: an exact living occurrence can enter one quest/tool custody receipt only once.
-- The existing Quest, carried-input and MINT owners retain their own complete guards.
CREATE TABLE game_quest_creature_captures (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
    transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation BETWEEN 1 AND 18446744073709551615),
    target_identity BYTEA NOT NULL CHECK(octet_length(target_identity)=16 AND target_identity<>decode(repeat('00',16),'hex')),
    target_generation NUMERIC(20,0) NOT NULL CHECK(target_generation BETWEEN 1 AND 18446744073709551615),
    creature_key TEXT NOT NULL CHECK(creature_key ~ '^oteryn:creature\.[a-z0-9_]+$' AND octet_length(creature_key)<=160),
    source_item UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    ingress BYTEA NOT NULL CHECK(octet_length(ingress) BETWEEN 1 AND 4096),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    UNIQUE(world_id,channel_id,scope_generation,target_identity,target_generation),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_quest_creature_capture_source ON game_quest_creature_captures(source_item);

CREATE FUNCTION game_quest_creature_capture_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source_row RECORD; capture_row RECORD; input_row RECORD; proof JSONB; ingress_json JSONB;
BEGIN
    SELECT * INTO source_row FROM game_quest_reward_sources
        WHERE (character_id,cause_id,cause_ordinal,transition_key)=
              (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
    IF NOT FOUND THEN
        RAISE EXCEPTION 'creature capture lacks its quest reward source' USING ERRCODE='23514';
    END IF;
    IF source_row.plan->'capture_creature' IS DISTINCT FROM 'true'::JSONB THEN
        IF TG_TABLE_NAME='game_quest_creature_captures' THEN
            RAISE EXCEPTION 'creature capture is not authored by its source' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
    SELECT * INTO capture_row FROM game_quest_creature_captures
        WHERE (character_id,cause_id,cause_ordinal,transition_key)=
              (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key);
    IF NOT FOUND OR capture_row.created_xact_id<>pg_current_xact_id()
        OR source_row.created_xact_id<>pg_current_xact_id()
        OR source_row.plan->>'creature_target' IS DISTINCT FROM capture_row.creature_key THEN
        RAISE EXCEPTION 'quest capture lacks its exact same-transaction target' USING ERRCODE='23514';
    END IF;
    SELECT * INTO input_row FROM game_quest_callback_consumptions c
        WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=
              (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
          AND c.item_instance_id=capture_row.source_item
          AND c.quantity=1 AND c.replacement IS NULL
          AND c.created_xact_id=pg_current_xact_id();
    IF NOT FOUND OR input_row.before_item->>'world_id' IS DISTINCT FROM capture_row.world_id::TEXT
        OR input_row.before_item->>'definition_production_key' IS DISTINCT FROM source_row.plan->>'carried_item' THEN
        RAISE EXCEPTION 'quest capture lacks its exact carried tool' USING ERRCODE='23514';
    END IF;
    proof:=convert_from(input_row.proof,'UTF8')::JSONB;
    SELECT jsonb_agg(get_byte(capture_row.ingress,n) ORDER BY n) INTO ingress_json
        FROM generate_series(0,octet_length(capture_row.ingress)-1) bytes(n);
    IF proof->'reward_callback' IS DISTINCT FROM source_row.plan
        OR proof #> '{carried_use,intent}' IS DISTINCT FROM ingress_json THEN
        RAISE EXCEPTION 'quest capture ingress differs from its carried tool receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_creature_capture_complete
    AFTER INSERT ON game_quest_creature_captures DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_creature_capture_complete();
CREATE CONSTRAINT TRIGGER game_quest_reward_capture_complete
    AFTER INSERT ON game_quest_reward_sources DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_creature_capture_complete();
CREATE TRIGGER game_quest_creature_capture_immutable BEFORE UPDATE OR DELETE ON game_quest_creature_captures
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_creature_capture_no_truncate BEFORE TRUNCATE ON game_quest_creature_captures
    EXECUTE FUNCTION game_item_reject_truncate();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_creature_capture_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_quest_creature_captures FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_creature_captures TO oteryn_game_runtime;
GRANT SELECT ON game_quest_creature_captures TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_quest_creature_capture_complete() FROM PUBLIC;
