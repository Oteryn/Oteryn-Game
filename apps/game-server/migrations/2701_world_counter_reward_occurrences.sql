-- Bounded world counters mint once per USE command, preserving the existing
-- paid, timed, NPC-counter and fixed-claim identities and their deferred proofs.
DO $migration$
DECLARE body TEXT; anchor TEXT := $anchor$SELECT CASE WHEN v_plan ? 'counter' THEN$anchor$;
BEGIN
    SELECT pg_get_functiondef('game_quest_reward_mint_claim_key(uuid,uuid,numeric,text,jsonb)'::regprocedure)
        INTO STRICT body;
    IF position('world-counter-use-v1' IN body)=0 THEN
        IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
            RAISE EXCEPTION 'world counter reward requires the exact adopted claim helper';
        END IF;
        EXECUTE replace(body,anchor,$replacement$SELECT CASE WHEN
            -- world-counter-use-v1: the complete original callback remains in the proof.
            v_plan #>> '{world_callback,trigger}'='use'
            AND v_plan #>> '{world_callback,key}'=v_transition
            AND v_plan #> '{world_callback,storage}'=v_plan->'storage'
            AND EXISTS(SELECT 1 FROM jsonb_array_elements(v_plan->'storage') s WHERE s ? 'add')
        THEN 'oteryn:quest-exchange/'||encode(sha256(
            convert_to('oteryn:quest-exchange:v1','UTF8')||decode('00','hex')
            ||uuid_send(v_character)||uuid_send(v_cause)
            ||convert_to(v_ordinal::text||':'||v_transition,'UTF8')),'hex')
        WHEN v_plan ? 'counter' THEN$replacement$);
    END IF;
END $migration$;

-- A per-command claim must retain an exact, bounded same-transaction USE increment.
-- This adds a proof; it does not replace any existing reward/source/receipt guard.
CREATE OR REPLACE FUNCTION game_quest_world_counter_reward_complete()
RETURNS trigger LANGUAGE plpgsql AS $function$
DECLARE r game_character_quest_receipts%ROWTYPE; s JSONB; slot INTEGER; delta NUMERIC;
BEGIN
    IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(
        coalesce(NEW.plan #> '{world_callback,storage}','[]'::jsonb)) effect
        WHERE effect ? 'add') THEN RETURN NULL; END IF;
    IF NEW.plan #>> '{world_callback,trigger}' IS DISTINCT FROM 'use'
        OR NEW.plan #>> '{world_callback,key}' IS DISTINCT FROM NEW.transition_key
        OR NEW.plan #>> '{world_callback,quest}' IS DISTINCT FROM NEW.plan->>'quest'
        OR NEW.plan #> '{world_callback,storage}' IS DISTINCT FROM NEW.plan->'storage'
        OR NEW.mint_transaction_id IS NULL THEN
        RAISE EXCEPTION 'world counter reward source mismatch' USING ERRCODE='23514';
    END IF;
    SELECT * INTO r FROM game_character_quest_receipts
        WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id
            AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR r.cause_kind<>'use' OR r.quest_key IS DISTINCT FROM NEW.plan->>'quest'
        OR r.created_xact_id<>pg_current_xact_id() OR NEW.created_xact_id<>pg_current_xact_id()
        OR r.completes THEN
        RAISE EXCEPTION 'world counter reward lacks exact USE receipt' USING ERRCODE='23514';
    END IF;
    FOR s IN SELECT value FROM jsonb_array_elements(NEW.plan->'storage') WHERE value ? 'add' LOOP
        slot:=array_position(r.track_keys,s->>'track');
        IF slot IS NULL
            OR EXISTS(SELECT 1 FROM jsonb_object_keys(s) key
                WHERE key NOT IN ('track','minimum','maximum','set','add','preserve','read_only'))
            OR jsonb_typeof(s->'minimum') IS DISTINCT FROM 'number'
            OR jsonb_typeof(s->'maximum') IS DISTINCT FROM 'number'
            OR jsonb_typeof(s->'set') IS DISTINCT FROM 'number'
            OR (s->>'minimum') !~ '^-?(0|[1-9][0-9]*)$'
            OR (s->>'maximum') !~ '^-?(0|[1-9][0-9]*)$'
            OR (s->>'set') !~ '^-?(0|[1-9][0-9]*)$'
            OR (s ? 'preserve' AND jsonb_typeof(s->'preserve') IS DISTINCT FROM 'boolean')
            OR (s ? 'read_only' AND jsonb_typeof(s->'read_only') IS DISTINCT FROM 'boolean')
            OR jsonb_typeof(s->'add') IS DISTINCT FROM 'number'
            OR (s->>'add') !~ '^[1-9][0-9]*$'
            OR coalesce((s->>'preserve')::boolean,false)
            OR coalesce((s->>'read_only')::boolean,false) THEN
            RAISE EXCEPTION 'world counter reward increment shape' USING ERRCODE='23514';
        END IF;
        delta:=(s->>'add')::numeric;
        IF delta>9223372036854775807::numeric
            OR r.values_before[slot] NOT BETWEEN (s->>'minimum')::bigint AND (s->>'maximum')::bigint
            OR r.values_after[slot]::numeric IS DISTINCT FROM r.values_before[slot]::numeric+delta
            OR r.values_after[slot]>(s->>'set')::bigint THEN
            RAISE EXCEPTION 'world counter reward increment mismatch' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $function$;
DROP TRIGGER IF EXISTS game_quest_world_counter_reward_complete ON game_quest_reward_sources;
CREATE CONSTRAINT TRIGGER game_quest_world_counter_reward_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_world_counter_reward_complete();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_world_counter_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_world_counter_reward_complete() FROM PUBLIC;
-- Trigger functions are invoked by PostgreSQL; no direct role execution is needed.
