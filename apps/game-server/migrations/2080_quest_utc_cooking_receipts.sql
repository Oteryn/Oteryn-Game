-- M42: the existing NPC receipt/input/MINT owners execute the admitted UTC planner.
-- Plain rewards retain their original once identity. Permitted cooking occurrences
-- are identified by their exact monotone source cook counter, never wall-clock text.
CREATE FUNCTION game_quest_npc_reward_claim_key(p_plan JSONB, p_transition TEXT)
RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE n INTEGER; counter JSONB; input TEXT := p_transition;
BEGIN
    IF p_plan->>'quest'='oteryn:quest.hot_cuisine_quest'
       AND p_transition LIKE 'oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-%' THEN
        IF p_transition !~ '^oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-([1-9]|1[0-5])$' THEN
            RAISE EXCEPTION 'NPC cooking rule identity' USING ERRCODE='23514';
        END IF;
        n := substring(p_transition FROM 'native-cook-([0-9]+)$')::INTEGER;
        SELECT value INTO STRICT counter FROM jsonb_array_elements(p_plan->'tracks')
          WHERE value->>'key'='oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-'||(n+1)::TEXT;
        IF (counter->>'before')::NUMERIC < 0
           OR (counter->>'after')::NUMERIC<>(counter->>'before')::NUMERIC+1 THEN
            RAISE EXCEPTION 'NPC cooking occurrence must advance once' USING ERRCODE='23514';
        END IF;
        input := p_transition||'/'||(counter->>'after');
    END IF;
    RETURN 'oteryn:reward-claim.npc-program.'||encode(sha256(convert_to(input,'UTF8')),'hex');
END $$;

