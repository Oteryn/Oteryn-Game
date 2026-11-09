-- World item rewards may burn an exact source-authored Ground offering as
-- well as carried inputs. Preserve every installed reward/receipt/MINT arm.
DO $migration$
DECLARE body TEXT;
 offering_anchor TEXT:=$anchor$    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c$anchor$;
 offering_proof TEXT:=$proof$    FOR expected IN SELECT value FROM jsonb_array_elements(
        coalesce(NEW.plan #> '{world_callback,offerings}','[]'::jsonb)) LOOP
        SELECT coalesce(sum(c.quantity),0) INTO actual FROM game_quest_callback_consumptions c
            WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                AND c.location_kind='ground' AND c.replacement IS NULL
                AND c.created_xact_id=pg_current_xact_id()
                AND c.before_item->>'definition_production_key'=expected->>'item'
                AND convert_from(c.proof,'UTF8')::jsonb->'callback'=NEW.plan->'world_callback'
                AND c.before_ground->'spatial_position'=to_jsonb(
                    int4send((expected #>> '{position,x}')::integer)
                    || int4send((expected #>> '{position,y}')::integer)
                    || int2send((expected #>> '{position,floor}')::smallint));
        IF actual IS DISTINCT FROM (expected->>'count')::bigint THEN
            RAISE EXCEPTION 'quest reward must consume exact Ground offering' USING ERRCODE='23514';
        END IF;
    END LOOP;
$proof$;
 original TEXT:=$old$            AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                WHERE e->>'item'=c.before_item->>'definition_production_key')) THEN$old$;
 replacement TEXT:=$new$            AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                WHERE e->>'item'=c.before_item->>'definition_production_key')
            AND (c.location_kind='ground' AND c.replacement IS NULL
                AND c.created_xact_id=pg_current_xact_id()
                AND convert_from(c.proof,'UTF8')::jsonb->'callback'=NEW.plan->'world_callback'
                AND EXISTS(SELECT 1 FROM jsonb_array_elements(
                    coalesce(NEW.plan #> '{world_callback,offerings}','[]'::jsonb)) e
                    WHERE e->>'item'=c.before_item->>'definition_production_key'
                      AND c.before_ground->'spatial_position'=to_jsonb(
                        int4send((e #>> '{position,x}')::integer)
                        || int4send((e #>> '{position,y}')::integer)
                        || int2send((e #>> '{position,floor}')::smallint))
                      AND c.quantity=(e->>'count')::integer)) IS NOT TRUE) THEN$new$;
BEGIN
 body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
 IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
  RAISE EXCEPTION 'unexpected installed quest reward input proof';
 END IF;
 IF (length(body)-length(replace(body,offering_anchor,'')))/length(offering_anchor)<>1 THEN
  RAISE EXCEPTION 'unexpected installed quest reward consumption guard';
 END IF;
 body:=replace(body,offering_anchor,offering_proof || offering_anchor);
 EXECUTE replace(body,original,replacement);
 EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
