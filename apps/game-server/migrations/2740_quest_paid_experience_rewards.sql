-- Compose carried-item repeatable rewards with main's fee-paid MINT owner.
-- A carried reward still needs its original command receipt, every exact BURN,
-- and either the existing MINT proof or the exact pending XP obligation. None of
-- the installed source, payment, item, revision or XP-consumption guards is removed.
CREATE FUNCTION game_quest_carried_repeatable_complete(p game_quest_reward_sources)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE SECURITY DEFINER AS $function$
DECLARE r game_character_quest_receipts%ROWTYPE; part JSONB; slot INTEGER;
        expected_xp_amount BIGINT; consumed BIGINT;
BEGIN
    IF p.created_xact_id<>pg_current_xact_id()
        OR p.plan->'repeatable' IS DISTINCT FROM 'true'::jsonb
        OR jsonb_typeof(p.plan->'take') IS DISTINCT FROM 'array'
        OR jsonb_array_length(p.plan->'take') NOT BETWEEN 1 AND 16
        OR jsonb_typeof(p.plan->'storage') IS DISTINCT FROM 'array'
        OR jsonb_array_length(p.plan->'storage')>8
        OR coalesce(p.plan->'state_ops','[]'::jsonb)<>'[]'::jsonb
        OR coalesce(p.plan->'participants','[]'::jsonb)<>'[]'::jsonb
        OR coalesce(p.plan->'outfit','null'::jsonb)<>'null'::jsonb
        OR coalesce(p.plan->'outfits','[]'::jsonb)<>'[]'::jsonb
        OR coalesce(p.plan->'achievement','null'::jsonb)<>'null'::jsonb
        OR coalesce(p.plan->'achievement_progress','[]'::jsonb)<>'[]'::jsonb
        OR coalesce(p.plan->'completes','false'::jsonb)<>'false'::jsonb
        OR p.plan ? 'fee' OR p.plan ? 'counter'
        OR p.plan ? 'world_claim' OR p.plan ? 'world_callback'
        OR cardinality(p.grant_keys)<>0
        OR jsonb_typeof(p.plan->'npc') IS DISTINCT FROM 'string'
        OR p.plan->>'npc' !~ '^oteryn:npc[.][a-z0-9_]+$'
    THEN RETURN FALSE; END IF;
    IF coalesce(p.plan->'experience','null'::jsonb)<>'null'::jsonb THEN
        IF jsonb_typeof(p.plan->'experience') IS DISTINCT FROM 'number'
            OR p.plan->>'experience' !~ '^[1-9][0-9]*$'
            OR (p.plan->>'experience')::numeric NOT BETWEEN 1 AND 100000000
        THEN RETURN FALSE; END IF;
        expected_xp_amount:=(p.plan->>'experience')::bigint;
    END IF;
    IF p.plan->'give'='null'::jsonb THEN
        IF p.mint_transaction_id IS NOT NULL OR expected_xp_amount IS NULL THEN RETURN FALSE; END IF;
    ELSIF jsonb_typeof(p.plan->'give')='object' THEN
        IF p.mint_transaction_id IS NULL THEN RETURN FALSE; END IF;
    ELSE RETURN FALSE; END IF;

    SELECT * INTO r FROM game_character_quest_receipts
     WHERE character_id=p.character_id AND cause_id=p.cause_id
       AND cause_ordinal=p.cause_ordinal AND transition_key=p.transition_key;
    IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id() OR r.cause_kind<>'command'
        OR r.quest_key IS DISTINCT FROM p.plan->>'quest'
        OR p.transition_key IS DISTINCT FROM p.plan->>'key' OR r.completes
        OR r.committed_character_revision<>r.original_character_revision+1
        OR r.level_before<>r.level_after OR r.experience_before<>r.experience_after
        OR r.values_before IS DISTINCT FROM r.values_after
        OR cardinality(r.track_keys)<>jsonb_array_length(p.plan->'storage')
    THEN RETURN FALSE; END IF;
    -- Preserve-only source predicates retain their original, fenced values.
    FOR part IN SELECT value FROM jsonb_array_elements(p.plan->'storage') LOOP
        slot:=array_position(r.track_keys,part->>'track');
        IF slot IS NULL OR part->'preserve' IS DISTINCT FROM 'true'::jsonb
            OR jsonb_typeof(part->'minimum') IS DISTINCT FROM 'number'
            OR jsonb_typeof(part->'maximum') IS DISTINCT FROM 'number'
            OR r.values_before[slot] NOT BETWEEN (part->>'minimum')::bigint AND (part->>'maximum')::bigint
        THEN RETURN FALSE; END IF;
    END LOOP;
    IF (SELECT count(DISTINCT value->>'track') FROM jsonb_array_elements(p.plan->'storage'))
        <>jsonb_array_length(p.plan->'storage')
        OR (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(p.plan->'take'))
        <>jsonb_array_length(p.plan->'take') THEN RETURN FALSE; END IF;
    FOR part IN SELECT value FROM jsonb_array_elements(p.plan->'take') LOOP
        IF jsonb_typeof(part->'item') IS DISTINCT FROM 'string'
            OR jsonb_typeof(part->'count') IS DISTINCT FROM 'number'
            OR part->>'count' !~ '^[1-9][0-9]*$'
            OR (part->>'count')::numeric NOT BETWEEN 1 AND 65535
        THEN RETURN FALSE; END IF;
        SELECT coalesce(sum(c.quantity),0) INTO consumed FROM game_quest_callback_consumptions c
         WHERE c.character_id=p.character_id AND c.cause_id=p.cause_id
           AND c.cause_ordinal=p.cause_ordinal AND c.transition_key=p.transition_key
           AND c.before_item->>'definition_production_key'=part->>'item'
           AND c.location_kind IN('container','nested','backpack','equipment')
           AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
        IF consumed IS DISTINCT FROM (part->>'count')::bigint THEN RETURN FALSE; END IF;
    END LOOP;
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
        WHERE c.character_id=p.character_id AND c.cause_id=p.cause_id
          AND c.cause_ordinal=p.cause_ordinal AND c.transition_key=p.transition_key
          AND (c.location_kind NOT IN('container','nested','backpack','equipment')
            OR c.replacement IS NOT NULL OR c.created_xact_id<>pg_current_xact_id()
            OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p.plan->'take') AS authored_cost(value)
                WHERE authored_cost.value->>'item'=c.before_item->>'definition_production_key')))
    THEN RETURN FALSE; END IF;
    IF expected_xp_amount IS NULL THEN
        IF EXISTS(SELECT 1 FROM game_character_quest_xp_obligations o
            WHERE o.character_id=p.character_id AND o.cause_id=p.cause_id
              AND o.cause_ordinal=p.cause_ordinal AND o.transition_key=p.transition_key)
        THEN RETURN FALSE; END IF;
    ELSIF NOT EXISTS(SELECT 1 FROM game_character_quest_xp_obligations o
        WHERE o.character_id=p.character_id AND o.cause_id=p.cause_id
          AND o.cause_ordinal=p.cause_ordinal AND o.transition_key=p.transition_key
          AND o.created_xact_id=pg_current_xact_id() AND o.amount=expected_xp_amount
          AND o.pinned_content_revision=r.pinned_content_revision
          AND NOT EXISTS(SELECT 1 FROM game_character_xp_receipts x
              WHERE x.reward_occurrence_id=o.reward_occurrence_id))
    THEN RETURN FALSE; END IF;
    RETURN TRUE;
