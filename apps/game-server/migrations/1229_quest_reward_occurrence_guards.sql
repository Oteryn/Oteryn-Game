-- Movement ingress uses the same exact native MINT participant as USE. Bind
-- each source receipt kind to its callback trigger; existing proof arms stay intact.
DO $migration$
DECLARE body TEXT; original TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    original:=$old$r.cause_kind='use'$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward cause proof';
    END IF;
    body:=replace(body,original,$new$r.cause_kind IN ('use','step')
            AND (r.cause_kind='step')=(NEW.plan #>> '{world_callback,trigger}'='step_in')$new$);
    original:=$old$AND NEW.plan #>> '{world_callback,trigger}' IN ('use','use_carried_on_target')$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed world reward trigger proof';
    END IF;
    body:=replace(body,original,$new$AND NEW.plan #>> '{world_callback,trigger}' IN ('use','step_in','use_carried_on_target')$new$);
    original:=$old$NEW.plan #>> '{world_callback,trigger}'='use'$old$;
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected installed plain world reward proof';
    END IF;
    EXECUTE replace(body,original,$new$NEW.plan #>> '{world_callback,trigger}' IN ('use','step_in')$new$);
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;

-- Exact negative custody evidence is rechecked by the fenced Rust writer.
-- This deferred participant independently proves the postcondition: no other
-- owned instance of any absent definition, and a refill closes its own gate.
CREATE FUNCTION game_quest_owned_absence_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE minted UUID; world UUID;
BEGIN
    IF NOT (NEW.plan ? 'absent') THEN RETURN NULL; END IF;
    IF jsonb_typeof(NEW.plan->'absent') IS DISTINCT FROM 'array'
        OR jsonb_array_length(NEW.plan->'absent') NOT BETWEEN 1 AND 8
        OR (SELECT count(DISTINCT value) FROM jsonb_array_elements_text(NEW.plan->'absent'))
            <>jsonb_array_length(NEW.plan->'absent')
        OR EXISTS(SELECT 1 FROM jsonb_array_elements_text(NEW.plan->'absent') a(item)
            WHERE a.item !~ '^oteryn:item[.][a-z0-9._/:-]+$' OR octet_length(a.item)>256)
        OR NEW.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'quest owned-absence predicate is malformed' USING ERRCODE='23514';
    END IF;
    IF NEW.mint_transaction_id IS NOT NULL THEN
        SELECT m.item_instance_id,i.world_id INTO STRICT minted,world
            FROM game_reward_claim_mint_receipts m JOIN game_item_instances i USING(item_instance_id)
            WHERE m.transaction_id=NEW.mint_transaction_id AND m.character_id=NEW.character_id
                AND m.created_xact_id=pg_current_xact_id();
    ELSE
        RAISE EXCEPTION 'quest owned-absence participant requires an Item MINT' USING ERRCODE='23514';
    END IF;
    IF world IS NULL OR EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,world) c
        JOIN game_item_instances i ON i.item_instance_id=c.item
        WHERE i.lifecycle=1 AND i.definition_family='Item'
            AND NEW.plan->'absent' ? i.definition_production_key
            AND i.item_instance_id IS DISTINCT FROM minted) THEN
        RAISE EXCEPTION 'quest owned-absence predicate has another owned instance' USING ERRCODE='23514';
    END IF;
    IF NEW.plan->'absent' ? (NEW.plan #>> '{give,item}') AND
        (minted IS NULL OR NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,world) c
            WHERE c.item=minted)) THEN
        RAISE EXCEPTION 'quest owned-absence refill must close its custody gate' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_owned_absence_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_owned_absence_complete();
DO $migration$
DECLARE body TEXT; at_begin INTEGER;
BEGIN
    body:=pg_get_functiondef('game_quest_earned_repeat_complete()'::regprocedure);
    at_begin:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF at_begin=0 THEN RAISE EXCEPTION 'unexpected earned-repeat guard body'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n'||$arm$
    IF jsonb_typeof(NEW.plan->'repeatable')='boolean' AND NEW.plan->>'repeatable'='true'
        AND jsonb_typeof(NEW.plan->'absent')='array' AND jsonb_array_length(NEW.plan->'absent')=1
        AND NEW.plan #>> '{absent,0}'=NEW.plan #>> '{give,item}'
        AND jsonb_array_length(NEW.plan->'take')=0
        AND jsonb_typeof(NEW.plan->'give')='object' AND NEW.mint_transaction_id IS NOT NULL
        AND coalesce(NEW.plan #> '{give,contents}','[]'::jsonb)='[]'::jsonb
        AND coalesce(NEW.plan #> '{give,backpack_outputs}','[]'::jsonb)='[]'::jsonb
        AND coalesce(NEW.plan->'fee','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'outfit','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'achievement','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'blessing','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'experience','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'health_loss','null'::jsonb)='null'::jsonb
        AND coalesce(NEW.plan->'participants','[]'::jsonb)='[]'::jsonb
        AND coalesce(NEW.plan->'state_ops','[]'::jsonb)='[]'::jsonb
        AND coalesce(NEW.plan->'outfits','[]'::jsonb)='[]'::jsonb
        AND coalesce(NEW.plan->'achievement_progress','[]'::jsonb)='[]'::jsonb
        AND NOT coalesce((NEW.plan->>'completes')::boolean,false)
        AND NEW.created_xact_id=pg_current_xact_id() THEN
        -- The existing complete source/MINT guard and the new owned-absence
        -- participant still execute; a successful MINT ends eligibility.
        RETURN NULL;
    END IF;
$arm$ from at_begin for 5);
    EXECUTE format('ALTER FUNCTION game_quest_owned_absence_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_earned_repeat_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_owned_absence_complete() FROM PUBLIC;