CREATE OR REPLACE FUNCTION game_quest_npc_item_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; q game_character_quest_receipts%ROWTYPE;
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR NEW.plan #>> '{rules,0}' IS DISTINCT FROM NEW.transition_key
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
                AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
                AND r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'
                AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest reward lacks same-transaction quest receipt' USING ERRCODE='23514';
    END IF;
    SELECT * INTO STRICT q FROM game_character_quest_receipts WHERE character_id=NEW.character_id
      AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF q.track_keys IS DISTINCT FROM ARRAY(SELECT e->>'key' FROM jsonb_array_elements(NEW.plan->'tracks') e)
      OR q.values_before IS DISTINCT FROM ARRAY(SELECT (e->>'before')::bigint FROM jsonb_array_elements(NEW.plan->'tracks') e)
      OR q.values_after IS DISTINCT FROM ARRAY(SELECT (e->>'after')::bigint FROM jsonb_array_elements(NEW.plan->'tracks') e)
      OR jsonb_typeof(NEW.plan->'program_digest') IS DISTINCT FROM 'array'
      OR jsonb_array_length(NEW.plan->'program_digest')<>32
      OR NOT EXISTS(SELECT 1 FROM game_quest_npc_program_receipts n WHERE n.character_id=NEW.character_id
        AND n.game_session_id=NEW.cause_id AND n.command_id=NEW.cause_ordinal AND n.transition_key=NEW.transition_key)
    THEN RAISE EXCEPTION 'NPC items require their exact programme tracks and NPC response' USING ERRCODE='23514'; END IF;
    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP
        SELECT coalesce(sum(c.quantity),0) INTO actual FROM game_quest_callback_consumptions c
            WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                AND c.before_item->>'definition_production_key'=expected->>'item'
                AND c.location_kind IN('container','nested','backpack','equipment')
                AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
        IF actual IS DISTINCT FROM (expected->>'count')::bigint THEN
            RAISE EXCEPTION 'quest reward must consume exact carried item count' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
        WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                WHERE e->>'item'=c.before_item->>'definition_production_key')) THEN
        RAISE EXCEPTION 'quest reward consumed an unauthored input' USING ERRCODE='23514';
    END IF;
    IF (NEW.plan->'give'='null'::jsonb) IS DISTINCT FROM (NEW.mint_transaction_id IS NULL)
        OR (NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS(
            SELECT 1 FROM game_reward_claim_mint_receipts m JOIN game_item_instances i
                ON i.item_instance_id=m.item_instance_id
            WHERE m.transaction_id=NEW.mint_transaction_id AND m.character_id=NEW.character_id
                AND m.game_session_id=NEW.cause_id AND m.command_id=NEW.cause_ordinal
                AND m.claim_family='RewardClaim'
                AND m.claim_production_key=game_quest_npc_reward_claim_key(NEW.plan,NEW.transition_key) AND m.created_xact_id=pg_current_xact_id()
                AND i.definition_production_key=NEW.plan #>> '{give,item}'
                AND m.quantity=(NEW.plan #>> '{give,count}')::bigint
                AND i.quantity=m.quantity AND i.lifecycle=1 AND i.minted_transaction_id=m.transaction_id)) THEN
        RAISE EXCEPTION 'quest reward lacks exact native backpack MINT' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

-- Preserve the original aggregate input/MINT checks above and add exact subtype
-- checks. A consumed filled container retains its immutable fluid facts on the
-- dead Item; only live carried instances can satisfy the next cooking operation.
CREATE FUNCTION game_quest_npc_typed_item_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; take JSONB;
BEGIN
    IF NOT (NEW.plan ? 'typed_take') AND NOT (NEW.plan->>'quest'='oteryn:quest.hot_cuisine_quest'
        AND NEW.transition_key LIKE 'oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-%') THEN
        RETURN NULL; -- existing independently proven plain NPC producers retain their contract
    END IF;
    IF jsonb_typeof(NEW.plan->'typed_take') IS DISTINCT FROM 'array'
       OR jsonb_array_length(NEW.plan->'typed_take')>16 THEN
        RAISE EXCEPTION 'NPC typed inputs need their exact source list' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'typed_take') e
              WHERE jsonb_typeof(e->'item') IS DISTINCT FROM 'string'
                 OR jsonb_typeof(e->'count') IS DISTINCT FROM 'number'
                 OR e->>'count' !~ '^[0-9]+$'
                 OR (e->>'count')::NUMERIC NOT BETWEEN 1 AND 100
                 OR (e->>'subtype' IS NOT NULL AND (e->>'subtype')::NUMERIC NOT BETWEEN 1 AND 255))
       OR (SELECT count(*) FROM jsonb_array_elements(NEW.plan->'typed_take'))
          <> (SELECT count(DISTINCT (e->>'item',coalesce(e->>'subtype','0')))
              FROM jsonb_array_elements(NEW.plan->'typed_take') e) THEN
        RAISE EXCEPTION 'NPC typed input shape or duplicate' USING ERRCODE='23514';
    END IF;
    FOR take IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP
        SELECT coalesce(sum((e->>'count')::BIGINT),0) INTO actual
          FROM jsonb_array_elements(NEW.plan->'typed_take') e WHERE e->>'item'=take->>'item';
        IF actual<>(take->>'count')::BIGINT THEN
            RAISE EXCEPTION 'NPC typed/aggregate source mismatch' USING ERRCODE='23514';
        END IF;
    END LOOP;
    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'typed_take') LOOP
        IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                      WHERE e->>'item'=expected->>'item') THEN
            RAISE EXCEPTION 'NPC typed input outside source list' USING ERRCODE='23514';
        END IF;
        SELECT coalesce(sum(c.quantity),0) INTO actual
          FROM game_quest_callback_consumptions c
          LEFT JOIN game_item_fluid_states f ON f.item_instance_id=c.item_instance_id
          WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND c.before_item->>'definition_production_key'=expected->>'item'
            AND coalesce(f.fluid_subtype,0)=coalesce((expected->>'subtype')::SMALLINT,0)
            AND c.created_xact_id=pg_current_xact_id();
        IF actual<>(expected->>'count')::BIGINT THEN
            RAISE EXCEPTION 'NPC exact fluid subtype consumption missing' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_npc_typed_item_complete
 AFTER INSERT ON game_quest_npc_item_sources DEFERRABLE INITIALLY DEFERRED
 FOR EACH ROW EXECUTE FUNCTION game_quest_npc_typed_item_complete();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_npc_item_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_npc_typed_item_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT),game_quest_npc_typed_item_complete() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_npc_reward_claim_key(JSONB,TEXT) TO oteryn_game_runtime;
