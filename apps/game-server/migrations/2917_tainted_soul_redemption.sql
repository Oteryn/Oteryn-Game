-- Redemption retains the selected talisman. Other live capture plans still require
-- their original same-transaction consumption and ingress proof.
CREATE OR REPLACE FUNCTION game_quest_creature_capture_complete() RETURNS trigger LANGUAGE plpgsql AS $$
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
    IF NEW.transition_key='oteryn:quest-reward/tainted-soul-redemption' THEN
        IF source_row.plan->>'carried_item' IS DISTINCT FROM 'oteryn:item.tibia.i22009'
            OR capture_row.creature_key IS DISTINCT FROM 'oteryn:creature.tainted_soul'
            OR NOT EXISTS (
                SELECT 1
                FROM game_quest_callback_carried(capture_row.character_id,capture_row.world_id,capture_row.source_item) c
                JOIN game_item_instances i ON i.item_instance_id=c.item
                WHERE i.item_instance_id=capture_row.source_item AND i.lifecycle=1
                    AND i.definition_family='Item' AND i.world_id=capture_row.world_id
                    AND i.definition_production_key='oteryn:item.tibia.i22009'
            ) THEN
            RAISE EXCEPTION 'redemption lacks its exact carried talisman' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
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
DO $migration$
DECLARE
    definition TEXT;
    anchor TEXT := $anchor$    IF jsonb_typeof(NEW.plan->'take') IS DISTINCT FROM 'array'$anchor$;
BEGIN
    definition := pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    IF (length(definition) - length(replace(definition, anchor, ''))) <> length(anchor) THEN
        RAISE EXCEPTION 'unexpected carried tool receipt proof';
    END IF;
    EXECUTE replace(definition, anchor, $arm$    IF NEW.transition_key='oteryn:quest-reward/tainted-soul-redemption' THEN
        IF NEW.created_xact_id<>pg_current_xact_id()
            OR NEW.plan->'capture_creature' IS DISTINCT FROM 'true'::jsonb
            OR NEW.plan->'take' IS DISTINCT FROM '[]'::jsonb
            OR NEW.plan->'give' IS DISTINCT FROM 'null'::jsonb THEN
            RAISE EXCEPTION 'redemption must retain its carried tool' USING ERRCODE='23514';
        END IF;
        -- The deferred capture guard verifies the exact selected talisman and receipt.
        RETURN NULL;
    END IF;
$arm$ || anchor);
    EXECUTE format('ALTER FUNCTION game_quest_creature_capture_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END
$migration$;
REVOKE ALL ON FUNCTION game_quest_creature_capture_complete() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_creature_capture_complete() TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_quest_creature_capture_complete() TO oteryn_game_control;
