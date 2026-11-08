-- Carried live-creature sampling may mint the transformed tool through the existing MINT owner.
CREATE FUNCTION game_quest_carried_output_valid(plan JSONB) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT COALESCE(plan->'give' = 'null'::jsonb OR (
   COALESCE(plan->>'creature_target','') ~ '^oteryn:creature\.[a-z0-9_]+$'
   AND jsonb_typeof(plan->'give') = 'object'
   AND plan #>> '{give,count}' = '1'
   AND COALESCE(plan #>> '{give,item}','') ~ '^oteryn:item\.tibia\.i[1-9][0-9]*$'
   AND COALESCE(jsonb_array_length(plan #> '{give,contents}'),0) = 0
   AND COALESCE(jsonb_array_length(plan #> '{give,additional}'),0) = 0
 ) IS TRUE, false)
$$;
REVOKE ALL ON FUNCTION game_quest_carried_output_valid(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_carried_output_valid(JSONB) TO oteryn_game_runtime;
-- Carried quest tools reuse existing same-transaction quest/input/grant proofs.
-- Bind the exact selected instance and bounded ingress before acknowledging USE.
CREATE OR REPLACE FUNCTION game_quest_carried_use_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_input RECORD; v_proof JSONB; v_source JSONB;
BEGIN
    IF NOT (NEW.plan ? 'carried_item') THEN
        RETURN NULL;
    END IF;
    IF jsonb_array_length(NEW.plan->'take')<>1
        OR NEW.plan #>> '{take,0,item}' IS DISTINCT FROM NEW.plan->>'carried_item'
        OR NEW.plan #>> '{take,0,count}' IS DISTINCT FROM '1'
        OR NOT game_quest_carried_output_valid(NEW.plan)
        OR NEW.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'carried quest tool lacks exact selected input' USING ERRCODE='23514';
    END IF;
    SELECT c.* INTO v_input FROM game_quest_callback_consumptions c
        WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND c.quantity=1 AND c.replacement IS NULL
            AND c.before_item->>'definition_production_key'=NEW.plan->>'carried_item'
            AND c.created_xact_id=pg_current_xact_id();
    IF NOT FOUND THEN
        RAISE EXCEPTION 'carried quest tool lacks selected-instance proof' USING ERRCODE='23514';
    END IF;
    v_proof:=convert_from(v_input.proof,'UTF8')::jsonb;
    SELECT jsonb_agg(get_byte(uuid_send(v_input.item_instance_id),n) ORDER BY n)
        INTO v_source FROM generate_series(0,15) AS bytes(n);
    IF v_proof->'reward_callback' IS DISTINCT FROM NEW.plan
        OR v_proof #> '{carried_use,source}' IS DISTINCT FROM v_source
        OR jsonb_typeof(v_proof #> '{carried_use,intent}') IS DISTINCT FROM 'array' THEN
        RAISE EXCEPTION 'carried quest tool ingress differs from selected instance' USING ERRCODE='23514';
    END IF;
    IF jsonb_array_length(v_proof #> '{carried_use,intent}') NOT BETWEEN 1 AND 4096
        OR EXISTS(SELECT 1 FROM jsonb_array_elements(v_proof #> '{carried_use,intent}') value
            WHERE CASE WHEN value::text ~ '^[0-9]{1,3}$' THEN value::text::integer NOT BETWEEN 0 AND 255 ELSE true END) THEN
        RAISE EXCEPTION 'carried quest tool ingress is malformed' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_carried_use_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_carried_use_complete() FROM PUBLIC;
