-- Bounded sex-qualified target sets retain legacy neutral grants and exact same-R proofs.
CREATE FUNCTION game_quest_reward_grant_keys(v_plan JSONB) RETURNS TEXT[] LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE v_targets JSONB:=coalesce(v_plan->'outfits','[]'::jsonb); v_target JSONB; v_addon INTEGER;
    v_keys TEXT[]:='{}'; v_sex TEXT; v_mask INTEGER; v_key TEXT;
BEGIN
    IF jsonb_typeof(v_targets) IS DISTINCT FROM 'array' OR jsonb_array_length(v_targets)>4 THEN
        RAISE EXCEPTION 'invalid bounded quest outfit targets' USING ERRCODE='23514'; END IF;
    IF v_plan->'outfit'<>'null'::jsonb THEN
        IF jsonb_array_length(v_targets)<>0 THEN RAISE EXCEPTION 'mixed legacy and target-set grants' USING ERRCODE='23514'; END IF;
        v_targets:=jsonb_build_array(v_plan->'outfit');
    END IF;
    FOR v_target IN SELECT value FROM jsonb_array_elements(v_targets) LOOP
        v_sex:=v_target->>'sex'; v_mask:=(v_target->>'addons')::integer;
        IF jsonb_typeof(v_target) IS DISTINCT FROM 'object' OR v_mask NOT BETWEEN 0 AND 3
            OR v_mask IS NULL OR (v_target->>'outfit') IS NULL
            OR (v_target->>'outfit')!~'^oteryn:outfit[.][a-z0-9_]+$'
            OR (v_sex IS NOT NULL AND v_sex NOT IN('female','male')) THEN
            RAISE EXCEPTION 'invalid sex-qualified quest outfit target' USING ERRCODE='23514'; END IF;
        FOR v_addon IN 0..2 LOOP
            IF v_addon<>0 AND (v_mask & (1 << (v_addon-1)))=0 THEN CONTINUE; END IF;
            v_key:=(v_target->>'outfit')||'/quest-grant/'||coalesce(v_sex||'/','')||v_addon;
            IF NOT v_key=ANY(v_keys) THEN v_keys:=array_append(v_keys,v_key); END IF;
        END LOOP;
    END LOOP;
    RETURN v_keys;
END $$;
ALTER TABLE game_quest_reward_sources DROP CONSTRAINT game_quest_reward_sources_grant_keys_check;
ALTER TABLE game_quest_reward_sources ADD CONSTRAINT game_quest_reward_sources_grant_keys_check
    CHECK(cardinality(grant_keys)<=12);

CREATE OR REPLACE FUNCTION game_quest_reward_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; account UUID; grant_key TEXT; expected_keys TEXT[];
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
                AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
                AND r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'
                AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest reward lacks same-transaction quest receipt' USING ERRCODE='23514';
    END IF;
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
                AND m.claim_production_key=NEW.transition_key AND m.created_xact_id=pg_current_xact_id()
                AND i.definition_production_key=NEW.plan #>> '{give,item}'
                AND m.quantity=(NEW.plan #>> '{give,count}')::bigint)) THEN
        RAISE EXCEPTION 'quest reward lacks exact native backpack MINT' USING ERRCODE='23514';
    END IF;
    SELECT account_id INTO STRICT account FROM game_character_roots WHERE character_id=NEW.character_id;
    -- Account unlocks can already exist from another Character. They still
    -- satisfy the reward, but must never be replaced with a second receipt.
    expected_keys:=game_quest_reward_grant_keys(NEW.plan);
    IF jsonb_array_length(NEW.plan->'take')>16
        OR cardinality(NEW.grant_keys)<>(SELECT count(DISTINCT k) FROM unnest(NEW.grant_keys) k) THEN
        RAISE EXCEPTION 'quest reward exceeds bounded source recipe or repeats a grant' USING ERRCODE='23514';
    END IF;
    FOREACH grant_key IN ARRAY expected_keys LOOP
        IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
            JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
            WHERE u.account_id=account AND u.unlock_key=grant_key AND r.source_kind='quest'
                AND r.content_revision='crystal-9f5a72c-quest-rewards-v1') THEN
            RAISE EXCEPTION 'quest outfit lacks compatible earned fact' USING ERRCODE='23514';
        END IF;
    END LOOP;
    FOREACH grant_key IN ARRAY NEW.grant_keys LOOP
        IF NOT grant_key=ANY(expected_keys) THEN
            RAISE EXCEPTION 'quest outfit has an unauthored grant key' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
            JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
            WHERE u.account_id=account AND u.unlock_key=grant_key
                AND r.granting_character_id=NEW.character_id AND r.source_kind='quest'
                AND r.content_revision='crystal-9f5a72c-quest-rewards-v1'
                AND r.xmin=pg_current_xact_id()::text::xid) THEN
            RAISE EXCEPTION 'quest outfit lacks same-transaction earned fact' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;