END $function$;

DO $migration$
DECLARE body TEXT; anchor TEXT; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    anchor:='IF NEW.plan->>''repeatable''=''true'' AND (';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'carried repeatable reward requires the exact main payment guard'; END IF;
    replacement:='IF NEW.plan->>''repeatable''=''true'' AND NOT game_quest_carried_repeatable_complete(NEW) AND (';
    EXECUTE replace(body,anchor,replacement);

    body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    anchor:='    IF cardinality(game_quest_reward_grant_keys(v_source.plan))=0 THEN';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'carried repeatable reward requires the exact value-only source guard'; END IF;
    replacement:=E'    IF game_quest_carried_repeatable_complete(v_source) THEN RETURN NULL; END IF;\n' || anchor;
    EXECUTE replace(body,anchor,replacement);

    -- The pure claim-key derivation mirrors the already bounded carried MINT
    -- shape. The source trigger above independently proves its exact payment.
    body:=pg_get_functiondef('game_quest_reward_mint_claim_key(UUID,UUID,NUMERIC,TEXT,JSONB)'::regprocedure);
    anchor:='AND (v_plan #>> ''{fee,gold}'')::numeric BETWEEN 1 AND 1000000000000';
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'carried repeatable reward requires the exact fee claim-key branch'; END IF;
    replacement:=$proof$AND ((v_plan #>> '{fee,gold}')::numeric BETWEEN 1 AND 1000000000000
            OR (jsonb_array_length(coalesce(v_plan->'take','[]'::jsonb)) BETWEEN 1 AND 16
                AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(v_plan->'take') part
                    WHERE jsonb_typeof(part->'count') IS DISTINCT FROM 'number'
                       OR (part->>'count')::numeric NOT BETWEEN 1 AND 65535)))$proof$;
    EXECUTE replace(body,anchor,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_carried_repeatable_complete(game_quest_reward_sources) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_reward_value_only_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_carried_repeatable_complete(game_quest_reward_sources) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_carried_repeatable_complete(game_quest_reward_sources) TO oteryn_game_runtime;