-- A source without quest-state deltas must still prove an admitted, one-shot value action.
-- Preserve the historical nonempty and completion-only arms exactly.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
    cardinality(track_keys) BETWEEN 0 AND 8
    AND (cardinality(track_keys)>0 OR completes OR transition_key LIKE 'oteryn:quest-reward/%')
    AND (cardinality(track_keys)=0 OR (array_ndims(track_keys)=1 AND array_lower(track_keys,1)=1
        AND array_ndims(values_before)=1 AND array_lower(values_before,1)=1
        AND array_ndims(values_after)=1 AND array_lower(values_after,1)=1))
    AND cardinality(values_before)=cardinality(track_keys) AND cardinality(values_after)=cardinality(track_keys)
    AND array_position(track_keys,NULL) IS NULL AND array_position(values_before,NULL) IS NULL
    AND array_position(values_after,NULL) IS NULL);
CREATE FUNCTION game_quest_reward_value_only_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_source game_quest_reward_sources%ROWTYPE; v_targets JSONB;
BEGIN
    IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;
    SELECT * INTO v_source FROM game_quest_reward_sources WHERE character_id=NEW.character_id
        AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR NEW.cause_kind<>'command' OR NEW.created_xact_id<>pg_current_xact_id()
        OR v_source.created_xact_id<>pg_current_xact_id() OR v_source.plan->>'quest' IS DISTINCT FROM NEW.quest_key
        OR jsonb_array_length(v_source.plan->'storage')<>0
        OR jsonb_array_length(coalesce(v_source.plan->'state_ops','[]'::jsonb))<>0
        OR cardinality(game_quest_reward_grant_keys(v_source.plan))=0 THEN
        RAISE EXCEPTION 'value-only quest lacks exact same-transaction source' USING ERRCODE='23514'; END IF;
    v_targets:=CASE WHEN v_source.plan->'outfit'<>'null'::jsonb THEN jsonb_build_array(v_source.plan->'outfit')
        ELSE coalesce(v_source.plan->'outfits','[]'::jsonb) END;
    IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(v_targets) t
        CROSS JOIN jsonb_array_elements(coalesce(v_source.plan->'forbids_addons','[]'::jsonb)) g
        WHERE t->>'outfit'=g->>'outfit' AND ((t->>'addons')::integer & (g->>'addons')::integer)<>0
            AND (t->>'sex' IS NULL OR t->>'sex'=coalesce(g->>'sex',v_source.plan->>'creation_sex'))) THEN
        RAISE EXCEPTION 'value-only quest lacks one-shot earned predicate' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_value_only_complete AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_value_only_complete();
DO $migration$
DECLARE v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY['game_quest_reward_grant_keys(jsonb)',
        'game_quest_reward_complete()','game_quest_reward_value_only_complete()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',v_function,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',v_function);
    END LOOP;
END $migration$;
GRANT EXECUTE ON FUNCTION game_quest_reward_grant_keys(JSONB) TO oteryn_game_runtime;
